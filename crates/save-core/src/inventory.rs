//! Game inventory selectors and atomic edits. Asset definitions are supplied once per document.
use crate::service::view;
use crate::wire::{base_tag, Payload, Record, TypeInfo};
use crate::workspace::{
    apply, attach, failure, field, next_object, resolve, scalar, text, Operation,
};
use crate::{Document, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub items: BTreeMap<String, Definition>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    #[serde(default)]
    pub family: Option<String>,
    pub stack: i32,
    pub size: String,
    pub groups: Vec<String>,
    pub is_bag: bool,
    pub capacity: i32,
    pub durability: bool,
    pub allowed: Option<Vec<String>>,
}
impl Catalog {
    pub fn validate(&self) -> Result<(), Error> {
        if self.items.len() > 100_000
            || self
                .items
                .iter()
                .any(|(id, d)| id.len() > 1024 || d.stack < 1 || d.capacity < 0)
        {
            return Err(failure("Invalid inventory catalog"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Capacity {
        value: String,
    },
    Remove {
        node: usize,
    },
    Put {
        node: Option<usize>,
        item: String,
        count: String,
        guid: String,
    },
}
fn value(doc: &Document, parent: usize, name: &str) -> Result<String, Error> {
    view(doc, field(doc, parent, name)?)
        .value
        .ok_or_else(|| failure("Missing scalar value"))
}
fn number(doc: &Document, parent: usize, name: &str) -> Result<i32, Error> {
    let node = field(doc, parent, name)?;
    if base_tag(doc.records[node].tag) != 24 {
        return Err(failure("Expected int32 inventory field"));
    }
    value(doc, parent, name)?
        .parse()
        .map_err(|_| failure("Invalid inventory number"))
}
fn array(doc: &Document, list: usize) -> Result<usize, Error> {
    let list = resolve(doc, list)?;
    let children = &doc.records[list].children;
    let arrays: Vec<_> = children
        .iter()
        .filter(|id| doc.records[**id].tag == 6)
        .copied()
        .collect();
    if arrays.len() != 1 {
        return Err(failure("Unsupported inventory collection"));
    }
    Ok(arrays[0])
}
fn contents(doc: &Document, item: usize) -> Result<usize, Error> {
    array(doc, field(doc, item, "inventory")?)
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
    name.split(',').next().unwrap_or("").to_owned()
}
fn properties(doc: &Document, item: usize) -> Result<Vec<usize>, Error> {
    let root = field(doc, item, "properties")?;
    let mut result = vec![];
    let mut pending = vec![root];
    let mut seen = HashSet::new();
    while let Some(id) = pending.pop() {
        let id = resolve(doc, id)?;
        if !seen.insert(id) {
            continue;
        }
        if short_type(doc, id).ends_with("SerializedItemProperty") {
            result.push(id);
        } else {
            pending.extend(&doc.records[id].children);
        }
    }
    Ok(result)
}
fn strings(doc: &Document, parent: usize, name: &str) -> Result<Vec<String>, Error> {
    doc.records[array(doc, field(doc, parent, name)?)?]
        .children
        .iter()
        .map(|id| {
            if let Payload::Text(v) = &doc.records[*id].payload {
                Ok(v.display())
            } else {
                Err(failure("Unsupported inventory filter"))
            }
        })
        .collect()
}
fn allowed(doc: &Document, catalog: &Catalog, container: usize, id: &str) -> Result<bool, Error> {
    if id.is_empty() || id == "empty" {
        return Ok(false);
    }
    let d = catalog
        .items
        .get(id)
        .ok_or_else(|| failure("Item definition unavailable"))?;
    if d.size != "Small" && d.size != "Big" {
        return Ok(false);
    }
    if d.size == "Big" {
        let mut ancestor = Some(container);
        while let Some(id) = ancestor {
            if doc.records[id]
                .name
                .as_ref()
                .is_some_and(|n| n.display() == "playerData")
            {
                return Ok(false);
            }
            ancestor = doc.records[id].parent;
        }
    }
    let container_id = value(doc, container, "id")?;
    if let Some(bag) = catalog.items.get(&container_id).filter(|d| d.is_bag) {
        let allowed = bag
            .allowed
            .as_ref()
            .ok_or_else(|| failure("Bag insertion rules are incomplete"))?;
        if !allowed.iter().any(|entry| entry == id) {
            return Ok(false);
        }
    }
    for p in properties(doc, container)? {
        let ty = short_type(doc, p);
        let white = ty == "WhiteListFilterSerializedItemProperty";
        if white || ty == "BlackListFilterSerializedItemProperty" {
            let filter = field(doc, p, if white { "whiteList" } else { "blackList" })?;
            let ids = strings(doc, filter, "itemsIds")?;
            let groups = strings(doc, filter, "groupsIds")?;
            let id_match = ids.iter().any(|v| v == id);
            let group_match = groups.iter().any(|v| d.groups.contains(v));
            if white && ((!ids.is_empty() && !id_match) || (!groups.is_empty() && !group_match)) {
                return Ok(false);
            }
            if !white && (id_match || group_match) {
                return Ok(false);
            }
        } else if !matches!(
            ty.as_str(),
            "AutoExpandSerializedItemProperty"
                | "FuelContainerSerializedItemProperty"
                | "DurabilitySerializedItemProperty"
        ) {
            return Err(failure(
                "This container has unsupported item rules; use the Inspector",
            ));
        }
    }
    Ok(true)
}
fn containers(doc: &Document, catalog: &Catalog) -> Result<Vec<(usize, String, String)>, Error> {
    let root = *doc
        .roots
        .first()
        .ok_or_else(|| failure("Missing save root"))?;
    let mut result: Vec<(usize, String, String)> = vec![];
    let mut seen = HashSet::new();
    if let Ok(player) = field(doc, root, "playerData")
        .and_then(|p| field(doc, p, "inventory"))
        .and_then(|p| field(doc, p, "inventoryItem"))
    {
        result.push((player, "Player".into(), "Player inventory".into()));
        seen.insert(player);
    }
    for id in doc.reachable()? {
        let r = &doc.records[id];
        if r.name.as_ref().is_none_or(|n| n.display() != "inventory")
            || short_type(doc, id) != "Inventory"
        {
            continue;
        }
        let Some(owner) = r.parent else {
            continue;
        };
        let mut cursor = Some(owner);
        let mut world = false;
        while let Some(n) = cursor {
            if doc.records[n]
                .name
                .as_ref()
                .is_some_and(|s| s.display() == "wgoDataList")
            {
                world = true;
                break;
            }
            cursor = doc.records[n].parent;
        }
        if !world {
            continue;
        }
        let item = field(doc, id, "inventoryItem")?;
        if !seen.insert(item)
            || (number(doc, item, "inventorySize")? == 0
                && doc.records[contents(doc, item)?].children.is_empty())
        {
            continue;
        }
        let title = value(doc, owner, "id").unwrap_or_else(|_| "World object".into());
        let kind = if title.to_lowercase().contains("chest") {
            "Chest"
        } else {
            "World object"
        };
        result.push((item, kind.into(), title));
    }
    let mut index = 0;
    while index < result.len() {
        let item = result[index].0;
        index += 1;
        for entry in &doc.records[contents(doc, item)?].children {
            let entry = resolve(doc, *entry)?;
            let id = value(doc, entry, "id")?;
            if catalog.items.get(&id).is_some_and(|d| d.is_bag) && seen.insert(entry) {
                result.push((entry, "Bag".into(), id));
            }
        }
    }
    result.sort_by_key(|(_, kind, _)| match kind.as_str() {
        "Player" => 0,
        "Bag" => 1,
        "Chest" => 2,
        _ => 3,
    });
    Ok(result)
}
#[cfg(test)]
pub(crate) fn read(doc: &Document, catalog: &Catalog) -> Result<Value, Error> {
    let mut result = vec![];
    for (node, kind, title) in containers(doc, catalog)? {
        let items:Vec<_>=doc.records[contents(doc,node)?].children.iter().map(|entry|{
            let item=resolve(doc,*entry)?;
            let durability=properties(doc,item)?.iter().find(|p|short_type(doc,**p)=="DurabilitySerializedItemProperty").and_then(|p|value(doc,*p,"durability").ok());
            Ok(json!({"node":entry,"id":value(doc,item,"id")?,"count":number(doc,item,"count")?.to_string(),"durability":durability}))
        }).collect::<Result<_,Error>>()?;
        let mut error = None;
        let mut valid = vec![];
        for id in catalog.items.keys() {
            match allowed(doc, catalog, node, id) {
                Ok(true) => valid.push(id),
                Ok(false) => {}
                Err(e) => {
                    error = Some(e.to_string());
                    break;
                }
            }
        }
        result.push(json!({"node":node,"kind":kind,"title":title,"capacity":number(doc,node,"inventorySize")?.to_string(),"items":items,"allowed":valid,"error":error}));
    }
    Ok(json!(result))
}

#[derive(Clone)]
struct Entry {
    node: usize,
    kind: String,
    title: String,
    location: Value,
    rule: usize,
}
#[derive(Default)]
pub(crate) struct Cache {
    entries: BTreeMap<usize, Entry>,
    rules: Vec<Value>,
    rule_keys: BTreeMap<String, usize>,
}
impl Cache {
    pub(crate) fn build(doc: &Document, catalog: &Catalog) -> Result<Self, Error> {
        let mut cache = Self::default();
        for (node, kind, title) in containers(doc, catalog)? {
            cache.register(doc, catalog, node, kind, title)?;
        }
        // Initialize the allocator once, rather than scanning the whole document per new object.
        if doc.next_object_id.get().is_none() {
            let next = doc
                .objects
                .keys()
                .max()
                .copied()
                .unwrap_or(-1)
                .checked_add(1)
                .ok_or_else(|| failure("Object ID limit exceeded"))?;
            doc.next_object_id.set(Some(next));
        }
        Ok(cache)
    }
    fn register(
        &mut self,
        doc: &Document,
        catalog: &Catalog,
        node: usize,
        kind: String,
        title: String,
    ) -> Result<(), Error> {
        let mut location = json!({});
        let mut cursor = doc.records[node].parent;
        let mut player = false;
        while let Some(id) = cursor {
            let ty = short_type(doc, id);
            if ty == "GameSceneData" {
                location["scene"] = json!(value(doc, id, "id").ok());
            }
            if ty.ends_with("WgoData")
                || doc.records[id]
                    .name
                    .as_ref()
                    .is_some_and(|n| n.display() == "playerData")
            {
                player |= doc.records[id]
                    .name
                    .as_ref()
                    .is_some_and(|n| n.display() == "playerData");
                if location.get("world").is_none() {
                    location["world"] = json!(value(doc, id, "worldId").ok());
                }
                if location.get("position").is_none() {
                    if let Ok(pos) = field(doc, id, "position") {
                        let values: Vec<_> = doc.records[pos]
                            .children
                            .iter()
                            .filter_map(|n| view(doc, *n).value)
                            .collect();
                        if values.len() == 3 {
                            location["position"] = json!(values);
                        }
                    }
                }
            }
            cursor = doc.records[id].parent;
        }
        let container_id = value(doc, node, "id")?;
        let bag = catalog.items.get(&container_id).is_some_and(|d| d.is_bag);
        let filters: Vec<_> = properties(doc, node)?
            .iter()
            .map(|p| {
                let ty = short_type(doc, *p);
                let data = if ty == "WhiteListFilterSerializedItemProperty"
                    || ty == "BlackListFilterSerializedItemProperty"
                {
                    field(
                        doc,
                        *p,
                        if ty.starts_with("White") {
                            "whiteList"
                        } else {
                            "blackList"
                        },
                    )
                    .and_then(|f| {
                        Ok(json!([
                            strings(doc, f, "itemsIds")?,
                            strings(doc, f, "groupsIds")?
                        ]))
                    })
                    .unwrap_or(json!("invalid"))
                } else {
                    Value::Null
                };
                json!([ty, data])
            })
            .collect();
        let key = json!([
            player,
            if bag { container_id.as_str() } else { "" },
            filters
        ])
        .to_string();
        let rule = if let Some(id) = self.rule_keys.get(&key) {
            *id
        } else {
            let mut allowed_ids = vec![];
            let mut error = None;
            for id in catalog.items.keys() {
                match allowed(doc, catalog, node, id) {
                    Ok(true) => allowed_ids.push(id.clone()),
                    Ok(false) => {}
                    Err(e) => {
                        error = Some(e.to_string());
                        break;
                    }
                }
            }
            let id = self.rules.len();
            self.rules
                .push(json!({"allowed":allowed_ids,"error":error}));
            self.rule_keys.insert(key, id);
            id
        };
        self.entries.insert(
            node,
            Entry {
                node,
                kind,
                title,
                location,
                rule,
            },
        );
        Ok(())
    }
    fn entry(&self, doc: &Document, entry: &Entry) -> Result<Value, Error> {
        let items=doc.records[contents(doc,entry.node)?].children.iter().map(|n|{
            let item=resolve(doc,*n)?;
            let durability=properties(doc,item)?.iter().find(|p|short_type(doc,**p)=="DurabilitySerializedItemProperty").and_then(|p|value(doc,*p,"durability").ok());
            Ok(json!({"node":n,"id":value(doc,item,"id")?,"count":number(doc,item,"count")?.to_string(),"durability":durability}))
        }).collect::<Result<Vec<_>,Error>>()?;
        Ok(
            json!({"node":entry.node,"kind":entry.kind,"title":entry.title,"location":entry.location,"ruleId":entry.rule,"capacity":number(doc,entry.node,"inventorySize")?.to_string(),"items":items}),
        )
    }
    pub(crate) fn snapshot(&self, doc: &Document) -> Result<Value, Error> {
        let mut entries = self.entries.values().collect::<Vec<_>>();
        entries.sort_by_key(|e| match e.kind.as_str() {
            "Player" => 0,
            "Bag" => 1,
            "Chest" => 2,
            _ => 3,
        });
        Ok(
            json!({"inventories":entries.into_iter().map(|e|self.entry(doc,e)).collect::<Result<Vec<_>,_>>()?,"rules":self.rules.iter().enumerate().map(|(id,v)|(id.to_string(),v.clone())).collect::<BTreeMap<_,_>>()}),
        )
    }
    pub(crate) fn updates(
        &mut self,
        doc: &Document,
        catalog: &Catalog,
        containers: &HashSet<usize>,
    ) -> Result<Value, Error> {
        let first_rule = self.rules.len();
        let removed: Vec<_> = self
            .entries
            .keys()
            .copied()
            .filter(|n| crate::workspace::active(doc, *n).is_err())
            .collect();
        for n in &removed {
            self.entries.remove(n);
        }
        let mut updated = containers.clone();
        let mut pending = containers
            .iter()
            .copied()
            .filter(|n| self.entries.contains_key(n))
            .collect::<Vec<_>>();
        while let Some(container) = pending.pop() {
            for n in &doc.records[contents(doc, container)?].children {
                let n = resolve(doc, *n)?;
                let id = value(doc, n, "id")?;
                if catalog.items.get(&id).is_some_and(|d| d.is_bag)
                    && !self.entries.contains_key(&n)
                {
                    self.register(doc, catalog, n, "Bag".into(), id)?;
                    updated.insert(n);
                    pending.push(n);
                }
            }
        }
        Ok(
            json!({"upsert":updated.iter().filter_map(|n|self.entries.get(n)).map(|e|self.entry(doc,e)).collect::<Result<Vec<_>,_>>()?,"removed":removed,"rules":self.rules.iter().enumerate().skip(first_rule).map(|(id,v)|(id.to_string(),v.clone())).collect::<BTreeMap<_,_>>()}),
        )
    }
    pub(crate) fn check_container(&self, doc: &Document, container: usize) -> Result<(), Error> {
        if !self.entries.contains_key(&container) {
            return Err(failure("Unsupported inventory"));
        }
        crate::workspace::active(doc, container)?;
        Ok(())
    }
}
pub(crate) fn new_guid<'a>(
    doc: &Document,
    catalog: &Catalog,
    edit: &'a Edit,
) -> Result<Option<&'a str>, Error> {
    let Edit::Put {
        node, item, guid, ..
    } = edit
    else {
        return Ok(None);
    };
    if let Some(node) = node {
        let old_id = value(doc, resolve(doc, *node)?, "id")?;
        let same_family = catalog
            .items
            .get(&old_id)
            .zip(catalog.items.get(item))
            .is_some_and(|(old, new)| {
                old.family.is_some()
                    && old.family == new.family
                    && old.capacity == new.capacity
                    && old.durability == new.durability
            });
        if old_id == *item || same_family {
            return Ok(None);
        }
    }
    Ok(Some(guid))
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
// Construct only the verified Item layout; never copy another item's identity or properties.
fn node(
    doc: &mut Document,
    parent: usize,
    name: Option<&str>,
    ty: Option<&str>,
    reference: bool,
) -> Result<usize, Error> {
    let type_info = if let Some(ty) = ty {
        if let Some((id, _)) = doc.types.iter().find(|(_, v)| v.as_str() == ty) {
            TypeInfo::Reference(*id)
        } else {
            let id = doc
                .types
                .keys()
                .max()
                .copied()
                .unwrap_or(-1)
                .checked_add(1)
                .ok_or_else(|| failure("Type ID limit exceeded"))?;
            doc.types.insert(id, ty.into());
            TypeInfo::Definition(id, text(ty))
        }
    } else {
        TypeInfo::None
    };
    let object = if reference {
        Some(next_object(doc)?)
    } else {
        None
    };
    attach(
        doc,
        parent,
        doc.records[parent].children.len(),
        Record {
            tag: match (reference, name.is_some()) {
                (true, true) => 1,
                (true, false) => 2,
                (false, true) => 3,
                _ => 4,
            },
            name: name.map(text),
            payload: Payload::Node {
                ty: type_info,
                object,
            },
            parent: None,
            children: vec![],
            offset: 0,
        },
    )
}
fn add(
    doc: &mut Document,
    parent: usize,
    name: Option<&str>,
    kind: &str,
    value: &str,
) -> Result<usize, Error> {
    attach(
        doc,
        parent,
        doc.records[parent].children.len(),
        scalar(kind, name.map(str::to_owned), value)?,
    )
}
fn new_item(
    doc: &mut Document,
    parent: usize,
    id: &str,
    count: i32,
    guid: &str,
    d: &Definition,
    guid_checked: bool,
) -> Result<usize, Error> {
    if guid.len() != 36
        || guid.chars().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c != '-'
            } else {
                !c.is_ascii_hexdigit()
            }
        })
    {
        return Err(failure("Invalid item GUID"));
    }
    if !guid_checked
        && doc.reachable()?.iter().any(|n| {
            short_type(doc, *n) == "SGuid"
                && value(doc, *n, "id").is_ok_and(|v| v.eq_ignore_ascii_case(guid))
        })
    {
        return Err(failure("Duplicate item GUID"));
    }
    let item = node(doc, parent, None, Some("Item, Assembly-CSharp"), true)?;
    add(doc, item, Some("id"), "string", id)?;
    add(doc, item, Some("count"), "i32", &count.to_string())?;
    let unique = node(
        doc,
        item,
        Some("uniqueId"),
        Some("SGuid, Assembly-CSharp"),
        true,
    )?;
    add(doc, unique, Some("id"), "string", guid)?;
    let inv = node(
        doc,
        item,
        Some("inventory"),
        Some("System.Collections.Generic.List`1[[Item, Assembly-CSharp]], mscorlib"),
        true,
    )?;
    add(doc, inv, None, "array", "")?;
    add(
        doc,
        item,
        Some("inventorySize"),
        "i32",
        &d.capacity.to_string(),
    )?;
    add(doc, item, Some("inventoryFillSize"), "i32", "-1")?;
    let props=node(doc,item,Some("properties"),Some("System.Collections.Generic.Dictionary`2[[System.Type, mscorlib],[SerializedItemProperty, Assembly-CSharp]], mscorlib"),true)?;
    add(doc, props, Some("comparer"), "null", "")?;
    let entries = add(doc, props, None, "array", "")?;
    if d.durability {
        let pair = node(doc, entries, None, None, false)?;
        // TypeFormatter writes the type name inside a reference node for the runtime Type.
        let type_name = doc
            .types
            .values()
            .find(|t| t.starts_with("System.RuntimeType,") || t.starts_with("System.MonoType,"))
            .cloned()
            .unwrap_or_else(|| "System.RuntimeType, mscorlib".into());
        let key = node(doc, pair, Some("$k"), Some(&type_name), true)?;
        add(
            doc,
            key,
            None,
            "string",
            "DurabilitySerializedItemProperty, Assembly-CSharp",
        )?;
        let p = node(
            doc,
            pair,
            Some("$v"),
            Some("DurabilitySerializedItemProperty, Assembly-CSharp"),
            true,
        )?;
        add(doc, p, Some("durability"), "f32", "1")?;
    }
    Ok(item)
}
pub(crate) fn write_inner(
    doc: &mut Document,
    catalog: &Catalog,
    container: usize,
    edit: Edit,
    out_of_bounds: bool,
    checked: bool,
) -> Result<(), Error> {
    if !checked
        && !containers(doc, catalog)?
            .iter()
            .any(|(n, _, _)| *n == container)
    {
        return Err(failure("Unsupported inventory"));
    }
    let array = contents(doc, container)?;
    let capacity = number(doc, container, "inventorySize")?;
    match edit {
        Edit::Capacity { value } => {
            let n: i32 = value
                .parse()
                .map_err(|_| failure("Capacity must be an integer"))?;
            if !out_of_bounds || n < 0 || (n as usize) < doc.records[array].children.len() {
                return Err(failure(
                    "Enable out of bounds edits; capacity cannot be smaller than its contents",
                ));
            }
            set(doc, container, "inventorySize", value)?;
        }
        Edit::Remove { node } => {
            if !doc.records[array].children.contains(&node) {
                return Err(failure("Item is not in this inventory"));
            }
            apply(doc, Operation::Remove { node })?;
        }
        Edit::Put {
            node: existing,
            item,
            count,
            guid,
        } => {
            let count: i32 = count
                .parse()
                .map_err(|_| failure("Amount must be an integer"))?;
            let d = catalog
                .items
                .get(&item)
                .ok_or_else(|| failure("Item definition unavailable"))?;
            if count < 1 || (!out_of_bounds && count > d.stack) {
                return Err(failure("Amount exceeds the item stack size"));
            }
            if !allowed(doc, catalog, container, &item)? {
                return Err(failure("Item is not allowed in this inventory"));
            }
            if let Some(existing) = existing {
                if !doc.records[array].children.contains(&existing) {
                    return Err(failure("Item is not in this inventory"));
                }
                let target = resolve(doc, existing)?;
                let old_id = value(doc, target, "id")?;
                let same_family = catalog.items.get(&old_id).is_some_and(|old| {
                    d.family.is_some()
                        && old.family == d.family
                        && old.capacity == d.capacity
                        && old.durability == d.durability
                });
                if old_id == item || same_family {
                    if old_id != item {
                        set(doc, target, "id", item.clone())?;
                    }
                    set(doc, target, "count", count.to_string())?;
                } else {
                    if !doc.records[contents(doc, target)?].children.is_empty() {
                        return Err(failure("Empty this item's inventory before replacing it"));
                    }
                    let index = doc.records[array]
                        .children
                        .iter()
                        .position(|n| *n == existing)
                        .unwrap();
                    let new = new_item(doc, array, &item, count, &guid, d, checked)?;
                    apply(doc, Operation::Remove { node: existing })?;
                    apply(doc, Operation::Move { node: new, index })?;
                }
            } else {
                if doc.records[array].children.len() >= capacity.max(0) as usize {
                    return Err(failure("Inventory is full"));
                }
                new_item(doc, array, &item, count, &guid, d, checked)?;
            }
        }
    }
    // The game caches occupied slots on the container. Never leave this cache stale.
    set(
        doc,
        container,
        "inventoryFillSize",
        doc.records[array].children.len().to_string(),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Workspace;
    fn catalog() -> Value {
        json!({"items":{
            "apple":{"stack":10,"size":"Small","groups":["food"],"isBag":false,"capacity":0,"durability":false,"allowed":null},
            "tool":{"stack":1,"size":"Small","groups":["tool"],"isBag":false,"capacity":0,"durability":true,"allowed":null},
            "bag":{"stack":1,"size":"Small","groups":[],"isBag":true,"capacity":2,"durability":false,"allowed":["apple"]},
            "log":{"stack":1,"size":"Big","groups":[],"isBag":false,"capacity":0,"durability":false,"allowed":null}
        }})
    }
    fn fixture() -> Vec<u8> {
        let mut doc = Document::decode(&[4, 46, 5]).unwrap();
        let p = node(&mut doc, 0, Some("playerData"), None, false).unwrap();
        let inv = node(
            &mut doc,
            p,
            Some("inventory"),
            Some("Inventory, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let d = Definition {
            family: None,
            stack: 1,
            size: "Small".into(),
            groups: vec![],
            is_bag: false,
            capacity: 3,
            durability: false,
            allowed: None,
        };
        let item = new_item(
            &mut doc,
            inv,
            "inventory",
            1,
            "11111111-1111-4111-8111-111111111111",
            &d,
            false,
        )
        .unwrap();
        doc.records[item].name = Some(text("inventoryItem"));
        doc.records[item].tag = 1;
        doc.rebuild().unwrap();
        doc.encode()
    }
    fn request(w: &mut Workspace, id: u32, v: Value) -> Result<Value, Error> {
        let inventory = v["op"] == "inventories";
        let mut result = w.request(id, serde_json::from_value(v).unwrap())?;
        if inventory {
            let rules = result["rules"].clone();
            for entry in result["inventories"].as_array_mut().unwrap() {
                let rule = &rules[entry["ruleId"].to_string()];
                entry["allowed"] = rule["allowed"].clone();
                entry["error"] = rule["error"].clone();
            }
            return Ok(result["inventories"].take());
        }
        Ok(result)
    }
    fn transact(
        w: &mut Workspace,
        id: u32,
        container: usize,
        action: Value,
        extra: bool,
    ) -> Result<Value, Error> {
        request(
            w,
            id,
            json!({"op":"transact","revision":w.summary(id).unwrap().revision,"operations":[{"op":"inventory","container":container,"action":action,"out_of_bounds":extra}]}),
        )
    }
    #[test]
    fn incremental_results_history_and_failed_batches() {
        let bytes = fixture();
        let mut w = Workspace::default();
        let id = w.open(&bytes).unwrap().document_id;
        request(
            &mut w,
            id,
            json!({"op":"inventory_catalog","catalog":catalog()}),
        )
        .unwrap();
        let snapshot = w
            .request(id, crate::workspace::Command::Inventories)
            .unwrap();
        let c = snapshot["inventories"][0]["node"].as_u64().unwrap() as usize;
        assert!(snapshot["inventories"][0].get("allowed").is_none());
        assert_eq!(snapshot["rules"].as_object().unwrap().len(), 1);
        let put = json!({"kind":"put","node":null,"item":"tool","count":"1","guid":"55555555-5555-4555-8555-555555555555"});
        let op = json!({"op":"inventory","container":c,"action":put});
        let rev = w.summary(id).unwrap().revision;
        let invalid = json!({"op":"inventory","container":c,"out_of_bounds":true,"action":{"kind":"capacity","value":"-1"}});
        assert!(request(
            &mut w,
            id,
            json!({"op":"transact","revision":rev,"operations":[op,invalid]})
        )
        .is_err());
        assert_eq!(w.export(id).unwrap(), bytes);
        let result = transact(&mut w, id, c, put, false).unwrap();
        assert_eq!(result["inventory"]["upsert"].as_array().unwrap().len(), 1);
        assert_eq!(result["inventory"]["rules"], json!({}));
        assert_eq!(result["inventoryInvalidated"], false);
        assert!(result["general"].is_null());
        let after = w.export(id).unwrap();
        assert_eq!(Some(after.len()), w.summary(id).unwrap().encoded_bytes);
        let rev = w.summary(id).unwrap().revision;
        request(&mut w, id, json!({"op":"undo","revision":rev})).unwrap();
        assert_eq!(w.export(id).unwrap(), bytes);
        assert!(!w.summary(id).unwrap().dirty);
        let rev = w.summary(id).unwrap().revision;
        let redone = request(&mut w, id, json!({"op":"redo","revision":rev})).unwrap();
        assert_eq!(redone["inventory"]["upsert"].as_array().unwrap().len(), 1);
        assert_eq!(w.export(id).unwrap(), after);
        Document::decode(&after).unwrap();
    }
    #[test]
    fn inventory_insertion_limits_durability_bags_and_undo() {
        let bytes = fixture();
        let mut w = Workspace::default();
        let id = w.open(&bytes).unwrap().document_id;
        request(
            &mut w,
            id,
            json!({"op":"inventory_catalog","catalog":catalog()}),
        )
        .unwrap();
        let inventories = request(&mut w, id, json!({"op":"inventories"})).unwrap();
        let c = inventories[0]["node"].as_u64().unwrap() as usize;
        assert_eq!(inventories[0]["kind"], "Player");
        assert!(!inventories[0]["allowed"]
            .as_array()
            .unwrap()
            .contains(&json!("log")));
        let put = |item: &str, count: &str| json!({"kind":"put","node":null,"item":item,"count":count,"guid":"22222222-2222-4222-8222-222222222222"});
        assert!(transact(&mut w, id, c, put("apple", "11"), false).is_err());
        assert_eq!(w.export(id).unwrap(), bytes);
        transact(&mut w, id, c, put("tool", "1"), false).unwrap();
        let exported = w.export(id).unwrap();
        Document::decode(&exported).unwrap();
        let inv = request(&mut w, id, json!({"op":"inventories"})).unwrap();
        assert_eq!(inv[0]["items"][0]["durability"], "1");
        let revision = w.summary(id).unwrap().revision;
        request(&mut w, id, json!({"op":"undo","revision":revision})).unwrap();
        assert_eq!(w.export(id).unwrap(), bytes);
        transact(&mut w, id, c, put("bag", "1"), false).unwrap();
        let inv = request(&mut w, id, json!({"op":"inventories"})).unwrap();
        assert_eq!(inv[1]["kind"], "Bag");
        assert_eq!(inv[1]["allowed"], json!(["apple"]));
        assert!(transact(&mut w, id, c, json!({"kind":"capacity","value":"5"}), false).is_err());
        transact(&mut w, id, c, json!({"kind":"capacity","value":"5"}), true).unwrap();
        let item = inv[0]["items"][0]["node"].as_u64().unwrap();
        transact(&mut w, id, c, json!({"kind":"remove","node":item}), false).unwrap();
        assert_eq!(
            request(&mut w, id, json!({"op":"inventories"})).unwrap()[0]["items"],
            json!([])
        );
        assert!(transact(&mut w, id, c, json!({"kind":"remove","node":item}), false).is_err());
    }
    #[test]
    fn deletion_replacement_and_guid_indexes_survive_history() {
        let bytes = fixture();
        let mut w = Workspace::default();
        let id = w.open(&bytes).unwrap().document_id;
        request(
            &mut w,
            id,
            json!({"op":"inventory_catalog","catalog":catalog()}),
        )
        .unwrap();
        let c = request(&mut w, id, json!({"op":"inventories"})).unwrap()[0]["node"]
            .as_u64()
            .unwrap() as usize;
        let put = |item: &str, node: Option<usize>, guid: &str| json!({"kind":"put","node":node,"item":item,"count":"1","guid":guid});
        let first = "44444444-4444-4444-8444-444444444444";
        let second = "55555555-5555-4555-8555-555555555555";
        let inserted = transact(&mut w, id, c, put("bag", None, first), false).unwrap();
        let bag = inserted["inventory"]["upsert"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["kind"] == "Bag")
            .unwrap()["node"]
            .as_u64()
            .unwrap() as usize;
        transact(&mut w, id, bag, put("apple", None, second), false).unwrap();
        let populated = w.export(id).unwrap();
        let removed = transact(&mut w, id, c, json!({"kind":"remove","node":bag}), false).unwrap();
        assert_eq!(removed["inventory"]["removed"], json!([bag]));
        assert_eq!(removed["inventoryInvalidated"], false);
        assert!(removed["deleted"].as_array().unwrap().len() > 12);
        let revision = w.summary(id).unwrap().revision;
        request(&mut w, id, json!({"op":"undo","revision":revision})).unwrap();
        assert_eq!(w.export(id).unwrap(), populated);
        assert!(transact(&mut w, id, c, put("apple", None, second), false).is_err());
        let revision = w.summary(id).unwrap().revision;
        request(&mut w, id, json!({"op":"redo","revision":revision})).unwrap();
        // A removed nested item's identity is available again; discarded redo records stay allocated.
        let result = transact(&mut w, id, c, put("apple", None, second), false).unwrap();
        let item = result["inventory"]["upsert"][0]["items"][0]["node"]
            .as_u64()
            .unwrap() as usize;
        let apple = w.export(id).unwrap();
        let replaced = transact(&mut w, id, c, put("tool", Some(item), first), false).unwrap();
        assert_eq!(replaced["inventoryInvalidated"], false);
        assert!(replaced["deleted"]
            .as_array()
            .unwrap()
            .contains(&json!(item)));
        assert_eq!(
            replaced["inventory"]["upsert"][0]["items"][0]["durability"],
            "1"
        );
        let tool = w.export(id).unwrap();
        let revision = w.summary(id).unwrap().revision;
        request(&mut w, id, json!({"op":"undo","revision":revision})).unwrap();
        assert_eq!(w.export(id).unwrap(), apple);
        let revision = w.summary(id).unwrap().revision;
        request(&mut w, id, json!({"op":"redo","revision":revision})).unwrap();
        assert_eq!(w.export(id).unwrap(), tool);
        // Redo restores the GUID index, including newly introduced type definitions.
        assert!(transact(&mut w, id, c, put("apple", None, first), false).is_err());
        assert_eq!(w.export(id).unwrap(), tool);
    }

    #[test]
    fn unchanged_amount_keeps_identity_and_atomic_batch_rejects() {
        let mut w = Workspace::default();
        let id = w.open(&fixture()).unwrap().document_id;
        request(
            &mut w,
            id,
            json!({"op":"inventory_catalog","catalog":catalog()}),
        )
        .unwrap();
        let c = request(&mut w, id, json!({"op":"inventories"})).unwrap()[0]["node"]
            .as_u64()
            .unwrap() as usize;
        transact(&mut w,id,c,json!({"kind":"put","node":null,"item":"apple","count":"2","guid":"33333333-3333-4333-8333-333333333333"}),false).unwrap();
        let item = request(&mut w, id, json!({"op":"inventories"})).unwrap()[0]["items"][0]["node"]
            .as_u64()
            .unwrap();
        let before = w.export(id).unwrap();
        transact(
            &mut w,
            id,
            c,
            json!({"kind":"put","node":item,"item":"apple","count":"2","guid":"unused"}),
            false,
        )
        .unwrap();
        assert_eq!(w.export(id).unwrap(), before);
        let rev = w.summary(id).unwrap().revision;
        let op = json!({"op":"inventory","container":c,"action":{"kind":"capacity","value":"9"},"out_of_bounds":true});
        assert!(request(&mut w,id,json!({"op":"transact","revision":rev,"operations":[op,{"op":"inventory","container":c,"action":{"kind":"remove","node":0}}]})).is_err());
        assert_eq!(w.export(id).unwrap(), before);
        transact(
            &mut w,
            id,
            c,
            json!({"kind":"put","node":item,"item":"apple","count":"50","guid":"unused"}),
            true,
        )
        .unwrap();
        assert_eq!(
            request(&mut w, id, json!({"op":"inventories"})).unwrap()[0]["items"][0]["count"],
            "50"
        );
    }

    #[test]
    fn world_whitelist_and_blacklist_and_large_items() {
        let mut doc = Document::decode(&fixture()).unwrap();
        let world = node(&mut doc, 0, Some("worldData"), None, false).unwrap();
        let list = node(&mut doc, world, Some("wgoDataList"), None, false).unwrap();
        let owner = node(&mut doc, list, None, Some("WgoData, Assembly-CSharp"), true).unwrap();
        add(&mut doc, owner, Some("id"), "string", "wood_container").unwrap();
        let inv = node(
            &mut doc,
            owner,
            Some("inventory"),
            Some("Inventory, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let definition = Definition {
            family: None,
            stack: 1,
            size: "Small".into(),
            groups: vec![],
            is_bag: false,
            capacity: 2,
            durability: false,
            allowed: None,
        };
        let container = new_item(
            &mut doc,
            inv,
            "inventory",
            1,
            "44444444-4444-4444-8444-444444444444",
            &definition,
            false,
        )
        .unwrap();
        doc.records[container].name = Some(text("inventoryItem"));
        doc.records[container].tag = 1;
        let props = field(&doc, container, "properties").unwrap();
        let array = array(&doc, props).unwrap();
        let prop = node(
            &mut doc,
            array,
            None,
            Some("WhiteListFilterSerializedItemProperty, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let filter = node(
            &mut doc,
            prop,
            Some("whiteList"),
            Some("WhiteListItemFilter, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let ids = node(&mut doc, filter, Some("itemsIds"), None, false).unwrap();
        let ids = add(&mut doc, ids, None, "array", "").unwrap();
        add(&mut doc, ids, None, "string", "log").unwrap();
        let groups = node(&mut doc, filter, Some("groupsIds"), None, false).unwrap();
        add(&mut doc, groups, None, "array", "").unwrap();
        doc.rebuild().unwrap();
        let catalog: Catalog = serde_json::from_value(catalog()).unwrap();
        let inventories = read(&doc, &catalog).unwrap();
        assert_eq!(inventories[0]["kind"], "Player");
        assert_eq!(inventories[1]["allowed"], json!(["log"]));
        let black = node(
            &mut doc,
            array,
            None,
            Some("BlackListFilterSerializedItemProperty, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let filter = node(
            &mut doc,
            black,
            Some("blackList"),
            Some("BlackListItemFilter, Assembly-CSharp"),
            true,
        )
        .unwrap();
        let ids = node(&mut doc, filter, Some("itemsIds"), None, false).unwrap();
        let ids = add(&mut doc, ids, None, "array", "").unwrap();
        add(&mut doc, ids, None, "string", "log").unwrap();
        let groups = node(&mut doc, filter, Some("groupsIds"), None, false).unwrap();
        add(&mut doc, groups, None, "array", "").unwrap();
        doc.rebuild().unwrap();
        assert_eq!(read(&doc, &catalog).unwrap()[1]["allowed"], json!([]));
    }
}
