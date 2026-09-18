use crate::wire::{base_tag, Document, Error, Payload, TypeInfo};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub original_bytes: usize,
    pub encoded_bytes: usize,
    pub records: usize,
    pub types: usize,
    pub objects: usize,
    pub revision: u32,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TreeField {
    pub id: usize,
    pub name: Option<String>,
    pub kind: String,
    pub tag: u8,
    pub value: String,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    pub id: usize,
    pub parent: Option<usize>,
    pub name: Option<String>,
    pub kind: String,
    pub tag: u8,
    pub type_name: Option<String>,
    pub value: Option<String>,
    pub reference_target: Option<usize>,
    pub child_count: usize,
    pub tree_fields: Vec<TreeField>,
    /// Input-file offset; does not change after variable-length edits.
    pub original_offset: usize,
    pub editable: bool,
}
#[derive(Serialize, Debug)]
pub struct Page {
    pub nodes: Vec<NodeView>,
    pub total: usize,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditRequest {
    pub node: usize,
    pub expected_tag: u8,
    pub revision: u32,
    pub value: String,
}
#[derive(Deserialize, Debug)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Children {
        parent: Option<usize>,
        offset: usize,
        limit: usize,
    },
    SetValue {
        edit: EditRequest,
    },
    Close,
}
#[derive(Serialize, Debug)]
#[serde(tag = "op", content = "data", rename_all = "snake_case")]
pub enum Response {
    Children(Page),
    SetValue(Summary),
    Close,
}

/// One open document per service. Adapters only transport bytes and these DTOs.
#[derive(Default)]
pub struct Service {
    document: Option<Document>,
    revision: u32,
}
impl Service {
    pub fn open(&mut self, bytes: &[u8]) -> Result<Summary, Error> {
        let document = Document::decode(bytes)?;
        self.bump()?;
        self.document = Some(document);
        self.summary()
    }
    fn bump(&mut self) -> Result<(), Error> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| Error("Session revision exhausted".into()))?;
        Ok(())
    }
    pub fn summary(&self) -> Result<Summary, Error> {
        let doc = self.doc()?;
        Ok(Summary {
            original_bytes: doc.original_size,
            encoded_bytes: doc.encoded_size,
            records: doc.record_count(),
            types: doc.type_count(),
            objects: doc.object_count(),
            revision: self.revision,
        })
    }
    fn doc(&self) -> Result<&Document, Error> {
        self.document
            .as_ref()
            .ok_or_else(|| Error("No save is open".into()))
    }
    pub fn export(&self) -> Result<Vec<u8>, Error> {
        Ok(self.doc()?.encode())
    }
    pub fn request(&mut self, request: Request) -> Result<Response, Error> {
        match request {
            Request::Children {
                parent,
                offset,
                limit,
            } => {
                if limit == 0 || limit > 200 {
                    return Err(Error("Page size must be 1..200".into()));
                }
                let doc = self.doc()?;
                let ids = match parent {
                    Some(id) => {
                        &doc.records
                            .get(id)
                            .ok_or_else(|| Error("Unknown node".into()))?
                            .children
                    }
                    None => &doc.roots,
                };
                let nodes = ids
                    .iter()
                    .skip(offset)
                    .take(limit)
                    .map(|id| view(doc, *id))
                    .collect();
                Ok(Response::Children(Page {
                    nodes,
                    total: ids.len(),
                }))
            }
            Request::SetValue { edit } => {
                if edit.revision != self.revision {
                    return Err(Error("Stale edit; reload the current document view".into()));
                }
                // Compute/validate replacement before touching the document or revision.
                let doc = self.doc()?;
                let record = doc
                    .records
                    .get(edit.node)
                    .ok_or_else(|| Error("Unknown node".into()))?;
                if record.tag != edit.expected_tag {
                    return Err(Error("Field type changed".into()));
                }
                if edit.value.len() > doc.max_bytes / 2 {
                    return Err(Error("Value exceeds size limit".into()));
                }
                let replacement = replacement(record.tag, &record.payload, &edit.value)?;
                let old_size = payload_size(&record.payload);
                let new_size = payload_size(&replacement);
                let encoded_size = doc.encoded_size - old_size + new_size;
                if encoded_size > doc.max_bytes {
                    return Err(Error("Edited document exceeds size limit".into()));
                }
                self.bump()?;
                let doc = self.document.as_mut().unwrap();
                doc.records[edit.node].payload = replacement;
                doc.encoded_size = encoded_size;
                Ok(Response::SetValue(self.summary()?))
            }
            Request::Close => {
                self.bump()?;
                self.document = None;
                Ok(Response::Close)
            }
        }
    }
}
fn payload_size(payload: &Payload) -> usize {
    match payload {
        Payload::Text(s) => 5 + s.bytes.len(),
        Payload::Fixed(b) => b.len(),
        _ => 0,
    }
}
pub(crate) fn replacement(tag: u8, payload: &Payload, value: &str) -> Result<Payload, Error> {
    let invalid = || Error("Invalid value or value outside the original wire type's range".into());
    macro_rules! integer {
        ($ty:ty) => {
            Payload::Fixed(
                value
                    .parse::<$ty>()
                    .map_err(|_| invalid())?
                    .to_le_bytes()
                    .to_vec(),
            )
        };
    }
    Ok(match base_tag(tag) {
        16 => integer!(i8),
        18 => integer!(u8),
        20 => integer!(i16),
        22 => integer!(u16),
        24 => integer!(i32),
        26 => integer!(u32),
        28 => integer!(i64),
        30 => integer!(u64),
        32 => {
            let number = value.parse::<f32>().map_err(|_| invalid())?;
            if !number.is_finite() {
                return Err(invalid());
            }
            Payload::Fixed(number.to_le_bytes().to_vec())
        }
        34 => {
            let number = value.parse::<f64>().map_err(|_| invalid())?;
            if !number.is_finite() {
                return Err(invalid());
            }
            Payload::Fixed(number.to_le_bytes().to_vec())
        }
        40 => {
            if let Payload::Text(text) = payload {
                Payload::Text(text.replace(value))
            } else {
                return Err(invalid());
            }
        }
        44 => Payload::Fixed(vec![match value {
            "true" => 1,
            "false" => 0,
            _ => return Err(invalid()),
        }]),
        _ => return Err(Error("This wire value is read-only".into())),
    })
}
pub(crate) fn view(doc: &Document, id: usize) -> NodeView {
    let record = &doc.records[id];
    let mut type_name = None;
    let mut reference_target = None;
    let value = match &record.payload {
        Payload::Node { ty, object } => {
            type_name = match ty {
                TypeInfo::Definition(id, _) | TypeInfo::Reference(id) => doc.types.get(id).cloned(),
                TypeInfo::None => None,
            };
            object.map(|id| format!("object #{id}"))
        }
        Payload::Text(text) => Some(text.display()),
        Payload::Array(count) => Some(format!("{count} elements")),
        Payload::Packed { count, width, .. } => Some(format!("{count} × {width} bytes")),
        Payload::Empty => None,
        Payload::Fixed(bytes) => {
            macro_rules! number {
                ($ty:ty) => {
                    <$ty>::from_le_bytes(bytes.as_slice().try_into().unwrap()).to_string()
                };
            }
            Some(match base_tag(record.tag) {
                10 => {
                    let object = i32::from_le_bytes(bytes.as_slice().try_into().unwrap());
                    reference_target = doc.objects.get(&object).copied();
                    format!("object #{object}")
                }
                16 => number!(i8),
                18 => number!(u8),
                20 => number!(i16),
                22 => number!(u16),
                24 => number!(i32),
                26 => number!(u32),
                28 => number!(i64),
                30 => number!(u64),
                32 => number!(f32),
                34 => number!(f64),
                44 => (bytes[0] == 1).to_string(),
                _ => bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(""),
            })
        }
    };
    let kind = match base_tag(record.tag) {
        1 | 2 => "reference_node",
        3 | 4 => "struct",
        6 => "array",
        8 => "primitive_array",
        10 => "internal_reference",
        12 | 14 | 50 | 51 => "external_reference",
        16 => "i8",
        18 => "u8",
        20 => "i16",
        22 => "u16",
        24 => "i32",
        26 => "u32",
        28 => "i64",
        30 => "u64",
        32 => "f32",
        34 => "f64",
        36 => "decimal",
        38 => "char",
        40 => "string",
        42 => "guid",
        44 => "bool",
        46 => "null",
        _ => "end",
    }
    .to_owned();
    let tree_fields = record
        .children
        .iter()
        .filter_map(|child_id| {
            let child = &doc.records[*child_id];
            scalar_display(child).map(|(kind, value)| TreeField {
                id: *child_id,
                name: child.name.as_ref().map(|name| name.display()),
                kind,
                tag: child.tag,
                value,
            })
        })
        .collect();
    NodeView {
        id,
        parent: record.parent,
        name: record.name.as_ref().map(|s| s.display()),
        kind,
        tag: record.tag,
        type_name,
        value,
        reference_target,
        child_count: record.children.len(),
        tree_fields,
        original_offset: record.offset,
        editable: matches!(
            base_tag(record.tag),
            16 | 18 | 20 | 22 | 24 | 26 | 28 | 30 | 32 | 34 | 40 | 44
        ),
    }
}

fn scalar_display(record: &crate::wire::Record) -> Option<(String, String)> {
    let base = base_tag(record.tag);
    let kind = match base {
        16 => "i8",
        18 => "u8",
        20 => "i16",
        22 => "u16",
        24 => "i32",
        26 => "u32",
        28 => "i64",
        30 => "u64",
        32 => "f32",
        34 => "f64",
        40 => "string",
        44 => "bool",
        _ => return None,
    }
    .to_owned();
    let value = match &record.payload {
        Payload::Text(text) if base == 40 => text.display(),
        Payload::Fixed(bytes) => {
            macro_rules! number {
                ($ty:ty) => {
                    <$ty>::from_le_bytes(bytes.as_slice().try_into().ok()?).to_string()
                };
            }
            match base {
                16 => number!(i8),
                18 => number!(u8),
                20 => number!(i16),
                22 => number!(u16),
                24 => number!(i32),
                26 => number!(u32),
                28 => number!(i64),
                30 => number!(u64),
                32 => number!(f32),
                34 => number!(f64),
                44 if bytes.len() == 1 => (bytes[0] == 1).to_string(),
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((kind, value))
}
