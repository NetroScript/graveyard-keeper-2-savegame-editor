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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub(crate) enum TypeInfo {
    None,
    Definition(i32, WireString),
    Reference(i32),
}
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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

/// Owns parsed records, not the original file. Encoding reconstructs every header and payload.
pub struct Document {
    pub(crate) records: Vec<Record>,
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
            records: vec![],
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
        for record in &self.records {
            record.write(&mut output);
        }
        output
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
