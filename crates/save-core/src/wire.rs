use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

/// Resource limits apply before allocation. The parser is iterative, not recursive.
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_records: usize,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 128 * 1024 * 1024,
            max_records: 1_000_000,
            max_depth: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WireString {
    pub wide: bool,
    pub bytes: Vec<u8>,
}
impl WireString {
    pub fn display(&self) -> String {
        if self.wide {
            String::from_utf16_lossy(
                &self
                    .bytes
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>(),
            )
        } else {
            self.bytes.iter().map(|b| char::from(*b)).collect()
        }
    }
    pub fn replace(&self, value: &str) -> Self {
        let units: Vec<_> = value.encode_utf16().collect();
        let wide = self.wide || units.iter().any(|u| *u > 255);
        let bytes = if wide {
            units.iter().flat_map(|u| u.to_le_bytes()).collect()
        } else {
            units.iter().map(|u| *u as u8).collect()
        };
        Self { wide, bytes }
    }
    fn write(&self, out: &mut Vec<u8>) {
        out.push(u8::from(self.wide));
        out.extend_from_slice(
            &((self.bytes.len() / if self.wide { 2 } else { 1 }) as i32).to_le_bytes(),
        );
        out.extend_from_slice(&self.bytes);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TypeInfo {
    None,
    Definition(i32, WireString),
    Reference(i32),
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Payload {
    Empty,
    Node {
        ty: TypeInfo,
        object: Option<i32>,
    },
    Array(i64),
    Packed {
        count: i32,
        width: i32,
        bytes: Vec<u8>,
    },
    Fixed(Vec<u8>),
    Text(WireString),
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Record {
    pub tag: u8,
    pub name: Option<WireString>,
    pub payload: Payload,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub offset: usize,
}
impl Record {
    pub fn write(&self, out: &mut Vec<u8>) {
        out.push(self.tag);
        if let Some(name) = &self.name {
            name.write(out);
        }
        match &self.payload {
            Payload::Empty => {}
            Payload::Node { ty, object } => {
                match ty {
                    TypeInfo::None => out.push(46),
                    TypeInfo::Reference(id) => {
                        out.push(48);
                        out.extend_from_slice(&id.to_le_bytes());
                    }
                    TypeInfo::Definition(id, name) => {
                        out.push(47);
                        out.extend_from_slice(&id.to_le_bytes());
                        name.write(out);
                    }
                }
                if let Some(id) = object {
                    out.extend_from_slice(&id.to_le_bytes());
                }
            }
            Payload::Array(count) => out.extend_from_slice(&count.to_le_bytes()),
            Payload::Packed {
                count,
                width,
                bytes,
            } => {
                out.extend_from_slice(&count.to_le_bytes());
                out.extend_from_slice(&width.to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Payload::Fixed(bytes) => out.extend_from_slice(bytes),
            Payload::Text(value) => value.write(out),
        }
    }
}

/// Captures the first version of each modified record during a transaction.
/// Mutable access deliberately goes through IndexMut; there is no DerefMut escape.
#[derive(Clone, Default)]
pub(crate) struct Records {
    values: Vec<Record>,
    journal: Option<(usize, HashMap<usize, Record>)>,
}
impl std::ops::Deref for Records {
    type Target = [Record];
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl std::ops::Index<usize> for Records {
    type Output = Record;
    fn index(&self, id: usize) -> &Record {
        &self.values[id]
    }
}
impl std::ops::IndexMut<usize> for Records {
    fn index_mut(&mut self, id: usize) -> &mut Record {
        if let Some((start, before)) = &mut self.journal {
            if id < *start {
                before.entry(id).or_insert_with(|| self.values[id].clone());
            }
        }
        &mut self.values[id]
    }
}
impl Records {
    pub(crate) fn push(&mut self, record: Record) {
        self.values.push(record);
    }
    pub(crate) fn truncate(&mut self, len: usize) {
        self.values.truncate(len);
    }
    pub(crate) fn begin(&mut self) {
        assert!(self.journal.is_none());
        self.journal = Some((self.len(), HashMap::new()));
    }
    pub(crate) fn touched(&self) -> Vec<usize> {
        self.journal.as_ref().unwrap().1.keys().copied().collect()
    }
    pub(crate) fn finish(&mut self) -> HashMap<usize, Record> {
        self.journal.take().unwrap().1
    }
}

/// Owns parsed records, not the original file. Encoding reconstructs every header and payload.
#[derive(Clone)]
pub struct Document {
    pub(crate) next_object_id: std::cell::Cell<Option<i32>>,
    pub(crate) records: Records,
    pub(crate) roots: Vec<usize>,
    pub(crate) types: HashMap<i32, String>,
    pub(crate) objects: HashMap<i32, usize>,
    pub(crate) original_size: usize,
    pub(crate) encoded_size: usize,
    pub(crate) max_bytes: usize,
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl Reader<'_> {
    fn error(&self, message: &str) -> Error {
        Error(format!("At byte {}: {}", self.pos, message))
    }
    fn take(&mut self, len: usize) -> Result<&[u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| self.error("length overflow"))?;
        if end > self.bytes.len() {
            return Err(self.error("truncated payload"));
        }
        let value = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(value)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<WireString> {
        let flag = self.byte()?;
        if flag > 1 {
            return Err(self.error("unknown string encoding"));
        }
        let count = self.i32()?;
        if count < 0 {
            return Err(self.error("negative string length"));
        }
        let length = (count as usize)
            .checked_mul(if flag == 1 { 2 } else { 1 })
            .ok_or_else(|| self.error("string length overflow"))?;
        Ok(WireString {
            wide: flag == 1,
            bytes: self.take(length)?.to_vec(),
        })
    }
}

fn named(tag: u8) -> bool {
    matches!(tag, 1 | 3 | 50) || ((9..=45).contains(&tag) && tag % 2 == 1)
}
pub(crate) fn base_tag(tag: u8) -> u8 {
    if named(tag) && (9..=45).contains(&tag) {
        tag + 1
    } else {
        tag
    }
}
pub(crate) fn terminal(tag: u8) -> bool {
    matches!(tag, 5 | 7 | 49)
}

impl Document {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        Self::decode_with_limits(bytes, Limits::default())
    }
    pub fn decode_with_limits(bytes: &[u8], limits: Limits) -> Result<Self> {
        if bytes.len() > limits.max_bytes {
            return Err(Error("File exceeds size limit".into()));
        }
        let mut r = Reader { bytes, pos: 0 };
        let mut doc = Self {
            next_object_id: std::cell::Cell::new(None),
            records: Records::default(),
            roots: vec![],
            types: HashMap::new(),
            objects: HashMap::new(),
            original_size: bytes.len(),
            encoded_size: bytes.len(),
            max_bytes: limits.max_bytes,
        };
        let mut stack: Vec<usize> = vec![];
        let mut references = HashSet::new();
        while r.pos < bytes.len() {
            if doc.records.len() >= limits.max_records {
                return Err(r.error("record limit exceeded"));
            }
            let offset = r.pos;
            let tag = r.byte()?;
            let name = if named(tag) { Some(r.string()?) } else { None };
            let payload = match tag {
                1..=4 => {
                    let ty = match r.byte()? {
                        46 => TypeInfo::None,
                        47 => {
                            let id = r.i32()?;
                            let value = r.string()?;
                            if id < 0 || doc.types.insert(id, value.display()).is_some() {
                                return Err(r.error("invalid/duplicate type ID"));
                            }
                            TypeInfo::Definition(id, value)
                        }
                        48 => {
                            let id = r.i32()?;
                            if !doc.types.contains_key(&id) {
                                return Err(r.error("undeclared type ID"));
                            }
                            TypeInfo::Reference(id)
                        }
                        _ => return Err(r.error("invalid type descriptor")),
                    };
                    let object = if tag <= 2 {
                        let id = r.i32()?;
                        if id < 0 || doc.objects.insert(id, doc.records.len()).is_some() {
                            return Err(r.error("invalid/duplicate object ID"));
                        }
                        Some(id)
                    } else {
                        None
                    };
                    Payload::Node { ty, object }
                }
                5 | 7 => {
                    let start = stack
                        .pop()
                        .ok_or_else(|| r.error("unexpected container end"))?;
                    let record = &doc.records[start];
                    if (tag == 7) != (record.tag == 6) {
                        return Err(r.error("mismatched container end"));
                    }
                    if let Payload::Array(count) = record.payload {
                        if count as u64 != record.children.len() as u64 {
                            return Err(r.error("array element count mismatch"));
                        }
                    }
                    Payload::Empty
                }
                6 => {
                    let count = r.i64()?;
                    if count < 0 || count as u64 > limits.max_records as u64 {
                        return Err(r.error("invalid array count"));
                    }
                    Payload::Array(count)
                }
                8 => {
                    let count = r.i32()?;
                    let width = r.i32()?;
                    if count < 0 || !matches!(width, 1 | 2 | 4 | 8 | 16) {
                        return Err(r.error("invalid primitive array dimensions"));
                    }
                    let len = (count as usize)
                        .checked_mul(width as usize)
                        .ok_or_else(|| r.error("primitive array overflow"))?;
                    Payload::Packed {
                        count,
                        width,
                        bytes: r.take(len)?.to_vec(),
                    }
                }
                39 | 40 | 50 | 51 => Payload::Text(r.string()?),
                45 | 46 => Payload::Empty,
                49 => {
                    if !stack.is_empty() || r.pos != bytes.len() {
                        return Err(r.error("unexpected end of stream or trailing data"));
                    }
                    Payload::Empty
                }
                _ => {
                    let width = match base_tag(tag) {
                        16 | 18 | 44 => 1,
                        20 | 22 | 38 => 2,
                        10 | 12 | 24 | 26 | 32 => 4,
                        28 | 30 | 34 => 8,
                        14 | 36 | 42 => 16,
                        _ => return Err(r.error(&format!("unsupported token {tag}"))),
                    };
                    let data = r.take(width)?.to_vec();
                    if base_tag(tag) == 10 {
                        references.insert(i32::from_le_bytes(data.as_slice().try_into().unwrap()));
                    }
                    if base_tag(tag) == 44 && data[0] > 1 {
                        return Err(r.error("invalid boolean"));
                    }
                    Payload::Fixed(data)
                }
            };
            let id = doc.records.len();
            let parent = stack.last().copied();
            if !terminal(tag) {
                if let Some(parent) = parent {
                    doc.records[parent].children.push(id);
                } else {
                    doc.roots.push(id);
                }
            }
            doc.records.push(Record {
                tag,
                name,
                payload,
                parent,
                children: vec![],
                offset,
            });
            if matches!(tag, 1..=4 | 6) {
                if stack.len() >= limits.max_depth {
                    return Err(r.error("nesting limit exceeded"));
                }
                stack.push(id);
            }
        }
        if !stack.is_empty() {
            return Err(r.error("unclosed container"));
        }
        if doc.roots.len() != 1 {
            return Err(r.error("expected one root value"));
        }
        if references.iter().any(|id| !doc.objects.contains_key(id)) {
            return Err(r.error("dangling object reference"));
        }
        Ok(doc)
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.encoded_size);
        let definitions: HashMap<_, _> = self
            .records
            .iter()
            .filter_map(|r| match &r.payload {
                Payload::Node {
                    ty: TypeInfo::Definition(id, name),
                    ..
                } => Some((*id, name)),
                _ => None,
            })
            .collect();
        let mut emitted = HashSet::new();
        let mut stack: Vec<(usize, bool)> =
            self.roots.iter().rev().map(|id| (*id, false)).collect();
        while let Some((id, end)) = stack.pop() {
            let record = &self.records[id];
            if end {
                output.push(if record.tag == 6 { 7 } else { 5 });
                continue;
            }
            let mut header = record.clone();
            header.children.clear();
            if let Payload::Node { ty, .. } = &mut header.payload {
                let type_id = match ty {
                    TypeInfo::Definition(id, _) | TypeInfo::Reference(id) => Some(*id),
                    _ => None,
                };
                if let Some(type_id) = type_id {
                    if emitted.insert(type_id) {
                        if let Some(name) = definitions.get(&type_id) {
                            *ty = TypeInfo::Definition(type_id, (*name).clone());
                        }
                    } else {
                        *ty = TypeInfo::Reference(type_id);
                    }
                }
            }
            header.write(&mut output);
            if matches!(record.tag, 1..=4 | 6) {
                stack.push((id, true));
                stack.extend(record.children.iter().rev().map(|child| (*child, false)));
            }
        }
        if self.records.iter().any(|r| r.tag == 49) {
            output.push(49);
        }
        output
    }

    pub(crate) fn reachable(&self) -> Result<Vec<usize>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut stack: Vec<_> = self.roots.iter().map(|id| (*id, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            if depth > Limits::default().max_depth || !seen.insert(id) {
                return Err(Error("Invalid/cyclic container hierarchy".into()));
            }
            let record = self
                .records
                .get(id)
                .ok_or_else(|| Error("Unknown node".into()))?;
            if record.tag == 0 || terminal(record.tag) {
                return Err(Error("Deleted node".into()));
            }
            out.push(id);
            stack.extend(
                record
                    .children
                    .iter()
                    .rev()
                    .map(|child| (*child, depth + 1)),
            );
        }
        Ok(out)
    }
    #[cfg(test)]
    pub(crate) fn rebuild(&mut self) -> Result<()> {
        let ids = self.reachable()?;
        if self.records.len() > Limits::default().max_records {
            return Err(Error("Record limit exceeded".into()));
        }
        self.objects.clear();
        for id in &ids {
            if let Payload::Node {
                object: Some(object),
                ..
            } = self.records[*id].payload
            {
                if self.objects.insert(object, *id).is_some() {
                    return Err(Error("Duplicate object ID".into()));
                }
            }
        }
        for id in &ids {
            let r = &self.records[*id];
            if base_tag(r.tag) == 10 {
                if let Payload::Fixed(bytes) = &r.payload {
                    let object = i32::from_le_bytes(
                        bytes
                            .as_slice()
                            .try_into()
                            .map_err(|_| Error("Invalid reference".into()))?,
                    );
                    if !self.objects.contains_key(&object) {
                        return Err(Error("Removal would leave a dangling reference; retarget it in the same transaction".into()));
                    }
                }
            }
            for child in &r.children {
                if self.records[*child].parent != Some(*id) {
                    return Err(Error("Invalid parent link".into()));
                }
            }
        }
        for id in ids {
            let r = &mut self.records[id];
            if matches!(r.payload, Payload::Array(_)) {
                r.payload = Payload::Array(r.children.len() as i64);
            }
        }
        self.encoded_size = self.encode().len();
        if self.encoded_size > self.max_bytes {
            return Err(Error("Document exceeds size limit".into()));
        }
        Ok(())
    }
    pub fn record_count(&self) -> usize {
        self.records.len()
    }
    pub fn type_count(&self) -> usize {
        self.types.len()
    }
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}
