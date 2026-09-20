use crate::service::{replacement, view};
use crate::wire::{base_tag, Payload, Record, TypeInfo, WireString};
use crate::{Document, Error, NodeView};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSummary {
    pub document_id: u32,
    pub revision: u32,
    pub original_bytes: usize,
    pub encoded_bytes: usize,
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
}
struct Change {
    id: usize,
    before: Option<Record>,
    after: Record,
}
struct Patch {
    changes: Vec<Change>,
    bytes: usize,
    local: Option<LocalHistory>,
}
struct LocalHistory {
    containers: HashSet<usize>,
    general: bool,
    invalidated: bool,
    types_before: HashMap<i32, String>,
    types_after: HashMap<i32, String>,
    guids: Vec<String>,
}
struct Session {
    doc: Document,
    revision: u32,
    saved: Vec<u8>,
    current: Vec<u8>,
    undo: VecDeque<Patch>,
    redo: Vec<Patch>,
    catalog: crate::inventory::Catalog,
    inventory_cache: Option<crate::inventory::Cache>,
    hashes: Vec<Vec<u8>>,
    live_count: usize,
}
#[derive(Default)]
pub struct Workspace {
    sessions: HashMap<u32, Session>,
    next: u32,
}
fn hash_record(doc: &Document, id: usize, hashes: &[Vec<u8>]) -> Vec<u8> {
    let mut header = vec![];
    doc.records[id].write(&mut header);
    let mut h = Sha256::new();
    h.update(header);
    for child in &doc.records[id].children {
        h.update(&hashes[*child]);
    }
    h.finalize().to_vec()
}
fn tree_hashes(doc: &Document) -> Result<(Vec<Vec<u8>>, usize), Error> {
    let ids = doc.reachable()?;
    let mut hashes = vec![vec![]; doc.records.len()];
    for id in ids.iter().rev() {
        hashes[*id] = hash_record(doc, *id, &hashes);
    }
    Ok((hashes, ids.len()))
}
fn root_hash(doc: &Document, hashes: &[Vec<u8>]) -> Vec<u8> {
    let mut h = Sha256::new();
    for root in &doc.roots {
        h.update(&hashes[*root]);
    }
    h.finalize().to_vec()
}
fn rehash(s: &mut Session, changed: &[usize]) {
    s.hashes.resize(s.doc.records.len(), vec![]);
    let mut ancestors = HashSet::new();
    for id in changed {
        let mut cursor = Some(*id);
        while let Some(n) = cursor {
            if !ancestors.insert(n) {
                break;
            }
            cursor = s.doc.records[n].parent;
        }
    }
    let mut ancestors: Vec<_> = ancestors
        .into_iter()
        .map(|n| {
            let mut depth = 0;
            let mut cursor = s.doc.records[n].parent;
            while let Some(p) = cursor {
                depth += 1;
                cursor = s.doc.records[p].parent;
            }
            (n, depth)
        })
        .collect();
    ancestors.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));
    for (n, _) in ancestors {
        s.hashes[n] = hash_record(&s.doc, n, &s.hashes);
    }
    s.current = root_hash(&s.doc, &s.hashes);
}
fn wire_size(r: &Record) -> usize {
    let mut out = vec![];
    r.write(&mut out);
    out.len() + usize::from(matches!(r.tag, 1..=4 | 6))
}
pub(crate) fn text(value: &str) -> WireString {
    WireString {
        wide: true,
        bytes: value.encode_utf16().flat_map(|n| n.to_le_bytes()).collect(),
    }
}
pub(crate) fn failure(message: &str) -> Error {
    Error(message.into())
}

impl Workspace {
    // Common editor operations touch a few known scalar fields or append a verified Item.
    // Snapshot only those records. No document clone, full-tree validation or encoding is needed.
    fn transact_local(
        &mut self,
        id: u32,
        operations: &[Operation],
    ) -> Result<Option<Value>, Error> {
        let s = self.sessions.get_mut(&id).unwrap();
        let mut scope = HashSet::new();
        let mut containers = HashSet::new();
        let mut general = false;
        for op in operations {
            match op {
                Operation::Inventory {
                    container, action, ..
                } => {
                    if s.inventory_cache.is_none() {
                        s.inventory_cache =
                            Some(crate::inventory::Cache::build(&s.doc, &s.catalog)?);
                    }
                    let Some(ids) = s
                        .inventory_cache
                        .as_ref()
                        .unwrap()
                        .scope(&s.doc, &s.catalog, *container, action)?
                    else {
                        return Ok(None);
                    };
                    scope.extend(ids);
                    containers.insert(*container);
                }
                Operation::General { values } => {
                    let fields = crate::general::read(&s.doc);
                    general = true;
                    for key in values.keys() {
                        let Some(node) = fields
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|v| v["key"] == *key)
                            .and_then(|v| v["node"].as_u64())
                        else {
                            return Ok(None);
                        };
                        scope.insert(node as usize);
                    }
                }
                Operation::Set { node, .. }
                    if matches!(base_tag(active(&s.doc, *node)?.tag), 16..=44) =>
                {
                    scope.insert(*node);
                    general = true;
                }
                _ => return Ok(None),
            }
        }
        // Mixed structural/Inspector batches use the general validator.
        if !containers.is_empty()
            && operations
                .iter()
                .any(|op| !matches!(op, Operation::Inventory { .. }))
        {
            return Ok(None);
        }
        let before: Vec<_> = scope
            .iter()
            .map(|id| (*id, s.doc.records[*id].clone()))
            .collect();
        let start = s.doc.records.len();
        let types = s.doc.types.clone();
        let next = s.doc.next_object_id.get();
        let result = (|| -> Result<(), Error> {
            let mut new_guids = HashSet::new();
            for op in operations {
                if let Operation::Inventory {
                    container,
                    action,
                    out_of_bounds,
                } = op
                {
                    if let crate::inventory::Edit::Put {
                        node: None, guid, ..
                    } = action
                    {
                        if !new_guids.insert(guid.to_ascii_lowercase()) {
                            return Err(failure("Duplicate item GUID"));
                        }
                    }
                    crate::inventory::write_inner(
                        &mut s.doc,
                        &s.catalog,
                        *container,
                        action.clone(),
                        *out_of_bounds,
                        true,
                    )?;
                } else {
                    apply(&mut s.doc, op.clone())?;
                }
            }
            if s.doc.records.len() > crate::wire::Limits::default().max_records {
                return Err(failure("Record limit exceeded"));
            }
            for node in start..s.doc.records.len() {
                active(&s.doc, node)?;
            }
            for id in scope.iter().copied().chain(start..s.doc.records.len()) {
                let r = &mut s.doc.records[id];
                if matches!(r.payload, Payload::Array(_)) {
                    r.payload = Payload::Array(r.children.len() as i64);
                }
            }
            let old_size: usize = before.iter().map(|(_, r)| wire_size(r)).sum();
            let new_size: usize = scope
                .iter()
                .copied()
                .chain(start..s.doc.records.len())
                .map(|n| wire_size(&s.doc.records[n]))
                .sum();
            let size = s.doc.encoded_size - old_size + new_size;
            if size > s.doc.max_bytes {
                return Err(failure("Document exceeds size limit"));
            }
            s.doc.encoded_size = size;
            Ok(())
        })();
        if let Err(e) = result {
            for (id, r) in before {
                s.doc.records[id] = r;
            }
            s.doc.records.truncate(start);
            s.doc.types = types;
            s.doc.next_object_id.set(next);
            return Err(e);
        }
        let mut patch = Patch {
            changes: vec![],
            bytes: 0,
            local: Some(LocalHistory {
                containers: containers.clone(),
                general,
                invalidated: operations
                    .iter()
                    .any(|op| matches!(op, Operation::Set { .. })),
                types_before: types,
                types_after: s.doc.types.clone(),
                guids: operations
                    .iter()
                    .filter_map(|op| match op {
                        Operation::Inventory {
                            action:
                                crate::inventory::Edit::Put {
                                    node: None, guid, ..
                                },
                            ..
                        } => Some(guid.to_ascii_lowercase()),
                        _ => None,
                    })
                    .collect(),
            }),
        };
        for (id, r) in before {
            if r != s.doc.records[id] {
                patch.bytes += record_size(&r) + record_size(&s.doc.records[id]);
                patch.changes.push(Change {
                    id,
                    before: Some(r),
                    after: s.doc.records[id].clone(),
                });
            }
        }
        for id in start..s.doc.records.len() {
            let r = &s.doc.records[id];
            patch.bytes += record_size(r);
            patch.changes.push(Change {
                id,
                before: None,
                after: r.clone(),
            });
            if let Payload::Node {
                object: Some(object),
                ..
            } = r.payload
            {
                s.doc.objects.insert(object, id);
            }
        }
        let changed: Vec<_> = patch.changes.iter().map(|c| c.id).collect();
        if !changed.is_empty() {
            s.hashes.resize(s.doc.records.len(), vec![]);
            s.live_count += s.doc.records.len() - start;
            let mut ancestors = HashSet::new();
            for id in &changed {
                let mut cursor = Some(*id);
                while let Some(n) = cursor {
                    if !ancestors.insert(n) {
                        break;
                    }
                    cursor = s.doc.records[n].parent;
                }
            }
            let mut ancestors: Vec<_> = ancestors
                .into_iter()
                .map(|n| {
                    let mut depth = 0;
                    let mut cursor = s.doc.records[n].parent;
                    while let Some(p) = cursor {
                        depth += 1;
                        cursor = s.doc.records[p].parent;
                    }
                    (n, depth)
                })
                .collect();
            ancestors.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));
            for (n, _) in ancestors {
                s.hashes[n] = hash_record(&s.doc, n, &s.hashes);
            }
            s.current = root_hash(&s.doc, &s.hashes);
            s.revision += 1;
            s.redo.clear();
            s.undo.push_back(patch);
            while s.undo.len() > 100
                || s.undo.iter().map(|p| p.bytes).sum::<usize>() > 64 * 1024 * 1024
            {
                s.undo.pop_front();
            }
        }
        let invalidated = operations
            .iter()
            .any(|op| matches!(op, Operation::Set { .. }));
        let inventory = if containers.is_empty() {
            // A raw scalar edit can change an inventory field or filter; refresh lazily if visited.
            if invalidated {
                s.inventory_cache = None;
            }
            None
        } else {
            let cache = s.inventory_cache.as_mut().unwrap();
            for op in operations {
                if let Operation::Inventory {
                    action:
                        crate::inventory::Edit::Put {
                            node: None, guid, ..
                        },
                    ..
                } = op
                {
                    cache.guids.insert(guid.to_ascii_lowercase());
                }
            }
            Some(cache.updates(&s.doc, &s.catalog, &containers)?)
        };
        let general = if general {
            Some(crate::general::read(&s.doc))
        } else {
            None
        };
        Ok(Some(
            json!({"summary":self.summary(id)?,"changed":changed,"deleted":[],"inventory":inventory,"general":general,"inventoryInvalidated":invalidated}),
        ))
    }
    pub fn open(&mut self, bytes: &[u8]) -> Result<DocumentSummary, Error> {
        let doc = Document::decode(bytes)?;
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| failure("Document ID limit exceeded"))?;
        let (hashes, live_count) = tree_hashes(&doc)?;
        let hash = root_hash(&doc, &hashes);
        self.sessions.insert(
            self.next,
            Session {
                doc,
                revision: 1,
                saved: hash.clone(),
                current: hash,
                undo: VecDeque::new(),
                redo: vec![],
                catalog: Default::default(),
                inventory_cache: None,
                hashes,
                live_count,
            },
        );
        self.summary(self.next)
    }
    pub fn export(&self, id: u32) -> Result<Vec<u8>, Error> {
        Ok(self.session(id)?.doc.encode())
    }
    fn session(&self, id: u32) -> Result<&Session, Error> {
        self.sessions
            .get(&id)
            .ok_or_else(|| failure("Document is closed"))
    }
    pub fn summary(&self, id: u32) -> Result<DocumentSummary, Error> {
        let s = self.session(id)?;
        Ok(DocumentSummary {
            document_id: id,
            revision: s.revision,
            original_bytes: s.doc.original_size,
            encoded_bytes: s.doc.encoded_size,
            records: s.live_count,
            types: s.doc.type_count(),
            objects: s.doc.object_count(),
            dirty: s.saved != s.current,
            can_undo: !s.undo.is_empty(),
            can_redo: !s.redo.is_empty(),
        })
    }
    pub fn mark_saved(&mut self, id: u32, revision: u32) -> Result<DocumentSummary, Error> {
        self.check_revision(id, revision)?;
        let s = self.sessions.get_mut(&id).unwrap();
        s.saved = s.current.clone();
        self.summary(id)
    }
    fn check_revision(&self, id: u32, revision: u32) -> Result<(), Error> {
        if self.session(id)?.revision != revision {
            return Err(failure(
                "Stale revision. Refresh before applying this edit.",
            ));
        }
        Ok(())
    }
    pub fn request(&mut self, id: u32, command: Command) -> Result<Value, Error> {
        if matches!(command, Command::Close) {
            self.sessions
                .remove(&id)
                .ok_or_else(|| failure("Document is closed"))?;
            return Ok(json!({"closed":id}));
        }
        let doc = &self.session(id)?.doc;
        match command {
            Command::Summary => Ok(json!(self.summary(id)?)),
            Command::Children {
                parent,
                offset,
                limit,
            } => {
                if limit == 0 || limit > 200 {
                    return Err(failure("Page size must be 1..200"));
                }
                let children = if let Some(node) = parent {
                    &active(doc, node)?.children
                } else {
                    &doc.roots
                };
                Ok(
                    json!({"nodes":children.iter().skip(offset).take(limit).map(|id|view(doc,*id)).collect::<Vec<NodeView>>(),"total":children.len()}),
                )
            }
            Command::Nodes { ids } => {
                if ids.len() > 200 {
                    return Err(failure("Too many node queries"));
                }
                let nodes: Vec<_> = ids
                    .iter()
                    .map(|id| {
                        active(doc, *id)?;
                        Ok(view(doc, *id))
                    })
                    .collect::<Result<_, Error>>()?;
                Ok(json!(nodes))
            }
            Command::General => Ok(json!(crate::general::read(doc))),
            Command::Inventories => {
                let s = self.sessions.get_mut(&id).unwrap();
                if s.inventory_cache.is_none() {
                    s.inventory_cache = Some(crate::inventory::Cache::build(&s.doc, &s.catalog)?);
                }
                s.inventory_cache.as_ref().unwrap().snapshot(&s.doc)
            }
            Command::InventoryCatalog { catalog } => {
                catalog.validate()?;
                self.sessions.get_mut(&id).unwrap().catalog = catalog;
                self.sessions.get_mut(&id).unwrap().inventory_cache = None;
                Ok(json!({"loaded":true}))
            }
            Command::Templates => Ok(json!(TEMPLATES
                .iter()
                .map(|s| serde_json::from_str::<Value>(s).unwrap())
                .collect::<Vec<_>>())),
            Command::Transact {
                revision,
                operations,
            } => self.transact(id, revision, operations),
            Command::Undo { revision } => self.history(id, revision, false),
            Command::Redo { revision } => self.history(id, revision, true),
            Command::MarkSaved { revision } => Ok(json!(self.mark_saved(id, revision)?)),
            Command::Close => unreachable!(),
        }
    }
    fn transact(
        &mut self,
        id: u32,
        revision: u32,
        operations: Vec<Operation>,
    ) -> Result<Value, Error> {
        self.check_revision(id, revision)?;
        if operations.is_empty() || operations.len() > 1000 {
            return Err(failure("A transaction must contain 1..1000 operations"));
        }
        if let Some(result) = self.transact_local(id, &operations)? {
            return Ok(result);
        }
        let inventory_only = operations
            .iter()
            .all(|op| matches!(op, Operation::Inventory { .. }));
        let inventory_containers: HashSet<_> = operations
            .iter()
            .filter_map(|op| match op {
                Operation::Inventory { container, .. } => Some(*container),
                _ => None,
            })
            .collect();
        let new_guids: Vec<_> = operations
            .iter()
            .filter_map(|op| match op {
                Operation::Inventory {
                    action: crate::inventory::Edit::Put { guid, .. },
                    ..
                } => Some(guid.to_ascii_lowercase()),
                _ => None,
            })
            .collect();
        let mut candidate = self.session(id)?.doc.clone();
        for op in operations {
            if let Operation::Inventory {
                container,
                action,
                out_of_bounds,
            } = op
            {
                crate::inventory::write(
                    &mut candidate,
                    &self.session(id)?.catalog,
                    container,
                    action,
                    out_of_bounds,
                )?;
            } else {
                apply(&mut candidate, op)?;
            }
        }
        candidate.rebuild()?;
        let s = self.sessions.get_mut(&id).unwrap();
        let mut patch = Patch {
            changes: vec![],
            bytes: 0,
            local: None,
        };
        for (index, after) in candidate.records.iter().enumerate() {
            let before = s.doc.records.get(index);
            if before != Some(after) {
                patch.bytes += record_size(after) + before.map(record_size).unwrap_or(0);
                patch.changes.push(Change {
                    id: index,
                    before: before.cloned(),
                    after: after.clone(),
                });
            }
        }
        let changed: Vec<_> = patch.changes.iter().map(|c| c.id).collect();
        let old: HashSet<_> = s.doc.reachable()?.into_iter().collect();
        let new: HashSet<_> = candidate.reachable()?.into_iter().collect();
        let deleted: Vec<_> = old.difference(&new).copied().collect();
        if !patch.changes.is_empty() {
            let (hashes, live_count) = tree_hashes(&candidate)?;
            s.current = root_hash(&candidate, &hashes);
            s.hashes = hashes;
            s.live_count = live_count;
            s.doc = candidate;
            if !inventory_only {
                s.inventory_cache = None;
            }
            s.revision = s
                .revision
                .checked_add(1)
                .ok_or_else(|| failure("Revision limit exceeded"))?;
            s.redo.clear();
            s.undo.push_back(patch);
            while s.undo.len() > 100
                || s.undo.iter().map(|p| p.bytes).sum::<usize>() > 64 * 1024 * 1024
            {
                s.undo.pop_front();
            }
        }
        let inventory = if inventory_only {
            if let Some(cache) = &mut s.inventory_cache {
                cache.guids.extend(new_guids);
                Some(cache.updates(&s.doc, &s.catalog, &inventory_containers)?)
            } else {
                None
            }
        } else {
            None
        };
        let invalidated = inventory.is_none();
        let general = (!inventory_only).then(|| crate::general::read(&s.doc));
        Ok(
            json!({"summary":self.summary(id)?,"changed":changed,"deleted":deleted,"general":general,"inventory":inventory,"inventoryInvalidated":invalidated}),
        )
    }
    fn history(&mut self, id: u32, revision: u32, redo: bool) -> Result<Value, Error> {
        self.check_revision(id, revision)?;
        let s = self.sessions.get_mut(&id).unwrap();
        let patch = if redo {
            s.redo.pop()
        } else {
            s.undo.pop_back()
        }
        .ok_or_else(|| failure("No history available"))?;
        if let Some(local) = &patch.local {
            let changed: Vec<_> = patch.changes.iter().map(|c| c.id).collect();
            let mut deleted = vec![];
            for change in &patch.changes {
                let old = if redo {
                    change.before.as_ref()
                } else {
                    Some(&change.after)
                };
                let new = if redo {
                    Some(&change.after)
                } else {
                    change.before.as_ref()
                };
                s.doc.encoded_size = s.doc.encoded_size - old.map(wire_size).unwrap_or(0)
                    + new.map(wire_size).unwrap_or(0);
                if let Some(record) = new {
                    s.doc.records[change.id] = record.clone();
                }
                if change.before.is_none() {
                    if redo {
                        s.live_count += 1;
                    } else {
                        s.live_count -= 1;
                        deleted.push(change.id);
                    }
                    if let Payload::Node {
                        object: Some(object),
                        ..
                    } = change.after.payload
                    {
                        if redo {
                            s.doc.objects.insert(object, change.id);
                        } else {
                            s.doc.objects.remove(&object);
                        }
                    }
                }
            }
            s.doc.types = if redo {
                local.types_after.clone()
            } else {
                local.types_before.clone()
            };
            rehash(s, &changed);
            s.revision += 1;
            let mut invalidated = local.invalidated;
            let inventory = if local.containers.is_empty() {
                None
            } else if let Some(cache) = &mut s.inventory_cache {
                for guid in &local.guids {
                    if redo {
                        cache.guids.insert(guid.clone());
                    } else {
                        cache.guids.remove(guid);
                    }
                }
                Some(cache.updates(&s.doc, &s.catalog, &local.containers)?)
            } else {
                invalidated = true;
                None
            };
            if invalidated {
                s.inventory_cache = None;
            }
            let general = local.general.then(|| crate::general::read(&s.doc));
            if redo {
                s.undo.push_back(patch);
            } else {
                s.redo.push(patch);
            }
            return Ok(
                json!({"summary":self.summary(id)?,"changed":changed,"deleted":deleted,"general":general,"inventory":inventory,"inventoryInvalidated":invalidated}),
            );
        }
        let old: HashSet<_> = s.doc.reachable()?.into_iter().collect();
        for change in &patch.changes {
            if redo {
                s.doc.records[change.id] = change.after.clone();
            } else if let Some(before) = &change.before {
                s.doc.records[change.id] = before.clone();
            }
            // Appended records stay allocated but unreachable: handles are never reused.
        }
        s.doc.rebuild()?;
        let (hashes, live_count) = tree_hashes(&s.doc)?;
        s.current = root_hash(&s.doc, &hashes);
        s.hashes = hashes;
        s.live_count = live_count;
        s.inventory_cache = None;
        s.revision += 1;
        let new: HashSet<_> = s.doc.reachable()?.into_iter().collect();
        let deleted: Vec<_> = old.difference(&new).copied().collect();
        let changed: Vec<_> = patch.changes.iter().map(|c| c.id).collect();
        if redo {
            s.undo.push_back(patch);
        } else {
            s.redo.push(patch);
        }
        Ok(
            json!({"summary":self.summary(id)?,"changed":changed,"deleted":deleted,"general":crate::general::read(&self.session(id)?.doc),"inventoryInvalidated":true}),
        )
    }
}
fn record_size(r: &Record) -> usize {
    let mut bytes = vec![];
    r.write(&mut bytes);
    bytes.len() + r.children.len() * 8 + 128
}
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
        if depth > 256 {
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
    }
    Ok(())
}
