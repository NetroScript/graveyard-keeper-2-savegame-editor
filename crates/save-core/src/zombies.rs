//! Zombie discovery and atomic body, appearance, equipment and talent edits.
use crate::inventory;
use crate::service::view;
use crate::wire::{base_tag, Payload, TypeInfo};
use crate::workspace::{apply, create_template, failure, field, resolve, Operation};
use crate::{Document, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    #[serde(default)]
    pub talents: BTreeMap<String, TalentNode>,
    #[serde(default)]
    pub appearances: BTreeMap<String, AppearanceSet>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TalentNode {
    pub talent: String,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub lock_type: String,
    #[serde(default)]
    pub available_at_start: bool,
    #[serde(default)]
    pub talent_value: i32,
    #[serde(default)]
    pub red: i32,
    #[serde(default)]
    pub green: i32,
    #[serde(default)]
    pub blue: i32,
    #[serde(default)]
    pub perk: Option<Perk>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Perk {
    pub id: String,
    #[serde(default)]
    pub resources_on_add: Vec<ResourceValue>,
    #[serde(default)]
    pub resources_on_remove: Vec<ResourceValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceValue {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: f32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSet {
    #[serde(default)]
    pub body_ids: Vec<i32>,
    #[serde(default)]
    pub head_ids: Vec<i32>,
    #[serde(default)]
    pub body_luts: Vec<String>,
    #[serde(default)]
    pub head_luts: Vec<String>,
}

impl Catalog {
    pub fn validate(&self) -> Result<(), Error> {
        if self.talents.len() > 10_000 || self.appearances.len() > 100 {
            return Err(failure("Invalid zombie catalog"));
        }
        for (id, node) in &self.talents {
            if id.is_empty()
                || id.len() > 1024
                || node.talent.is_empty()
                || node.parents.len() > 100
                || node.red < 0
                || node.green < 0
                || node.blue < 0
                || node
                    .perk
                    .as_ref()
                    .is_some_and(|perk| perk.id.is_empty() || perk.id.len() > 1024)
            {
                return Err(failure("Invalid zombie talent catalog"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Rename {
        name: String,
    },
    SetPoints {
        red: String,
        green: String,
        blue: String,
    },
    SetAppearance {
        set: String,
        body: i32,
        head: i32,
        #[serde(default)]
        body_lut: String,
        #[serde(default)]
        head_lut: String,
    },
    BodyInventory {
        action: inventory::Edit,
        #[serde(default)]
        out_of_bounds: bool,
    },
    CargoInventory {
        action: inventory::Edit,
        #[serde(default)]
        out_of_bounds: bool,
    },
    Equip {
        slot: String,
        #[serde(default)]
        node: Option<usize>,
        #[serde(default)]
        item: Option<String>,
    },
    SetTalents {
        ids: Vec<String>,
        #[serde(default)]
        grant_points: bool,
    },
}

fn short_type(doc: &Document, node: usize) -> String {
    let name = match &doc.records[node].payload {
        Payload::Node {
            ty: TypeInfo::Definition(_, name),
            ..
        } => name.display(),
        Payload::Node {
            ty: TypeInfo::Reference(id),
            ..
        } => doc.types.get(id).cloned().unwrap_or_default(),
        _ => String::new(),
    };
    name.split(',').next().unwrap_or("").trim().to_owned()
}

fn value(doc: &Document, parent: usize, name: &str) -> Result<String, Error> {
    view(doc, field(doc, parent, name)?)
        .value
        .ok_or_else(|| failure(&format!("Missing zombie field: {name}")))
}

fn set(doc: &mut Document, parent: usize, name: &str, value: String) -> Result<(), Error> {
    let node = field(doc, parent, name)?;
    apply(
        doc,
        Operation::Set {
            node,
            tag: doc.records[node].tag,
            value,
        },
    )
}

fn integer(doc: &Document, parent: usize, name: &str) -> Result<i32, Error> {
    value(doc, parent, name)?
        .parse()
        .map_err(|_| failure(&format!("Invalid zombie integer: {name}")))
}

fn array(doc: &Document, owner: usize, name: &str) -> Result<usize, Error> {
    let list = resolve(doc, field(doc, owner, name)?)?;
    let arrays: Vec<_> = doc.records[list]
        .children
        .iter()
        .copied()
        .filter(|id| base_tag(doc.records[*id].tag) == 6)
        .collect();
    if arrays.len() != 1 {
        return Err(failure(&format!("Unsupported zombie list: {name}")));
    }
    Ok(arrays[0])
}

fn text_value(doc: &Document, node: usize) -> Result<String, Error> {
    match &doc.records[node].payload {
        Payload::Text(value) => Ok(value.display()),
        _ => Err(failure("Expected zombie string")),
    }
}

fn text_list(doc: &Document, owner: usize, name: &str) -> Result<Vec<String>, Error> {
    doc.records[array(doc, owner, name)?]
        .children
        .iter()
        .map(|id| {
            text_value(doc, *id).map_err(|_| {
                failure(&format!(
                    "Expected a string in zombie list {name} (node {id}, tag {})",
                    doc.records[*id].tag
                ))
            })
        })
        .collect()
}

fn replace_text_list(
    doc: &mut Document,
    owner: usize,
    name: &str,
    values: &[String],
) -> Result<(), Error> {
    let target = array(doc, owner, name)?;
    for child in doc.records[target].children.clone().into_iter().rev() {
        apply(doc, Operation::Remove { node: child })?;
    }
    for value in values {
        apply(
            doc,
            Operation::Insert {
                parent: target,
                index: doc.records[target].children.len(),
                name: None,
                kind: "string".into(),
                value: value.clone(),
            },
        )?;
    }
    Ok(())
}

pub(crate) fn is_zombie(doc: &Document, node: usize) -> bool {
    short_type(doc, node) == "ZombieWgoData"
}

pub(crate) fn zombies(doc: &Document) -> Result<Vec<usize>, Error> {
    let mut result = vec![];
    // Concrete Odin objects occur once in the record table. Searching that table avoids
    // rebuilding the reachable set for the whole save just to find a handful of zombies.
    for node in 0..doc.records.len() {
        if is_zombie(doc, node) && crate::workspace::active(doc, node).is_ok() {
            result.push(node);
        }
    }
    Ok(result)
}

fn zombie(doc: &Document, handle: usize) -> Result<usize, Error> {
    let handle = resolve(doc, handle)?;
    if short_type(doc, handle) != "ZombieWgoData" {
        return Err(failure("Unknown zombie"));
    }
    Ok(handle)
}

fn body(doc: &Document, zombie: usize) -> Result<usize, Error> {
    resolve(doc, field(doc, zombie, "zombieItem")?)
}

fn body_contents(doc: &Document, zombie: usize) -> Result<usize, Error> {
    array(doc, body(doc, zombie)?, "inventory")
}

fn porter_container(doc: &Document, zombie: usize) -> Option<usize> {
    let inventory = resolve(doc, field(doc, zombie, "porterInventory").ok()?).ok()?;
    resolve(doc, field(doc, inventory, "inventoryItem").ok()?).ok()
}

fn guid(doc: &Document, owner: usize) -> Result<String, Error> {
    value(doc, resolve(doc, field(doc, owner, "uniqueId")?)?, "id")
}

fn guid_field(doc: &Document, owner: usize, name: &str) -> Result<String, Error> {
    value(doc, resolve(doc, field(doc, owner, name)?)?, "id")
}

fn set_guid_field(
    doc: &mut Document,
    owner: usize,
    name: &str,
    value: String,
) -> Result<(), Error> {
    let guid = resolve(doc, field(doc, owner, name)?)?;
    set(doc, guid, "id", value)
}

fn is_empty_guid(value: &str) -> bool {
    value.is_empty() || value.eq_ignore_ascii_case("00000000-0000-0000-0000-000000000000")
}

fn clear_missing_equipment(doc: &mut Document, zombie: usize) -> Result<(), Error> {
    let available: HashSet<_> = doc.records[body_contents(doc, zombie)?]
        .children
        .iter()
        .filter_map(|entry| resolve(doc, *entry).ok())
        .filter_map(|item| guid(doc, item).ok())
        .collect();
    for field_name in ["equippedHand", "equippedArmor", "equippedCollar"] {
        let current = guid_field(doc, zombie, field_name)?;
        if !is_empty_guid(&current) && !available.contains(&current) {
            set_guid_field(
                doc,
                zombie,
                field_name,
                "00000000-0000-0000-0000-000000000000".into(),
            )?;
        }
    }
    Ok(())
}

fn game_res(
    doc: &Document,
    owner: usize,
) -> Result<(usize, usize, BTreeMap<String, usize>), Error> {
    let res = resolve(doc, field(doc, owner, "gameRes")?)?;
    let types = array(doc, res, "resType")?;
    let values = array(doc, res, "resValues")?;
    if doc.records[types].children.len() != doc.records[values].children.len() {
        return Err(failure("Zombie resource lists have different lengths"));
    }
    let mut mapping = BTreeMap::new();
    for (&kind, &atom) in doc.records[types]
        .children
        .iter()
        .zip(&doc.records[values].children)
    {
        let kind = text_value(doc, kind)
            .map_err(|_| failure("Zombie numeric resource key is not a string"))?;
        let atom = resolve(doc, atom)?;
        let atom_kind = value(doc, atom, "type")?;
        if kind != atom_kind {
            return Err(failure("Zombie resource lists disagree"));
        }
        let value_node = field(doc, atom, "value")?;
        if mapping.insert(kind, value_node).is_some() {
            return Err(failure("Duplicate zombie resource"));
        }
    }
    Ok((types, values, mapping))
}

fn read_float_resource(doc: &Document, owner: usize, name: &str) -> Result<f32, Error> {
    let (_, _, resources) = game_res(doc, owner)?;
    match resources.get(name) {
        Some(node) => view(doc, *node)
            .value
            .ok_or_else(|| failure("Missing zombie resource value"))?
            .parse()
            .map_err(|_| failure("Invalid zombie resource value")),
        None => Ok(0.0),
    }
}

fn set_float_resource(
    doc: &mut Document,
    owner: usize,
    name: &str,
    number: f32,
) -> Result<(), Error> {
    if name.is_empty() || name.len() > 1024 || !number.is_finite() {
        return Err(failure("Invalid zombie resource"));
    }
    let (types, values, resources) = game_res(doc, owner)?;
    let target = if let Some(node) = resources.get(name) {
        *node
    } else {
        let index = doc.records[values].children.len();
        let atom = create_template(
            doc,
            values,
            index,
            None,
            "game-res-atom",
            BTreeMap::from([("type".into(), name.to_owned())]),
        )?;
        apply(
            doc,
            Operation::Insert {
                parent: types,
                index,
                name: None,
                kind: "string".into(),
                value: name.to_owned(),
            },
        )?;
        field(doc, atom, "value")?
    };
    apply(
        doc,
        Operation::Set {
            node: target,
            tag: doc.records[target].tag,
            value: number.to_string(),
        },
    )
}

fn game_res_str(doc: &Document, owner: usize) -> Result<(usize, usize), Error> {
    let res = resolve(doc, field(doc, owner, "gameResStr")?)?;
    let keys = array(doc, res, "keys")?;
    let values = array(doc, res, "values")?;
    if doc.records[keys].children.len() != doc.records[values].children.len() {
        return Err(failure(
            "Zombie string resource lists have different lengths",
        ));
    }
    Ok((keys, values))
}

fn read_string_resource(doc: &Document, owner: usize, name: &str) -> Result<String, Error> {
    let (keys, values) = game_res_str(doc, owner)?;
    for (&key, &value) in doc.records[keys]
        .children
        .iter()
        .zip(&doc.records[values].children)
    {
        if text_value(doc, key)
            .map_err(|_| failure("Zombie appearance resource key is not a string"))?
            == name
        {
            return if base_tag(doc.records[value].tag) == 46 {
                Ok(String::new())
            } else {
                text_value(doc, value)
                    .map_err(|_| failure("Zombie appearance resource value is not a string"))
            };
        }
    }
    Ok(String::new())
}

fn set_string_resource(
    doc: &mut Document,
    owner: usize,
    name: &str,
    value: String,
) -> Result<(), Error> {
    if name.is_empty() || name.len() > 1024 || value.len() > 1024 {
        return Err(failure("Invalid zombie appearance value"));
    }
    let (keys, values) = game_res_str(doc, owner)?;
    for index in 0..doc.records[keys].children.len() {
        if text_value(doc, doc.records[keys].children[index])? == name {
            let node = doc.records[values].children[index];
            if base_tag(doc.records[node].tag) == 46 {
                apply(doc, Operation::Remove { node })?;
                return apply(
                    doc,
                    Operation::Insert {
                        parent: values,
                        index,
                        name: None,
                        kind: "string".into(),
                        value,
                    },
                );
            }
            return apply(
                doc,
                Operation::Set {
                    node,
                    tag: doc.records[node].tag,
                    value,
                },
            );
        }
    }
    let index = doc.records[keys].children.len();
    apply(
        doc,
        Operation::Insert {
            parent: keys,
            index,
            name: None,
            kind: "string".into(),
            value: name.to_owned(),
        },
    )?;
    apply(
        doc,
        Operation::Insert {
            parent: values,
            index,
            name: None,
            kind: "string".into(),
            value,
        },
    )
}

fn talents(doc: &Document, zombie: usize) -> Result<Vec<usize>, Error> {
    doc.records[array(doc, zombie, "talentData")?]
        .children
        .iter()
        .map(|node| resolve(doc, *node))
        .collect()
}

fn active_perks(doc: &Document, zombie: usize) -> Result<Vec<(usize, String)>, Error> {
    let array = array(doc, zombie, "activePerks")?;
    doc.records[array]
        .children
        .iter()
        .map(|entry| {
            let node = resolve(doc, *entry)?;
            Ok((*entry, value(doc, node, "id")?))
        })
        .collect()
}

fn body_items(doc: &Document, zombie: usize) -> Result<Vec<Value>, Error> {
    let equipped = HashMap::from([
        (guid_field(doc, zombie, "equippedHand")?, "hand"),
        (guid_field(doc, zombie, "equippedArmor")?, "armor"),
        (guid_field(doc, zombie, "equippedCollar")?, "collar"),
    ]);
    doc.records[body_contents(doc, zombie)?]
        .children
        .iter()
        .map(|entry| {
            let item = resolve(doc, *entry)?;
            let item_guid = guid(doc, item)?;
            Ok(json!({
                "node": entry,
                "id": value(doc, item, "id")?,
                "count": value(doc, item, "count")?,
                "guid": item_guid,
                "equipped": equipped.get(&item_guid).copied()
            }))
        })
        .collect()
}

fn inventory_items(doc: &Document, container: usize) -> Result<Vec<Value>, Error> {
    doc.records[array(doc, container, "inventory")?]
        .children
        .iter()
        .map(|entry| {
            let item = resolve(doc, *entry)?;
            Ok(json!({
                "node": entry,
                "id": value(doc, item, "id")?,
                "count": value(doc, item, "count")?,
                "guid": guid(doc, item).unwrap_or_default(),
                "equipped": Value::Null
            }))
        })
        .collect()
}

fn carried_item(doc: &Document, zombie: usize, role: i32) -> Option<Value> {
    let (field_name, label) = match role {
        2 => ("caretakerPortableItem", "Carried delivery"),
        6 => ("gardenerPortableItem", "Carried gardening item"),
        7 => ("conveyorTransporterPortableItem", "Carried delivery"),
        _ => return None,
    };
    let item = resolve(doc, field(doc, zombie, field_name).ok()?).ok()?;
    let id = value(doc, item, "id").ok()?;
    if id.is_empty() || id == "empty" {
        return None;
    }
    Some(json!({
        "label": label,
        "item": {
            "node": field(doc, zombie, field_name).ok()?,
            "id": id,
            "count": value(doc, item, "count").ok()?,
            "guid": guid(doc, item).unwrap_or_default(),
            "equipped": Value::Null
        }
    }))
}

fn role(value: i32) -> &'static str {
    match value {
        0 => "Free",
        1 => "Crafter",
        2 => "Caretaker",
        3 => "Conveyor crafter",
        4 => "Worker",
        5 => "Porter",
        6 => "Gardener",
        7 => "Conveyor transporter",
        8 => "Fighter",
        _ => "Unknown",
    }
}

fn appearance_set_for_id(id: &str) -> &'static str {
    if id == "zombie_assistant" {
        "zombie_assistant"
    } else {
        "zombie_worker"
    }
}

fn read_one(
    doc: &Document,
    items: &inventory::Catalog,
    _catalog: &Catalog,
    zombie: usize,
) -> Result<Value, Error> {
    let entries = body_items(doc, zombie)?;
    let mut red = 0i32;
    let mut white = 0i32;
    for entry in &entries {
        let count = entry["count"]
            .as_str()
            .unwrap_or("0")
            .parse::<i32>()
            .unwrap_or(0);
        if let Some(definition) = entry["id"].as_str().and_then(|id| items.items.get(id)) {
            red = red.saturating_add(definition.red_skulls.saturating_mul(count));
            white = white.saturating_add(definition.white_skulls.saturating_mul(count));
        }
    }
    let mut branches = vec![];
    let mut used = 0usize;
    for branch in talents(doc, zombie)? {
        let studied = text_list(doc, branch, "studiedLevelUps")?;
        used += studied.len();
        branches.push(json!({
            "id": value(doc, branch, "id")?,
            "value": value(doc, branch, "curTalentValue")?,
            "studied": studied
        }));
    }
    let type_value = integer(doc, zombie, "zombieType")?;
    let body_id = read_float_resource(doc, zombie, "zombie_body_id")? as i32;
    let head_id = read_float_resource(doc, zombie, "zombie_head_id")? as i32;
    let zombie_id = value(doc, zombie, "id")?;
    // Assistants receive body 1002 in CreateAssistantFromThis. Other saved
    // workers use the ordinary worker body family. The game still resolves LUT
    // names through zombie_worker in WgoPart.SetupZombieSkin.
    let appearance_set = appearance_set_for_id(&zombie_id);
    let attached = guid_field(doc, zombie, "attachedWgoDataUniqueId").unwrap_or_default();
    let cargo = porter_container(doc, zombie)
        .map(|container| {
            Ok::<_, Error>(json!({
                "node": container,
                "capacity": value(doc, container, "inventorySize")?,
                "items": inventory_items(doc, container)?
            }))
        })
        .transpose()?;
    Ok(json!({
            "node": zombie,
            "id": zombie_id,
            "guid": guid(doc, zombie)?,
            "name": value(doc, zombie, "name")?,
            "role": role(type_value),
            "roleValue": type_value,
            "world": value(doc, zombie, "worldId").unwrap_or_default(),
            "attachedGuid": (!is_empty_guid(&attached)).then_some(attached),
            "body": body(doc, zombie)?,
            "bodyCapacity": value(doc, body(doc, zombie)?, "inventorySize")?,
            "items": entries,
            "cargo": cargo,
            "carriedItem": carried_item(doc, zombie, type_value),
            "redSkulls": red.clamp(0, 999).to_string(),
            "whiteSkulls": white.clamp(0, 999).to_string(),
            "usedTalentSlots": used,
            "points": {
                "red": value(doc, zombie, "techRed")?,
                "green": value(doc, zombie, "techGreen")?,
                "blue": value(doc, zombie, "techBlue")?
            },
            "appearance": {
                "set": appearance_set,
                "body": body_id,
                "head": head_id,
                "bodyLut": read_string_resource(doc, zombie, "zombie_body_lut")?,
                "headLut": read_string_resource(doc, zombie, "zombie_head_lut")?
            },
            "talents": branches,
            "disabledTalents": text_list(doc, zombie, "disabledTalentLevelUps")?,
            "activePerks": active_perks(doc, zombie)?.into_iter().map(|(_, id)| id).collect::<Vec<_>>()
    }))
}

#[derive(Default)]
pub(crate) struct Cache {
    entries: BTreeMap<usize, Value>,
}

impl Cache {
    pub(crate) fn build_for(
        doc: &Document,
        items: &inventory::Catalog,
        catalog: &Catalog,
        handles: impl IntoIterator<Item = usize>,
    ) -> Result<Self, Error> {
        let mut cache = Self::default();
        cache.refresh(doc, items, catalog, handles)?;
        Ok(cache)
    }

    pub(crate) fn refresh(
        &mut self,
        doc: &Document,
        items: &inventory::Catalog,
        catalog: &Catalog,
        handles: impl IntoIterator<Item = usize>,
    ) -> Result<(), Error> {
        for handle in handles {
            match zombie(doc, handle) {
                Ok(handle) => {
                    self.entries
                        .insert(handle, read_one(doc, items, catalog, handle)?);
                }
                Err(_) => {
                    self.entries.remove(&handle);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Value {
        json!({"zombies": self.entries.values().cloned().collect::<Vec<_>>()})
    }
}

pub(crate) fn validate(
    doc: &Document,
    items: &inventory::Catalog,
    handles: impl IntoIterator<Item = usize>,
) -> Result<(), Error> {
    let mut seen = HashSet::new();
    for handle in handles {
        let zombie = zombie(doc, handle)?;
        if !seen.insert(zombie) {
            continue;
        }
        let mut organs = HashSet::new();
        let equipped_guids: HashSet<_> = [
            guid_field(doc, zombie, "equippedHand")?,
            guid_field(doc, zombie, "equippedArmor")?,
            guid_field(doc, zombie, "equippedCollar")?,
        ]
        .into_iter()
        .filter(|guid| !is_empty_guid(guid))
        .collect();
        let mut pocket_slots = 0usize;
        let mut red = 0i32;
        let mut item_by_guid = HashMap::new();
        for entry in &doc.records[body_contents(doc, zombie)?].children {
            let item = resolve(doc, *entry)?;
            let id = value(doc, item, "id")?;
            let count = integer(doc, item, "count")?;
            if count < 1 {
                return Err(failure("Zombie body items must have a positive amount"));
            }
            let item_guid = guid(doc, item)?;
            item_by_guid.insert(item_guid.clone(), id.clone());
            let Some(definition) = items.items.get(&id) else {
                continue;
            };
            red = red.saturating_add(definition.red_skulls.saturating_mul(count));
            if definition.is_organ_mistake {
                return Err(failure("Zombie contains an invalid organ variant"));
            }
            if definition.is_main_organ
                && (!organs.insert(definition.item_type.clone()) || count != 1)
            {
                return Err(failure("Zombie has duplicate main organs"));
            }
            if !definition.is_main_organ
                && !definition
                    .groups
                    .iter()
                    .any(|group| group == "burial_reward")
                && !equipped_guids.contains(&item_guid)
            {
                pocket_slots = pocket_slots.saturating_add(1);
            }
        }
        if pocket_slots > 6 {
            return Err(failure("Zombie body has more than six pocket items"));
        }
        let used: usize = talents(doc, zombie)?
            .into_iter()
            .map(|branch| text_list(doc, branch, "studiedLevelUps").map(|ids| ids.len()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        if used > red.clamp(0, 999) as usize {
            return Err(failure("Zombie talents exceed available red-skull slots"));
        }
        for (field_name, expected) in [
            ("equippedHand", "hand"),
            ("equippedArmor", "armor"),
            ("equippedCollar", "collar"),
        ] {
            let equipped = guid_field(doc, zombie, field_name)?;
            if is_empty_guid(&equipped) {
                continue;
            }
            let id = item_by_guid.get(&equipped).ok_or_else(|| {
                failure("Zombie equipment reference is not in its body inventory")
            })?;
            let Some(definition) = items.items.get(id) else {
                continue;
            };
            let valid = match expected {
                "armor" => definition.item_type == "BodyArmor",
                "collar" => definition.item_type == "Collar",
                _ => definition.item_type != "BodyArmor" && definition.item_type != "Collar",
            };
            if !valid {
                return Err(failure("Zombie equipment is in the wrong slot"));
            }
            if expected == "collar"
                && definition.red_skulls_max_collar > definition.red_skulls_min_collar
                && (red < definition.red_skulls_min_collar
                    || red > definition.red_skulls_max_collar)
            {
                return Err(failure(
                    "The equipped collar does not accept this red-skull total",
                ));
            }
        }
    }
    Ok(())
}

fn checked_point(value: String) -> Result<i32, Error> {
    let value: i32 = value
        .parse()
        .map_err(|_| failure("Zombie points must be 32-bit integers"))?;
    if value < 0 {
        return Err(failure("Zombie points cannot be negative"));
    }
    Ok(value)
}

fn validate_dependencies(catalog: &Catalog, selected: &HashSet<String>) -> Result<(), Error> {
    if catalog
        .talents
        .iter()
        .any(|(id, node)| node.available_at_start && !selected.contains(id))
    {
        return Err(failure("Starting zombie talents cannot be removed"));
    }
    for id in selected {
        let node = catalog
            .talents
            .get(id)
            .ok_or_else(|| failure("Unknown zombie talent"))?;
        let locked = if node.lock_type.eq_ignore_ascii_case("Any") {
            !node.parents.is_empty() && !node.parents.iter().any(|parent| selected.contains(parent))
        } else {
            node.parents.iter().any(|parent| !selected.contains(parent))
        };
        if locked {
            return Err(failure("Zombie talent dependencies are incomplete"));
        }
    }
    Ok(())
}

fn write_talents(
    doc: &mut Document,
    zombie: usize,
    catalog: &Catalog,
    ids: Vec<String>,
    grant_points: bool,
) -> Result<(), Error> {
    if ids.len() > catalog.talents.len() {
        return Err(failure("Too many zombie talents"));
    }
    let selected: HashSet<_> = ids.into_iter().collect();
    validate_dependencies(catalog, &selected)?;
    let red_skulls = body_items(doc, zombie)?.iter().fold(0i32, |total, entry| {
        total.saturating_add(
            entry["count"]
                .as_str()
                .unwrap_or("0")
                .parse::<i32>()
                .unwrap_or(0),
        )
    });
    // The definitive skull validation is performed below from the inventory catalog by the caller.
    let _ = red_skulls;
    let existing: HashSet<_> = talents(doc, zombie)?
        .into_iter()
        .flat_map(|branch| text_list(doc, branch, "studiedLevelUps").unwrap_or_default())
        .collect();
    let mut costs = [0i32; 3];
    for id in selected.difference(&existing) {
        let node = &catalog.talents[id];
        costs[0] = costs[0]
            .checked_add(node.red)
            .ok_or_else(|| failure("Point cost overflow"))?;
        costs[1] = costs[1]
            .checked_add(node.green)
            .ok_or_else(|| failure("Point cost overflow"))?;
        costs[2] = costs[2]
            .checked_add(node.blue)
            .ok_or_else(|| failure("Point cost overflow"))?;
    }
    if !grant_points {
        for (field_name, cost) in [
            ("techRed", costs[0]),
            ("techGreen", costs[1]),
            ("techBlue", costs[2]),
        ] {
            let current = integer(doc, zombie, field_name)?;
            if current < cost {
                return Err(failure("Not enough zombie technology points"));
            }
            set(doc, zombie, field_name, (current - cost).to_string())?;
        }
    }
    for branch in talents(doc, zombie)? {
        let talent = value(doc, branch, "id")?;
        let mut branch_ids: Vec<_> = selected
            .iter()
            .filter(|id| {
                catalog
                    .talents
                    .get(*id)
                    .is_some_and(|node| node.talent == talent)
            })
            .cloned()
            .collect();
        branch_ids.sort_by_key(|id| {
            catalog
                .talents
                .get(id)
                .map(|node| (node.parents.len(), id.clone()))
                .unwrap_or_default()
        });
        let total = branch_ids.iter().try_fold(0i32, |sum, id| {
            sum.checked_add(catalog.talents[id].talent_value)
                .ok_or_else(|| failure("Zombie talent value overflow"))
        })?;
        replace_text_list(doc, branch, "studiedLevelUps", &branch_ids)?;
        set(doc, branch, "curTalentValue", total.to_string())?;
    }
    replace_text_list(doc, zombie, "disabledTalentLevelUps", &[])?;

    let zombie_perks: HashSet<_> = catalog
        .talents
        .values()
        .filter_map(|node| node.perk.as_ref().map(|perk| perk.id.clone()))
        .collect();
    let desired: HashSet<_> = selected
        .iter()
        .filter_map(|id| {
            catalog.talents[id]
                .perk
                .as_ref()
                .map(|perk| perk.id.clone())
        })
        .collect();
    let perk_array = array(doc, zombie, "activePerks")?;
    for (entry, id) in active_perks(doc, zombie)?.into_iter().rev() {
        if zombie_perks.contains(&id) && !desired.contains(&id) {
            apply(doc, Operation::Remove { node: entry })?;
        }
    }
    let retained: HashSet<_> = active_perks(doc, zombie)?
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    for id in desired.difference(&retained) {
        create_template(
            doc,
            perk_array,
            doc.records[perk_array].children.len(),
            None,
            "perk-data",
            BTreeMap::from([
                ("id".into(), id.clone()),
                ("currentDuration".into(), "0".into()),
            ]),
        )?;
    }
    for node in catalog.talents.values() {
        let Some(perk) = &node.perk else { continue };
        let effects = if desired.contains(&perk.id) {
            &perk.resources_on_add
        } else if existing.iter().any(|id| {
            catalog
                .talents
                .get(id)
                .and_then(|n| n.perk.as_ref())
                .is_some_and(|p| p.id == perk.id)
        }) {
            &perk.resources_on_remove
        } else {
            continue;
        };
        for effect in effects {
            set_float_resource(doc, zombie, &effect.kind, effect.value)?;
        }
    }
    Ok(())
}

pub(crate) fn write(
    doc: &mut Document,
    items: &inventory::Catalog,
    catalog: &Catalog,
    handle: usize,
    edit: Edit,
) -> Result<(), Error> {
    let zombie = zombie(doc, handle)?;
    match edit {
        Edit::Rename { name } => {
            let name = name.trim().to_owned();
            if name.is_empty() || name.chars().count() > 80 {
                return Err(failure("Zombie names must contain 1..80 characters"));
            }
            set(doc, zombie, "name", name)?;
            set(doc, zombie, "nameRandomed", "false".into())?;
        }
        Edit::SetPoints { red, green, blue } => {
            set(doc, zombie, "techRed", checked_point(red)?.to_string())?;
            set(doc, zombie, "techGreen", checked_point(green)?.to_string())?;
            set(doc, zombie, "techBlue", checked_point(blue)?.to_string())?;
        }
        Edit::SetAppearance {
            set: set_id,
            body,
            head,
            body_lut,
            head_lut,
        } => {
            let expected_set = appearance_set_for_id(&value(doc, zombie, "id")?);
            if set_id != expected_set {
                return Err(failure(
                    "This worker type does not support the selected appearance set",
                ));
            }
            let appearance = catalog
                .appearances
                .get(&set_id)
                .ok_or_else(|| failure("Unknown zombie appearance set"))?;
            if !appearance.body_ids.contains(&body) || !appearance.head_ids.contains(&head) {
                return Err(failure("Invalid zombie body or head"));
            }
            let runtime_luts = catalog
                .appearances
                .get("zombie_worker")
                .ok_or_else(|| failure("Missing zombie worker appearance set"))?;
            if (!body_lut.is_empty() && !runtime_luts.body_luts.contains(&body_lut))
                || (!head_lut.is_empty() && !runtime_luts.head_luts.contains(&head_lut))
            {
                return Err(failure("Invalid zombie color palette"));
            }
            set_float_resource(doc, zombie, "zombie_body_id", body as f32)?;
            set_float_resource(doc, zombie, "zombie_head_id", head as f32)?;
            set_string_resource(doc, zombie, "zombie_body_lut", body_lut)?;
            set_string_resource(doc, zombie, "zombie_head_lut", head_lut)?;
        }
        Edit::BodyInventory {
            action,
            out_of_bounds,
        } => {
            if let inventory::Edit::Put {
                item, count, node, ..
            } = &action
            {
                let definition = items
                    .items
                    .get(item)
                    .ok_or_else(|| failure("Item definition unavailable"))?;
                if definition.size != "Small" || definition.is_bag {
                    return Err(failure("Only small, non-bag items belong in a zombie body"));
                }
                if definition.is_organ_mistake {
                    return Err(failure("Invalid organ variants cannot be installed"));
                }
                if definition.is_main_organ {
                    let count: i32 = count.parse().map_err(|_| failure("Invalid organ amount"))?;
                    if count != 1 {
                        return Err(failure("A zombie can have only one of each main organ"));
                    }
                    for entry in &doc.records[body_contents(doc, zombie)?].children {
                        if Some(*entry) == *node {
                            continue;
                        }
                        let other = resolve(doc, *entry)?;
                        let other_id = value(doc, other, "id")?;
                        if items.items.get(&other_id).is_some_and(|candidate| {
                            candidate.is_main_organ && candidate.item_type == definition.item_type
                        }) {
                            return Err(failure("This zombie already has that main organ"));
                        }
                    }
                }
            }
            inventory::write_inner(
                doc,
                items,
                body(doc, zombie)?,
                action,
                out_of_bounds,
                true,
                false,
            )?;
            clear_missing_equipment(doc, zombie)?;
        }
        Edit::CargoInventory {
            action,
            out_of_bounds,
        } => {
            if integer(doc, zombie, "zombieType")? != 5 {
                return Err(failure("Only porters have an editable cargo inventory"));
            }
            let container = porter_container(doc, zombie)
                .ok_or_else(|| failure("This porter does not have a cargo inventory"))?;
            let target = match &action {
                inventory::Edit::Put { node, item, .. } => {
                    let definition = items
                        .items
                        .get(item)
                        .ok_or_else(|| failure("Item definition unavailable"))?;
                    if definition.size != "Small" || definition.is_bag {
                        return Err(failure(
                            "Only small, non-bag items can be edited in porter cargo",
                        ));
                    }
                    *node
                }
                inventory::Edit::Remove { node } => Some(*node),
                inventory::Edit::Capacity { .. } => {
                    return Err(failure("Porter cargo capacity is fixed by the game"));
                }
            };
            if let Some(target) = target {
                let contents = array(doc, container, "inventory")?;
                if !doc.records[contents].children.contains(&target) {
                    return Err(failure("Item is not in this porter's cargo"));
                }
                let current = resolve(doc, target)?;
                let id = value(doc, current, "id")?;
                if id == "fake_porter_slot_filler"
                    || items.items.get(&id).is_some_and(|item| item.size == "Big")
                {
                    return Err(failure(
                        "Large porter cargo is managed together with a reserved slot",
                    ));
                }
            }
            inventory::write_inner(doc, items, container, action, out_of_bounds, true, false)?;
        }
        Edit::Equip { slot, node, item } => {
            let field_name = match slot.as_str() {
                "hand" => "equippedHand",
                "armor" => "equippedArmor",
                "collar" => "equippedCollar",
                _ => return Err(failure("Unknown zombie equipment slot")),
            };
            let node = if node.is_some() && item.is_some() {
                return Err(failure("Choose equipment by node or item ID, not both"));
            } else if let Some(item_id) = item {
                doc.records[body_contents(doc, zombie)?]
                    .children
                    .iter()
                    .copied()
                    .find(|entry| {
                        resolve(doc, *entry)
                            .ok()
                            .and_then(|item| value(doc, item, "id").ok())
                            .as_deref()
                            == Some(item_id.as_str())
                    })
                    .ok_or_else(|| failure("Equipment item is not in this zombie's inventory"))?
                    .into()
            } else {
                node
            };
            let value = if let Some(node) = node {
                if !doc.records[body_contents(doc, zombie)?]
                    .children
                    .contains(&node)
                {
                    return Err(failure("Equipment is not in this zombie's body inventory"));
                }
                let item = resolve(doc, node)?;
                let definition = items
                    .items
                    .get(&value(doc, item, "id")?)
                    .ok_or_else(|| failure("Equipment definition unavailable"))?;
                let valid = match slot.as_str() {
                    "collar" => definition.item_type == "Collar",
                    "armor" => definition.item_type == "BodyArmor",
                    "hand" => {
                        definition.item_type != "Collar" && definition.item_type != "BodyArmor"
                    }
                    _ => false,
                };
                if !valid {
                    return Err(failure("Item is incompatible with this equipment slot"));
                }
                guid(doc, item)?
            } else {
                "00000000-0000-0000-0000-000000000000".into()
            };
            set_guid_field(doc, zombie, field_name, value)?;
        }
        Edit::SetTalents { ids, grant_points } => {
            let skulls = body_items(doc, zombie)?
                .iter()
                .fold(0i32, |sum, entry| {
                    let count = entry["count"]
                        .as_str()
                        .unwrap_or("0")
                        .parse::<i32>()
                        .unwrap_or(0);
                    let value = entry["id"]
                        .as_str()
                        .and_then(|id| items.items.get(id))
                        .map(|item| item.red_skulls)
                        .unwrap_or(0);
                    sum.saturating_add(count.saturating_mul(value))
                })
                .clamp(0, 999) as usize;
            let unique: HashSet<_> = ids.iter().collect();
            if unique.len() != ids.len() || ids.len() > skulls {
                return Err(failure("Zombie talents exceed available red-skull slots"));
            }
            write_talents(doc, zombie, catalog, ids, grant_points)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::{add, new_item, node, Definition};
    use crate::{Command, Workspace};

    fn list(doc: &mut Document, owner: usize, name: &str, ty: &str) -> usize {
        let list = node(doc, owner, Some(name), Some(ty), true).unwrap();
        add(doc, list, None, "array", "").unwrap()
    }

    fn sguid(doc: &mut Document, owner: usize, name: &str, id: &str) {
        let guid = node(doc, owner, Some(name), Some("SGuid, Assembly-CSharp"), true).unwrap();
        add(doc, guid, Some("id"), "string", id).unwrap();
    }

    fn fixture() -> Vec<u8> {
        let mut doc = Document::decode(&[4, 46, 5]).unwrap();
        let zombie = node(
            &mut doc,
            0,
            Some("zombie"),
            Some("ZombieWgoData, Assembly-CSharp"),
            true,
        )
        .unwrap();
        add(&mut doc, zombie, Some("id"), "string", "zombie").unwrap();
        sguid(
            &mut doc,
            zombie,
            "uniqueId",
            "10000000-0000-4000-8000-000000000001",
        );
        add(&mut doc, zombie, Some("name"), "string", "Old name").unwrap();
        add(&mut doc, zombie, Some("nameRandomed"), "bool", "true").unwrap();
        add(&mut doc, zombie, Some("zombieType"), "i32", "1").unwrap();
        add(&mut doc, zombie, Some("worldId"), "string", "village").unwrap();
        sguid(
            &mut doc,
            zombie,
            "attachedWgoDataUniqueId",
            "00000000-0000-0000-0000-000000000000",
        );
        for name in ["equippedHand", "equippedArmor", "equippedCollar"] {
            sguid(
                &mut doc,
                zombie,
                name,
                "00000000-0000-0000-0000-000000000000",
            );
        }
        for (name, value) in [("techRed", "20"), ("techGreen", "20"), ("techBlue", "20")] {
            add(&mut doc, zombie, Some(name), "i32", value).unwrap();
        }
        let definition = Definition {
            stack: 1,
            size: "Small".into(),
            capacity: 20,
            ..Default::default()
        };
        let body = new_item(
            &mut doc,
            zombie,
            "body_zombie",
            1,
            "20000000-0000-4000-8000-000000000001",
            &definition,
            None,
            false,
        )
        .unwrap();
        doc.records[body].name = Some(crate::workspace::text("zombieItem"));
        doc.records[body].tag = 1;
        let body_array = array(&doc, body, "inventory").unwrap();
        new_item(
            &mut doc,
            body_array,
            "brain_3_1:3",
            1,
            "30000000-0000-4000-8000-000000000001",
            &Definition {
                stack: 1,
                size: "Small".into(),
                ..Default::default()
            },
            None,
            false,
        )
        .unwrap();
        let res = node(
            &mut doc,
            zombie,
            Some("gameRes"),
            Some("LazyBearTechnology.GameRes, LazyBearTechnology"),
            true,
        )
        .unwrap();
        list(
            &mut doc,
            res,
            "resType",
            "System.Collections.Generic.List`1[[System.String, mscorlib]], mscorlib",
        );
        list(&mut doc, res, "resValues", "System.Collections.Generic.List`1[[LazyBearTechnology.GameResAtom, LazyBearTechnology]], mscorlib");
        let strings = node(
            &mut doc,
            zombie,
            Some("gameResStr"),
            Some("GameResStr, Assembly-CSharp"),
            true,
        )
        .unwrap();
        list(
            &mut doc,
            strings,
            "keys",
            "System.Collections.Generic.List`1[[System.String, mscorlib]], mscorlib",
        );
        list(
            &mut doc,
            strings,
            "values",
            "System.Collections.Generic.List`1[[System.String, mscorlib]], mscorlib",
        );
        list(
            &mut doc,
            zombie,
            "activePerks",
            "System.Collections.Generic.List`1[[PerkData, Assembly-CSharp]], mscorlib",
        );
        let talents = list(
            &mut doc,
            zombie,
            "talentData",
            "System.Collections.Generic.List`1[[ZombieTalentData, Assembly-CSharp]], mscorlib",
        );
        let branch = node(
            &mut doc,
            talents,
            None,
            Some("ZombieTalentData, Assembly-CSharp"),
            true,
        )
        .unwrap();
        add(&mut doc, branch, Some("id"), "string", "talent_red").unwrap();
        add(&mut doc, branch, Some("curTalentValue"), "i32", "0").unwrap();
        list(
            &mut doc,
            branch,
            "studiedLevelUps",
            "System.Collections.Generic.List`1[[System.String, mscorlib]], mscorlib",
        );
        list(
            &mut doc,
            zombie,
            "disabledTalentLevelUps",
            "System.Collections.Generic.List`1[[System.String, mscorlib]], mscorlib",
        );
        doc.rebuild().unwrap();
        doc.encode()
    }

    fn porter_fixture() -> Vec<u8> {
        let mut doc = Document::decode(&fixture()).unwrap();
        let zombie = zombies(&doc).unwrap()[0];
        set(&mut doc, zombie, "zombieType", "5".into()).unwrap();
        let inventory = node(
            &mut doc,
            zombie,
            Some("porterInventory"),
            Some("Inventory, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let container = new_item(
            &mut doc,
            inventory,
            "inventory",
            1,
            "40000000-0000-4000-8000-000000000001",
            &Definition {
                stack: 1,
                size: "Small".into(),
                capacity: 4,
                ..Default::default()
            },
            None,
            false,
        )
        .unwrap();
        doc.records[container].name = Some(crate::workspace::text("inventoryItem"));
        doc.records[container].tag = 1;
        add(&mut doc, inventory, Some("customViewId"), "string", "").unwrap();
        doc.rebuild().unwrap();
        doc.encode()
    }

    #[test]
    fn edits_round_trip_and_undo_atomically() {
        let bytes = fixture();
        let mut workspace = Workspace::default();
        let summary = workspace.open(&bytes).unwrap();
        let items: inventory::Catalog = serde_json::from_value(json!({"items":{
            "brain_3_1:3":{"stack":1,"size":"Small","groups":[],"isBag":false,"capacity":0,"durability":false,"allowed":null,"itemType":"Brain","isMainOrgan":true,"redSkulls":3,"whiteSkulls":1}
        }})).unwrap();
        workspace
            .request(
                summary.document_id,
                Command::InventoryCatalog { catalog: items },
            )
            .unwrap();
        let catalog: Catalog = serde_json::from_value(json!({
            "talents":{"z_start":{"talent":"talent_red","parents":[],"availableAtStart":true,"talentValue":2,"red":0,"green":0,"blue":0,"perk":{"id":"zperk_start","resourcesOnAdd":[{"type":"worker_bonus","value":1.0}],"resourcesOnRemove":[{"type":"worker_bonus","value":0.0}]}}},
            "appearances":{
                "zombie_worker":{"bodyIds":[1001],"headIds":[1050],"bodyLuts":[],"headLuts":["hed_lut_01"]},
                "zombie_wild":{"bodyIds":[1051],"headIds":[1050],"bodyLuts":["bdy_lut_05"],"headLuts":["hed_lut_01"]}
            }
        })).unwrap();
        workspace
            .request(summary.document_id, Command::ZombieCatalog { catalog })
            .unwrap();
        let initial = workspace
            .request(summary.document_id, Command::Zombies)
            .unwrap();
        let handle = initial["zombies"][0]["node"].as_u64().unwrap() as usize;
        assert_eq!(initial["zombies"][0]["redSkulls"], "3");
        let unsupported = workspace
            .request(
                summary.document_id,
                Command::Transact {
                    revision: summary.revision,
                    operations: vec![Operation::Zombie {
                        zombie: handle,
                        action: Edit::SetAppearance {
                            set: "zombie_wild".into(),
                            body: 1051,
                            head: 1050,
                            body_lut: "bdy_lut_05".into(),
                            head_lut: "hed_lut_01".into(),
                        },
                    }],
                },
            )
            .unwrap_err();
        assert!(unsupported
            .to_string()
            .contains("does not support the selected appearance set"));
        let result = workspace
            .request(
                summary.document_id,
                Command::Transact {
                    revision: summary.revision,
                    operations: vec![
                        Operation::Zombie {
                            zombie: handle,
                            action: Edit::Rename { name: "Ada".into() },
                        },
                        Operation::Zombie {
                            zombie: handle,
                            action: Edit::SetAppearance {
                                set: "zombie_worker".into(),
                                body: 1001,
                                head: 1050,
                                body_lut: "".into(),
                                head_lut: "hed_lut_01".into(),
                            },
                        },
                        Operation::Zombie {
                            zombie: handle,
                            action: Edit::SetTalents {
                                ids: vec!["z_start".into()],
                                grant_points: true,
                            },
                        },
                    ],
                },
            )
            .unwrap();
        let changed = workspace
            .request(summary.document_id, Command::Zombies)
            .unwrap();
        assert_eq!(changed["zombies"][0]["name"], "Ada");
        assert_eq!(changed["zombies"][0]["appearance"]["headLut"], "hed_lut_01");
        assert_eq!(changed["zombies"][0]["talents"][0]["value"], "2");
        assert_eq!(changed["zombies"][0]["activePerks"][0], "zperk_start");
        let revision = result["summary"]["revision"].as_u64().unwrap() as u32;
        workspace
            .request(summary.document_id, Command::Undo { revision })
            .unwrap();
        let restored = workspace
            .request(summary.document_id, Command::Zombies)
            .unwrap();
        assert_eq!(restored["zombies"][0]["name"], "Old name");
        assert_eq!(workspace.export(summary.document_id).unwrap(), bytes);
    }

    #[test]
    fn appearance_set_follows_generated_worker_kind() {
        assert_eq!(appearance_set_for_id("zombie"), "zombie_worker");
        assert_eq!(appearance_set_for_id("zmb_wild_mob_allie"), "zombie_worker");
        assert_eq!(
            appearance_set_for_id("zombie_assistant"),
            "zombie_assistant"
        );
    }

    #[test]
    fn porter_cargo_is_exposed_edited_and_undone() {
        let bytes = porter_fixture();
        let mut workspace = Workspace::default();
        let summary = workspace.open(&bytes).unwrap();
        let items: inventory::Catalog = serde_json::from_value(json!({"items":{
            "brain_3_1:3":{"stack":1,"size":"Small","groups":[],"isBag":false,"capacity":0,"durability":false,"allowed":null,"itemType":"Brain","isMainOrgan":true,"redSkulls":3,"whiteSkulls":1},
            "apple":{"stack":20,"size":"Small","groups":[],"isBag":false,"capacity":0,"durability":false,"allowed":null,"itemType":"Food"}
        }})).unwrap();
        workspace
            .request(
                summary.document_id,
                Command::InventoryCatalog { catalog: items },
            )
            .unwrap();
        let initial = workspace
            .request(summary.document_id, Command::Zombies)
            .unwrap();
        let zombie = initial["zombies"][0]["node"].as_u64().unwrap() as usize;
        assert_eq!(initial["zombies"][0]["cargo"]["capacity"], "4");
        assert_eq!(initial["zombies"][0]["cargo"]["items"], json!([]));
        let result = workspace
            .request(
                summary.document_id,
                Command::Transact {
                    revision: summary.revision,
                    operations: vec![Operation::Zombie {
                        zombie,
                        action: Edit::CargoInventory {
                            action: inventory::Edit::Put {
                                node: None,
                                item: "apple".into(),
                                count: "3".into(),
                                guid: "50000000-0000-4000-8000-000000000001".into(),
                                durability: None,
                            },
                            out_of_bounds: false,
                        },
                    }],
                },
            )
            .unwrap();
        let changed = workspace
            .request(summary.document_id, Command::Zombies)
            .unwrap();
        assert_eq!(changed["zombies"][0]["cargo"]["items"][0]["id"], "apple");
        assert_eq!(changed["zombies"][0]["cargo"]["items"][0]["count"], "3");
        let revision = result["summary"]["revision"].as_u64().unwrap() as u32;
        workspace
            .request(summary.document_id, Command::Undo { revision })
            .unwrap();
        assert_eq!(workspace.export(summary.document_id).unwrap(), bytes);
    }
}
