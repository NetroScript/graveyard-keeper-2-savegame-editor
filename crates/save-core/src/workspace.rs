use crate::service::{replacement, view};
use crate::wire::{base_tag, Payload, Record, TypeInfo, WireString};
use crate::{Document, Error, NodeView};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSummary {
    pub document_id: u32,
    pub revision: u32,
    pub original_bytes: usize,
    pub encoded_bytes: Option<usize>,
    pub records: usize,
    pub types: usize,
    pub objects: usize,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Summary,
    Children {
        parent: Option<usize>,
        offset: usize,
        limit: usize,
    },
    Nodes {
        ids: Vec<usize>,
    },
    General,
    Drops,
    Progression,
    InventoryCatalog {
        catalog: crate::inventory::Catalog,
    },
    Inventories,
    Templates,
    Transact {
        revision: u32,
        operations: Vec<Operation>,
    },
    Undo {
        revision: u32,
    },
    Redo {
        revision: u32,
    },
    MarkSaved {
        revision: u32,
    },
    Close,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    Set {
        node: usize,
        tag: u8,
        value: String,
    },
    Insert {
        parent: usize,
        index: usize,
        name: Option<String>,
        kind: String,
        value: String,
    },
    Template {
        parent: usize,
        index: usize,
        name: Option<String>,
        template: String,
        #[serde(default)]
        values: BTreeMap<String, String>,
    },
    Duplicate {
        node: usize,
    },
    Remove {
        node: usize,
    },
    Move {
        node: usize,
        index: usize,
    },
    Retarget {
        node: usize,
        target: usize,
    },
    General {
        values: BTreeMap<String, String>,
    },
    Inventory {
        container: usize,
        action: crate::inventory::Edit,
        #[serde(default)]
        out_of_bounds: bool,
    },
    Drops {
        action: crate::drops::Edit,
    },
    Progression {
        action: crate::progression::Edit,
    },
}
mod transactions;
pub use transactions::Workspace;
pub(crate) use transactions::{failure, text};
pub(crate) fn active(doc: &Document, id: usize) -> Result<&Record, Error> {
    let record = doc.records.get(id).ok_or_else(|| failure("Unknown node"))?;
    let mut cursor = id;
    let mut depth = 0;
    loop {
        let r = &doc.records[cursor];
        if let Some(parent) = r.parent {
            let p = doc
                .records
                .get(parent)
                .ok_or_else(|| failure("Invalid parent"))?;
            if !p.children.contains(&cursor) {
                return Err(failure("Node was removed"));
            }
            cursor = parent;
        } else {
            if !doc.roots.contains(&cursor) {
                return Err(failure("Node was removed"));
            }
            break;
        }
        depth += 1;
        if depth > 256 || (depth == 256 && matches!(record.tag, 1..=4 | 6)) {
            return Err(failure("Hierarchy depth exceeded"));
        }
    }
    Ok(record)
}
pub(crate) fn resolve(doc: &Document, id: usize) -> Result<usize, Error> {
    let r = active(doc, id)?;
    if base_tag(r.tag) == 10 {
        if let Payload::Fixed(b) = &r.payload {
            return doc
                .objects
                .get(&i32::from_le_bytes(b.as_slice().try_into().unwrap()))
                .copied()
                .ok_or_else(|| failure("Unresolved reference"));
        }
    }
    Ok(id)
}
pub(crate) fn field(doc: &Document, parent: usize, name: &str) -> Result<usize, Error> {
    let parent = resolve(doc, parent)?;
    let matches: Vec<_> = active(doc, parent)?
        .children
        .iter()
        .filter(|id| {
            doc.records[**id]
                .name
                .as_ref()
                .is_some_and(|n| n.display() == name)
        })
        .copied()
        .collect();
    if matches.len() != 1 {
        return Err(failure(&format!("Missing or ambiguous field: {name}")));
    }
    resolve(doc, matches[0])
}
pub(crate) fn scalar(kind: &str, name: Option<String>, value: &str) -> Result<Record, Error> {
    let base = match kind {
        "i8" => 16,
        "u8" => 18,
        "i16" => 20,
        "u16" => 22,
        "i32" => 24,
        "u32" => 26,
        "i64" => 28,
        "u64" => 30,
        "f32" => 32,
        "f64" => 34,
        "string" => 40,
        "bool" => 44,
        "null" => 46,
        "array" => 6,
        _ => return Err(failure("Unsupported scalar type")),
    };
    let tag = if name.is_some() && base != 6 {
        base - 1
    } else {
        base
    };
    if base == 6 && name.is_some() {
        return Err(failure(
            "Odin arrays cannot be named; use the containing list node",
        ));
    }
    let payload = match base {
        46 => Payload::Empty,
        6 => Payload::Array(0),
        40 => Payload::Text(text(value)),
        _ => replacement(tag, &Payload::Empty, value)?,
    };
    Ok(Record {
        tag,
        name: name.as_deref().map(text),
        payload,
        parent: None,
        children: vec![],
        offset: 0,
    })
}
pub(crate) fn attach(
    doc: &mut Document,
    parent: usize,
    index: usize,
    mut r: Record,
) -> Result<usize, Error> {
    let p = active(doc, parent)?;
    if !matches!(p.tag, 1..=4 | 6) || index > p.children.len() {
        return Err(failure("Choose a container and valid insertion index"));
    }
    if p.tag == 6 && r.name.is_some() {
        return Err(failure("Array elements must be unnamed"));
    }
    let id = doc.records.len();
    r.parent = Some(parent);
    doc.records.push(r);
    doc.records[parent].children.insert(index, id);
    Ok(id)
}
pub(crate) fn next_object(doc: &Document) -> Result<i32, Error> {
    let id = if let Some(id) = doc.next_object_id.get() {
        id
    } else {
        doc.records
            .iter()
            .filter_map(|r| match r.payload {
                Payload::Node {
                    object: Some(id), ..
                } => Some(id),
                _ => None,
            })
            .max()
            .unwrap_or(-1)
            .checked_add(1)
            .ok_or_else(|| failure("Object ID limit exceeded"))?
    };
    doc.next_object_id.set(Some(
        id.checked_add(1)
            .ok_or_else(|| failure("Object ID limit exceeded"))?,
    ));
    Ok(id)
}
pub(crate) fn create_template(
    doc: &mut Document,
    parent: usize,
    index: usize,
    name: Option<String>,
    template: &str,
    values: BTreeMap<String, String>,
) -> Result<usize, Error> {
    let t = TEMPLATES
        .iter()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .find(|t| t["id"] == template)
        .ok_or_else(|| failure("Unknown template"))?;
    let type_name = t["typeName"]
        .as_str()
        .ok_or_else(|| failure("Invalid template"))?;
    let (type_id, new_type) =
        if let Some((id, _)) = doc.types.iter().find(|(_, n)| n.as_str() == type_name) {
            (*id, false)
        } else {
            let id = doc
                .types
                .keys()
                .max()
                .copied()
                .unwrap_or(-1)
                .checked_add(1)
                .ok_or_else(|| failure("Type ID limit exceeded"))?;
            doc.types.insert(id, type_name.into());
            (id, true)
        };
    let object = if t["reference"].as_bool().unwrap_or(false) {
        Some(next_object(doc)?)
    } else {
        None
    };
    let tag = match (object.is_some(), name.is_some()) {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 3,
        (false, false) => 4,
    };
    let record = Record {
        tag,
        name: name.as_deref().map(text),
        payload: Payload::Node {
            ty: if new_type {
                TypeInfo::Definition(type_id, text(type_name))
            } else {
                TypeInfo::Reference(type_id)
            },
            object,
        },
        parent: None,
        children: vec![],
        offset: 0,
    };
    let node = attach(doc, parent, index, record)?;
    for (i, f) in t["fields"]
        .as_array()
        .ok_or_else(|| failure("Invalid template fields"))?
        .iter()
        .enumerate()
    {
        let n = f["name"].as_str().map(str::to_owned);
        let key = n.clone().unwrap_or_else(|| i.to_string());
        let value = values
            .get(&key)
            .map(String::as_str)
            .unwrap_or(f["value"].as_str().unwrap_or("0"));
        let r = scalar(f["kind"].as_str().unwrap_or(""), n, value)?;
        attach(doc, node, i, r)?;
    }
    Ok(node)
}
pub(crate) fn apply(doc: &mut Document, op: Operation) -> Result<(), Error> {
    match op {
        Operation::Set { node, tag, value } => {
            if value.len() > doc.max_bytes / 2 {
                return Err(failure("Value exceeds size limit"));
            }
            let r = active(doc, node)?;
            if r.tag != tag {
                return Err(failure("Field type changed"));
            }
            let p = replacement(tag, &r.payload, &value)?;
            doc.records[node].payload = p;
        }
        Operation::Insert {
            parent,
            index,
            name,
            kind,
            value,
        } => {
            attach(doc, parent, index, scalar(&kind, name, &value)?)?;
        }
        Operation::Template {
            parent,
            index,
            name,
            template,
            values,
        } => {
            create_template(doc, parent, index, name, &template, values)?;
        }
        Operation::Remove { node } => {
            let parent = active(doc, node)?
                .parent
                .ok_or_else(|| failure("Cannot remove the root"))?;
            doc.records[parent].children.retain(|id| *id != node);
        }
        Operation::Move { node, index } => {
            let parent = active(doc, node)?
                .parent
                .ok_or_else(|| failure("Cannot move the root"))?;
            let children = &mut doc.records[parent].children;
            if index >= children.len() {
                return Err(failure("Invalid sibling position"));
            }
            children.retain(|id| *id != node);
            children.insert(index, node);
        }
        Operation::Retarget { node, target } => {
            if base_tag(active(doc, node)?.tag) != 10 {
                return Err(failure("Select an internal reference"));
            }
            let Payload::Node {
                object: Some(object),
                ..
            } = active(doc, target)?.payload
            else {
                return Err(failure("Target must declare an object"));
            };
            doc.records[node].payload = Payload::Fixed(object.to_le_bytes().to_vec());
        }
        Operation::Duplicate { node } => {
            let parent = active(doc, node)?
                .parent
                .ok_or_else(|| failure("Cannot duplicate the root"))?;
            let index = doc.records[parent]
                .children
                .iter()
                .position(|id| *id == node)
                .unwrap()
                + 1;
            let mut ids = vec![];
            let mut pending = vec![node];
            while let Some(id) = pending.pop() {
                ids.push(id);
                pending.extend(doc.records[id].children.iter().rev());
            }
            let start = doc.records.len();
            let mapping: HashMap<_, _> = ids
                .iter()
                .enumerate()
                .map(|(i, id)| (*id, start + i))
                .collect();
            let mut objects = HashMap::new();
            let mut next = next_object(doc)?;
            for id in &ids {
                if let Payload::Node {
                    object: Some(old), ..
                } = doc.records[*id].payload
                {
                    objects.insert(old, next);
                    next = next
                        .checked_add(1)
                        .ok_or_else(|| failure("Object ID limit exceeded"))?;
                }
            }
            doc.next_object_id.set(Some(next));
            for id in &ids {
                let mut r = doc.records[*id].clone();
                r.parent = if *id == node {
                    Some(parent)
                } else {
                    r.parent.map(|p| mapping[&p])
                };
                r.children = r.children.iter().map(|c| mapping[c]).collect();
                r.offset = 0;
                if let Payload::Node {
                    object: Some(obj), ..
                } = &mut r.payload
                {
                    *obj = objects[obj];
                }
                if base_tag(r.tag) == 10 {
                    if let Payload::Fixed(b) = &mut r.payload {
                        let old = i32::from_le_bytes(b.as_slice().try_into().unwrap());
                        if let Some(new) = objects.get(&old) {
                            *b = new.to_le_bytes().to_vec();
                        }
                    }
                }
                doc.records.push(r);
            }
            doc.records[parent].children.insert(index, start);
        }
        Operation::General { values } => crate::general::write(doc, values)?,
        Operation::Inventory { .. } => return Err(failure("Inventory edits require a catalog")),
        Operation::Drops { .. } => return Err(failure("Drop edits require a transaction")),
        Operation::Progression { .. } => {
            return Err(failure("Progression edits require a transaction"))
        }
    }
    Ok(())
}
