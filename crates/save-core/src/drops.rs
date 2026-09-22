//! Discovery and atomic cleanup of persisted world drops.
use crate::service::view;
use crate::wire::base_tag;
use crate::workspace::{apply, failure, field, resolve, Operation};
use crate::{Document, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Remove { nodes: Vec<usize> },
}

struct Scene {
    id: String,
    node: usize,
}

struct DropEntry {
    node: usize,
    source: &'static str,
    kind: String,
    id: String,
    count: i32,
    world: String,
    x: String,
    y: String,
    z: String,
}

fn value(doc: &Document, parent: usize, name: &str) -> Result<String, Error> {
    view(doc, field(doc, parent, name)?)
        .value
        .ok_or_else(|| failure(&format!("Missing drop field: {name}")))
}

fn integer(doc: &Document, parent: usize, name: &str) -> Result<i32, Error> {
    let node = field(doc, parent, name)?;
    if !matches!(
        base_tag(doc.records[node].tag),
        16 | 18 | 20 | 22 | 24 | 26 | 28 | 30
    ) {
        return Err(failure(&format!("Drop field is not an integer: {name}")));
    }
    value(doc, parent, name)?
        .parse::<i64>()
        .ok()
        .and_then(|number| i32::try_from(number).ok())
        .ok_or_else(|| failure(&format!("Invalid drop number: {name}")))
}

fn array(doc: &Document, list: usize) -> Result<usize, Error> {
    let list = resolve(doc, list)?;
    let arrays: Vec<_> = doc.records[list]
        .children
        .iter()
        .filter(|id| doc.records[**id].tag == 6)
        .copied()
        .collect();
    if arrays.len() != 1 {
        return Err(failure("Unsupported drop collection layout"));
    }
    Ok(arrays[0])
}

fn list(doc: &Document, parent: usize, name: &str) -> Result<usize, Error> {
    array(doc, field(doc, parent, name)?)
}

fn scenes(doc: &Document) -> Result<Vec<Scene>, Error> {
    let root = *doc
        .roots
        .first()
        .ok_or_else(|| failure("Missing save root"))?;
    let world = field(doc, root, "worldData")?;
    let scenes = list(doc, world, "gameSceneDataList")?;
    doc.records[scenes]
        .children
        .iter()
        .map(|entry| {
            let node = resolve(doc, *entry)?;
            Ok(Scene {
                id: value(doc, node, "id")?,
                node,
            })
        })
        .collect()
}

fn position(doc: &Document, parent: usize, name: &str) -> Result<(String, String, String), Error> {
    let position = resolve(doc, field(doc, parent, name)?)?;
    let children = &doc.records[position].children;
    if children.len() != 3 {
        return Err(failure("Drop position is not a Vector3"));
    }
    let names = ["x", "y", "z"];
    let mut values = vec![];
    for (index, child) in children.iter().enumerate() {
        let child = resolve(doc, *child)?;
        let record = &doc.records[child];
        let valid_name = record.name.as_ref().is_none_or(|name| {
            let name = name.display();
            name == names[index] || name == format!("m_{}", names[index])
        });
        if !valid_name || base_tag(record.tag) != 32 {
            return Err(failure("Drop position has an unsupported Vector3 layout"));
        }
        values.push(
            view(doc, child)
                .value
                .ok_or_else(|| failure("Missing drop position component"))?,
        );
    }
    Ok((values.remove(0), values.remove(0), values.remove(0)))
}

fn ordinary_drops(doc: &Document) -> Result<Vec<DropEntry>, Error> {
    let mut result = vec![];
    for scene in scenes(doc)? {
        for (list_name, source) in [("droppedItems", "Dropped"), ("queuedDrops", "Queued")] {
            let list = list(doc, scene.node, list_name)?;
            for entry in &doc.records[list].children {
                let drop = resolve(doc, *entry)?;
                let item = field(doc, drop, "item")?;
                let id = value(doc, item, "id")?;
                let count = integer(doc, item, "count")?;
                if count < 0 {
                    return Err(failure("Drop count is negative"));
                }
                let drop_type = integer(doc, drop, "dropType")?;
                let kind = match drop_type {
                    0 => "Item",
                    1 => "World object",
                    _ => "Unknown",
                }
                .to_owned();
                let world = value(doc, drop, "worldId").unwrap_or_else(|_| scene.id.clone());
                let (x, y, z) = position(doc, drop, "pos")?;
                result.push(DropEntry {
                    node: *entry,
                    source,
                    kind,
                    id,
                    count,
                    world,
                    x,
                    y,
                    z,
                });
            }
        }
    }
    Ok(result)
}

pub(crate) fn read(doc: &Document) -> Result<Value, Error> {
    let drops = ordinary_drops(doc)?;
    Ok(json!({
        "drops": drops.into_iter().map(|drop| json!({
            "node": drop.node,
            "source": drop.source,
            "type": drop.kind,
            "id": drop.id,
            "count": drop.count.to_string(),
            "location": {
                "world": drop.world,
                "x": drop.x,
                "y": drop.y,
                "z": drop.z
            }
        })).collect::<Vec<_>>()
    }))
}

pub(crate) fn write(doc: &mut Document, edit: Edit) -> Result<(), Error> {
    match edit {
        Edit::Remove { nodes } => {
            if nodes.is_empty() || nodes.len() > 100_000 {
                return Err(failure("Select between 1 and 100,000 drops"));
            }
            let available: HashSet<_> = ordinary_drops(doc)?
                .into_iter()
                .map(|drop| drop.node)
                .collect();
            let mut selected = HashSet::new();
            for node in nodes {
                if !selected.insert(node) {
                    return Err(failure("A drop was selected more than once"));
                }
                if !available.contains(&node) {
                    return Err(failure("A selected drop is no longer available"));
                }
                apply(doc, Operation::Remove { node })?;
            }
        }
    }
    Ok(())
}
