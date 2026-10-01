use crate::inventory::{new_item, Definition};
use crate::service::view;
use crate::wire::{base_tag, Payload, WireString};
use crate::workspace::{apply, create_template, failure, field, resolve, Operation};
use crate::{Document, Error};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const RESOURCES: &[&str] = &[
    "energy",
    "stamina",
    "money",
    "insanity",
    "happiness",
    "tech_red",
    "tech_green",
    "tech_blue",
];
/// Currencies stored as item stacks. Faith is kept in the player inventory, science in the
/// study table. The game totals top-level stacks and stacks one level deep inside bags.
const ITEMS: &[&str] = &["faith", "science"];
/// `ItemDef.stackCount` of both currency items.
const ITEM_STACK: i32 = 999;
const STUDY_TABLE: &str = "survey_wgo";
fn player(doc: &Document) -> Result<usize, Error> {
    field(
        doc,
        *doc.roots.first().ok_or_else(|| failure("Missing root"))?,
        "playerData",
    )
}
fn array(doc: &Document, list: usize) -> Result<usize, Error> {
    let list = resolve(doc, list)?;
    let ids: Vec<_> = doc.records[list]
        .children
        .iter()
        .copied()
        .filter(|id| doc.records[*id].tag == 6)
        .collect();
    if ids.len() != 1 {
        return Err(failure("Unsupported resource list layout"));
    }
    Ok(ids[0])
}
fn resources(doc: &Document) -> Result<(usize, usize, BTreeMap<String, usize>), Error> {
    let res = field(doc, player(doc)?, "res")?;
    let types = array(doc, field(doc, res, "resType")?)?;
    let values = array(doc, field(doc, res, "resValues")?)?;
    if doc.records[types].children.len() != doc.records[values].children.len() {
        return Err(failure("Resource lists have different lengths"));
    }
    let mut mapping = BTreeMap::new();
    for (&t, &v) in doc.records[types]
        .children
        .iter()
        .zip(&doc.records[values].children)
    {
        let Payload::Text(name) = &doc.records[t].payload else {
            return Err(failure("Resource name is not a string"));
        };
        let name = name.display();
        let atom_type = field(doc, v, "type")?;
        if view(doc, atom_type).value.as_deref() != Some(&name) {
            return Err(failure("Resource lists disagree"));
        }
        let value = field(doc, v, "value")?;
        if base_tag(doc.records[value].tag) != 32 {
            return Err(failure("Resource value is not float32"));
        }
        if mapping.insert(name, value).is_some() {
            return Err(failure("Duplicate resource names"));
        }
    }
    Ok((types, values, mapping))
}
fn health(doc: &Document, name: &str) -> Result<usize, Error> {
    let id = field(
        doc,
        field(doc, player(doc)?, "hpComponent")?,
        if name == "hp" { "hp" } else { "maxHpValue" },
    )?;
    if base_tag(doc.records[id].tag) != 24 {
        return Err(failure("Health field is not int32"));
    }
    Ok(id)
}
fn text_value(doc: &Document, parent: usize, name: &str) -> Option<String> {
    field(doc, parent, name)
        .ok()
        .and_then(|id| view(doc, id).value)
}
fn equals(text: &WireString, value: &str) -> bool {
    if text.wide {
        text.bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .eq(value.encode_utf16())
    } else {
        text.bytes == value.as_bytes()
    }
}
/// Allocation-free `id` check for scanning thousands of live world objects.
fn has_id(doc: &Document, object: usize, id: &str) -> bool {
    doc.records[object].children.iter().any(|child| {
        let record = &doc.records[*child];
        record.name.as_ref().is_some_and(|n| equals(n, "id"))
            && matches!(&record.payload, Payload::Text(text) if equals(text, id))
    })
}
/// Returns the `inventoryItem` that holds a currency, using the same handle as inventory discovery.
fn item_container(doc: &Document, key: &str) -> Result<usize, Error> {
    if key == "faith" {
        return field(doc, field(doc, player(doc)?, "inventory")?, "inventoryItem");
    }
    let root = *doc.roots.first().ok_or_else(|| failure("Missing root"))?;
    let scenes = array(
        doc,
        field(doc, field(doc, root, "worldData")?, "gameSceneDataList")?,
    )?;
    let mut tables = vec![];
    for scene in &doc.records[scenes].children {
        let Ok(objects) = field(doc, *scene, "wgoDataList").and_then(|list| array(doc, list))
        else {
            continue;
        };
        for &entry in &doc.records[objects].children {
            // The list is live, so only references need resolving. `resolve` would walk
            // every entry's ancestors and sibling lists.
            let object = if base_tag(doc.records[entry].tag) == 10 {
                resolve(doc, entry)?
            } else {
                entry
            };
            if has_id(doc, object, STUDY_TABLE) {
                tables.push(field(
                    doc,
                    field(doc, object, "inventory")?,
                    "inventoryItem",
                )?);
            }
        }
    }
    match tables[..] {
        [table] => Ok(table),
        [] => Err(failure("No study table was found in this save")),
        _ => Err(failure(
            "This save has several study tables; edit science in the Inventory view",
        )),
    }
}
fn count(doc: &Document, item: usize) -> Result<(usize, i32), Error> {
    let node = field(doc, item, "count")?;
    if base_tag(doc.records[node].tag) != 24 {
        return Err(failure("Item count is not int32"));
    }
    let value = view(doc, node)
        .value
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| failure("Invalid item count"))?;
    Ok((node, value))
}
struct Stack {
    container: usize,
    entry: usize,
    count_node: usize,
    count: i32,
}
/// Matches `Item.GetTotalCountInInventory`: top-level stacks, then stacks inside bags.
fn stacks(doc: &Document, container: usize, id: &str) -> Result<Vec<Stack>, Error> {
    let mut result = vec![];
    let mut bags = vec![];
    for &entry in &doc.records[array(doc, field(doc, container, "inventory")?)?].children {
        let item = resolve(doc, entry)?;
        if text_value(doc, item, "id").as_deref() == Some(id) {
            let (count_node, count) = count(doc, item)?;
            result.push(Stack {
                container,
                entry,
                count_node,
                count,
            });
        } else if let Ok(contents) = field(doc, item, "inventory").and_then(|l| array(doc, l)) {
            if !doc.records[contents].children.is_empty() {
                bags.push((item, contents));
            }
        }
    }
    for (bag, contents) in bags {
        for &entry in &doc.records[contents].children {
            let item = resolve(doc, entry)?;
            if text_value(doc, item, "id").as_deref() == Some(id) {
                let (count_node, count) = count(doc, item)?;
                result.push(Stack {
                    container: bag,
                    entry,
                    count_node,
                    count,
                });
            }
        }
    }
    Ok(result)
}
fn read_item(doc: &Document, key: &str) -> Result<(Option<usize>, String), Error> {
    let stacks = stacks(doc, item_container(doc, key)?, key)?;
    let total: i64 = stacks.iter().map(|s| i64::from(s.count)).sum();
    Ok((stacks.first().map(|s| s.count_node), total.to_string()))
}
/// Inventory containers that a General edit of these values may change.
pub(crate) fn item_containers(doc: &Document, values: &BTreeMap<String, String>) -> Vec<usize> {
    let mut result = BTreeSet::new();
    for key in ITEMS.iter().filter(|key| values.contains_key(**key)) {
        if let Ok(container) = item_container(doc, key) {
            result.insert(container);
            if let Ok(stacks) = stacks(doc, container, key) {
                result.extend(stacks.iter().map(|s| s.container));
            }
        }
    }
    result.into_iter().collect()
}
/// Whether an edit of this inventory container can change a General currency value.
pub(crate) fn shows_items(doc: &Document, container: usize) -> bool {
    let mut cursor = Some(container);
    while let Some(id) = cursor {
        let record = &doc.records[id];
        if record
            .name
            .as_ref()
            .is_some_and(|n| n.display() == "playerData")
            || (matches!(record.payload, Payload::Node { .. }) && has_id(doc, id, STUDY_TABLE))
        {
            return true;
        }
        cursor = record.parent;
    }
    false
}
/// Sets a currency total. Existing stacks are filled in game order and emptied stacks are
/// removed; any remainder is added to new top-level stacks with the supplied fresh GUIDs.
fn write_item(
    doc: &mut Document,
    key: &str,
    value: &str,
    guids: &mut std::slice::Iter<'_, String>,
) -> Result<(), Error> {
    let total: i32 = value
        .parse()
        .ok()
        .filter(|v| *v >= 0)
        .ok_or_else(|| failure(&format!("Enter a whole nonnegative amount of {key}")))?;
    let container = item_container(doc, key)?;
    let mut remaining = total;
    let mut resized = BTreeSet::new();
    for stack in stacks(doc, container, key)? {
        // Preserve an existing oversized stack rather than splitting it.
        let amount = remaining.min(ITEM_STACK.max(stack.count));
        remaining -= amount;
        if amount == 0 {
            apply(doc, Operation::Remove { node: stack.entry })?;
            resized.insert(stack.container);
        } else if amount != stack.count {
            apply(
                doc,
                Operation::Set {
                    node: stack.count_node,
                    tag: doc.records[stack.count_node].tag,
                    value: amount.to_string(),
                },
            )?;
        }
    }
    if remaining > 0 {
        let contents = array(doc, field(doc, container, "inventory")?)?;
        let size = field(doc, container, "inventorySize")?;
        if base_tag(doc.records[size].tag) != 24 {
            return Err(failure("Inventory size is not int32"));
        }
        let capacity: i32 = view(doc, size)
            .value
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| failure("Invalid inventory size"))?;
        let needed = (remaining + ITEM_STACK - 1) / ITEM_STACK;
        let used = doc.records[contents].children.len() as i64;
        if used + i64::from(needed) > i64::from(capacity.max(0)) {
            return Err(failure(&format!(
                "Not enough free inventory slots for {key}: needs {needed} more"
            )));
        }
        let definition = Definition {
            stack: ITEM_STACK,
            size: "Small".into(),
            ..Definition::default()
        };
        while remaining > 0 {
            let amount = remaining.min(ITEM_STACK);
            let guid = guids
                .next()
                .ok_or_else(|| failure("A new item identity is required"))?;
            // GUID uniqueness is validated against the finished transaction.
            new_item(doc, contents, key, amount, guid, &definition, None, true)?;
            remaining -= amount;
        }
        resized.insert(container);
    }
    // The game caches occupied slots on each container. Never leave this cache stale.
    for container in resized {
        let length = doc.records[array(doc, field(doc, container, "inventory")?)?]
            .children
            .len();
        let node = field(doc, container, "inventoryFillSize")?;
        apply(
            doc,
            Operation::Set {
                node,
                tag: doc.records[node].tag,
                value: length.to_string(),
            },
        )?;
    }
    Ok(())
}
pub(crate) fn read(doc: &Document) -> Value {
    let resources = resources(doc);
    let fields:Vec<_>=["hp","max_hp"].into_iter().chain(RESOURCES.iter().copied()).map(|key|{
        let result=if key=="hp"||key=="max_hp"{health(doc,key).map(Some)}else{resources.as_ref().map(|(_,_,m)|m.get(key).copied()).map_err(Clone::clone)};
        match result {Ok(node)=>json!({"key":key,"node":node,"value":node.and_then(|id|view(doc,id).value).unwrap_or_else(||"0".into()),"error":null}),Err(e)=>json!({"key":key,"node":null,"value":null,"error":e.to_string()})}
    }).chain(ITEMS.iter().map(|key| match read_item(doc, key) {
        Ok((node, value)) => json!({"key":key,"node":node,"value":value,"error":null}),
        Err(e) => json!({"key":key,"node":null,"value":null,"error":e.to_string()}),
    })).collect();
    json!(fields)
}
/// Adds `amount` to a player resource such as `tech_red`, creating it when missing.
/// A negative amount fails instead of taking the resource below zero.
pub(crate) fn add(doc: &mut Document, key: &str, amount: f64) -> Result<(), Error> {
    if amount == 0.0 {
        return Ok(());
    }
    let current = match resources(doc)?.2.get(key) {
        Some(node) => view(doc, *node)
            .value
            .unwrap_or_default()
            .parse::<f64>()
            .map_err(|_| failure("Invalid resource value"))?,
        None => 0.0,
    };
    if current + amount < 0.0 {
        let name = key.strip_prefix("tech_").map_or_else(
            || key.replace('_', " "),
            |color| format!("{color} technology points"),
        );
        return Err(failure(&format!(
            "Not enough {name}: needs {}, has {current}",
            -amount
        )));
    }
    write(
        doc,
        BTreeMap::from([(key.to_string(), (current + amount).to_string())]),
        &[],
    )
}
/// `guids` supplies fresh identities for currency stacks that have to be created.
pub(crate) fn write(
    doc: &mut Document,
    values: BTreeMap<String, String>,
    guids: &[String],
) -> Result<(), Error> {
    let mut guids = guids.iter();
    for (key, value) in values {
        if ITEMS.contains(&key.as_str()) {
            write_item(doc, &key, &value, &mut guids)?;
            continue;
        }
        let number: f64 = value
            .parse()
            .map_err(|_| failure("Enter a finite nonnegative number"))?;
        if !number.is_finite() || number < 0.0 || (key == "max_hp" && number <= 0.0) {
            return Err(failure(
                "Enter a finite nonnegative value; maximum health must be positive",
            ));
        }
        let node = if key == "hp" || key == "max_hp" {
            health(doc, &key)?
        } else {
            if !RESOURCES.contains(&key.as_str()) {
                return Err(failure("Unknown General field"));
            }
            let (types, atoms, mapping) = resources(doc)?;
            if let Some(id) = mapping.get(&key) {
                *id
            } else {
                let index = doc.records[atoms].children.len();
                let atom = create_template(
                    doc,
                    atoms,
                    index,
                    None,
                    "game-res-atom",
                    BTreeMap::from([("type".into(), key.clone())]),
                )?;
                apply(
                    doc,
                    Operation::Insert {
                        parent: types,
                        index,
                        name: None,
                        kind: "string".into(),
                        value: key,
                    },
                )?;
                field(doc, atom, "value")?
            }
        };
        apply(
            doc,
            Operation::Set {
                node,
                tag: doc.records[node].tag,
                value,
            },
        )?;
    }
    Ok(())
}
