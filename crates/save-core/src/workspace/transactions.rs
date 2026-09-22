use super::*;
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
struct Change {
    id: usize,
    before: Option<Record>,
    after: Record,
    before_live: bool,
    after_live: bool,
}
struct Patch {
    changes: Vec<Change>,
    bytes: usize,
    containers: HashSet<usize>,
    general: bool,
    invalidated: bool,
    types_before: HashMap<i32, String>,
    types_after: HashMap<i32, String>,
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
    live: Vec<bool>,
    live_count: usize,
    references: HashMap<i32, HashSet<usize>>,
    guids: HashMap<String, HashSet<usize>>,
    encoded_bytes: std::cell::Cell<Option<usize>>,
}
#[derive(Default)]
pub struct Workspace {
    sessions: HashMap<u32, Session>,
    next: u32,
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

fn hash_record(doc: &Document, id: usize, hashes: &[Vec<u8>]) -> Vec<u8> {
    let mut header = vec![];
    doc.records[id].write(&mut header);
    let mut hash = Sha256::new();
    hash.update(header);
    for child in &doc.records[id].children {
        hash.update(&hashes[*child]);
    }
    hash.finalize().to_vec()
}
fn root_hash(doc: &Document, hashes: &[Vec<u8>]) -> Vec<u8> {
    let mut hash = Sha256::new();
    for root in &doc.roots {
        hash.update(&hashes[*root]);
    }
    hash.finalize().to_vec()
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
fn object(record: &Record) -> Option<i32> {
    if let Payload::Node { object, .. } = record.payload {
        object
    } else {
        None
    }
}
fn reference(record: &Record) -> Option<i32> {
    if base_tag(record.tag) == 10 {
        if let Payload::Fixed(bytes) = &record.payload {
            return Some(i32::from_le_bytes(bytes.as_slice().try_into().unwrap()));
        }
    }
    None
}
fn guid(doc: &Document, record: &Record) -> Option<String> {
    let Payload::Text(value) = &record.payload else {
        return None;
    };
    if record.name.as_ref()?.display() != "id" {
        return None;
    }
    let Payload::Node { ty, .. } = &doc.records[record.parent?].payload else {
        return None;
    };
    let id = match ty {
        TypeInfo::Definition(id, _) | TypeInfo::Reference(id) => id,
        _ => return None,
    };
    let name = doc.types.get(id)?.split(',').next()?.trim();
    (name == "SGuid").then(|| value.display().to_ascii_lowercase())
}
fn record_size(record: &Record) -> usize {
    let mut bytes = vec![];
    record.write(&mut bytes);
    bytes.len() + record.children.len() * std::mem::size_of::<usize>() + 128
}
fn index_record(s: &mut Session, id: usize, record: &Record, add: bool) {
    if let Some(object) = object(record) {
        if add {
            s.doc.objects.insert(object, id);
        } else {
            s.doc.objects.remove(&object);
        }
    }
    if let Some(target) = reference(record) {
        if add {
            s.references.entry(target).or_default().insert(id);
        } else if let Some(nodes) = s.references.get_mut(&target) {
            nodes.remove(&id);
            if nodes.is_empty() {
                s.references.remove(&target);
            }
        }
    }
    if let Some(guid) = guid(&s.doc, record) {
        if add {
            s.guids.entry(guid).or_default().insert(id);
        } else if let Some(nodes) = s.guids.get_mut(&guid) {
            nodes.remove(&id);
            if nodes.is_empty() {
                s.guids.remove(&guid);
            }
        }
    }
}
// Check only references whose target or source is affected by this transaction.
// Checks happen against the final batch, allowing remove + retarget in either order.
fn validate_changes(s: &Session, changes: &[Change]) -> Result<(), Error> {
    let affected: HashSet<_> = changes.iter().map(|c| c.id).collect();
    let mut objects = HashMap::new();
    for change in changes.iter().filter(|c| c.after_live) {
        if let Some(object) = object(&change.after) {
            if objects.insert(object, change.id).is_some()
                || s.doc
                    .objects
                    .get(&object)
                    .is_some_and(|n| !affected.contains(n))
            {
                return Err(failure("Duplicate object ID"));
            }
        }
    }
    let exists = |id: &i32| {
        objects.contains_key(id) || s.doc.objects.get(id).is_some_and(|n| !affected.contains(n))
    };
    for change in changes {
        if change.before_live {
            if let Some(object) = change.before.as_ref().and_then(object) {
                if !exists(&object)
                    && s.references
                        .get(&object)
                        .is_some_and(|refs| refs.iter().any(|n| !affected.contains(n)))
                {
                    return Err(failure("Removal would leave a dangling reference; retarget it in the same transaction"));
                }
            }
        }
        if change.after_live {
            if let Some(target) = reference(&change.after) {
                if !exists(&target) {
                    return Err(failure("Removal would leave a dangling reference; retarget it in the same transaction"));
                }
            }
        }
    }
    Ok(())
}
fn update_indexes(s: &mut Session, patch: &Patch, redo: bool) {
    s.live.resize(s.doc.records.len(), false);
    // Remove all old index entries before adding new ones (including shared GUIDs).
    for c in &patch.changes {
        let (old, live) = if redo {
            (c.before.as_ref(), c.before_live)
        } else {
            (Some(&c.after), c.after_live)
        };
        if live {
            index_record(s, c.id, old.unwrap(), false);
            s.live_count -= 1;
        }
    }
    for c in &patch.changes {
        let (new, live) = if redo {
            (Some(&c.after), c.after_live)
        } else {
            (c.before.as_ref(), c.before_live)
        };
        s.live[c.id] = live;
        if live {
            index_record(s, c.id, new.unwrap(), true);
            s.live_count += 1;
        }
    }
    s.encoded_bytes.set(None);
}
fn delta(s: &mut Session, patch: &Patch) -> Option<Value> {
    if patch.invalidated {
        s.inventory_cache = None;
        return None;
    }
    if patch.containers.is_empty() {
        return None;
    }
    let result = s
        .inventory_cache
        .as_mut()?
        .updates(&s.doc, &s.catalog, &patch.containers);
    match result {
        Ok(value) => Some(value),
        // A raw/mixed edit can make a formerly supported container unavailable.
        // The transaction still succeeded; discovery is refreshed on demand.
        Err(_) => {
            s.inventory_cache = None;
            None
        }
    }
}
impl Workspace {
    pub fn open(&mut self, bytes: &[u8]) -> Result<DocumentSummary, Error> {
        let doc = Document::decode(bytes)?;
        doc.next_object_id.set(Some(
            doc.objects
                .keys()
                .max()
                .copied()
                .unwrap_or(-1)
                .checked_add(1)
                .ok_or_else(|| failure("Object ID limit exceeded"))?,
        ));
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| failure("Document ID limit exceeded"))?;
        let ids = doc.reachable()?;
        let mut hashes = vec![vec![]; doc.records.len()];
        let mut live = vec![false; doc.records.len()];
        for id in ids.iter().rev() {
            hashes[*id] = hash_record(&doc, *id, &hashes);
            live[*id] = true;
        }
        let hash = root_hash(&doc, &hashes);
        let mut s = Session {
            doc,
            revision: 1,
            saved: hash.clone(),
            current: hash,
            undo: VecDeque::new(),
            redo: vec![],
            catalog: Default::default(),
            inventory_cache: None,
            hashes,
            live,
            live_count: ids.len(),
            references: HashMap::new(),
            guids: HashMap::new(),
            encoded_bytes: std::cell::Cell::new(Some(bytes.len())),
        };
        for id in ids {
            let record = s.doc.records[id].clone();
            index_record(&mut s, id, &record, true);
        }
        self.sessions.insert(self.next, s);
        self.summary(self.next)
    }
    pub fn export(&self, id: u32) -> Result<Vec<u8>, Error> {
        let s = self.session(id)?;
        let bytes = s.doc.encode();
        if bytes.len() > s.doc.max_bytes {
            return Err(failure("Document exceeds size limit"));
        }
        // Comprehensive wire validation is paid once when exporting, never while editing.
        Document::decode(&bytes)?;
        s.encoded_bytes.set(Some(bytes.len()));
        Ok(bytes)
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
            encoded_bytes: s.encoded_bytes.get(),
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
                let nodes = ids
                    .iter()
                    .map(|id| {
                        active(doc, *id)?;
                        Ok(view(doc, *id))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                Ok(json!(nodes))
            }
            Command::General => Ok(crate::general::read(doc)),
            Command::Drops => crate::drops::read(doc),
            Command::Inventories => {
                let s = self.sessions.get_mut(&id).unwrap();
                if s.inventory_cache.is_none() {
                    s.inventory_cache = Some(crate::inventory::Cache::build(&s.doc, &s.catalog)?);
                }
                s.inventory_cache.as_ref().unwrap().snapshot(&s.doc)
            }
            Command::InventoryCatalog { catalog } => {
                catalog.validate()?;
                let s = self.sessions.get_mut(&id).unwrap();
                s.catalog = catalog;
                s.inventory_cache = None;
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
        let next_revision = revision
            .checked_add(1)
            .ok_or_else(|| failure("Revision limit exceeded"))?;
        let s = self.sessions.get_mut(&id).unwrap();
        let general_unchanged = operations
            .iter()
            .all(|op| matches!(op, Operation::Inventory { .. } | Operation::Drops { .. }));
        if operations
            .iter()
            .any(|op| matches!(op, Operation::Inventory { .. }))
            && s.inventory_cache.is_none()
        {
            s.inventory_cache = Some(crate::inventory::Cache::build(&s.doc, &s.catalog)?);
        }
        let containers = operations
            .iter()
            .filter_map(|op| {
                if let Operation::Inventory { container, .. } = op {
                    Some(*container)
                } else {
                    None
                }
            })
            .collect();
        let invalidated = operations.iter().any(|op| {
            !matches!(
                op,
                Operation::Inventory { .. } | Operation::General { .. } | Operation::Drops { .. }
            )
        });
        let start = s.doc.records.len();
        let types = s.doc.types.clone();
        let next_object = s.doc.next_object_id.get();
        s.doc.records.begin();
        let mut new_guids = HashSet::new();
        let result = (|| {
            for op in operations {
                if let Operation::Inventory {
                    container,
                    action,
                    out_of_bounds,
                } = op
                {
                    s.inventory_cache
                        .as_ref()
                        .unwrap()
                        .check_container(&s.doc, container)?;
                    if let Some(guid) = crate::inventory::new_guid(&s.doc, &s.catalog, &action)? {
                        let guid = guid.to_ascii_lowercase();
                        new_guids.insert(guid);
                    }
                    crate::inventory::write_inner(
                        &mut s.doc,
                        &s.catalog,
                        container,
                        action,
                        out_of_bounds,
                        true,
                    )?;
                } else if let Operation::Drops { action } = op {
                    crate::drops::write(&mut s.doc, action)?;
                } else {
                    apply(&mut s.doc, op)?;
                }
            }
            if s.doc.records.len() > crate::wire::Limits::default().max_records {
                return Err(failure("Record limit exceeded"));
            }
            for id in s
                .doc
                .records
                .touched()
                .into_iter()
                .chain(start..s.doc.records.len())
            {
                if matches!(s.doc.records[id].payload, Payload::Array(_)) {
                    let count = s.doc.records[id].children.len();
                    s.doc.records[id].payload = Payload::Array(count as i64);
                }
            }
            Ok(())
        })();
        let before = s.doc.records.finish();
        let changes = (|| {
            result?;
            let mut affected: HashSet<usize> = before
                .keys()
                .copied()
                .chain(start..s.doc.records.len())
                .collect();
            // Only detached subtrees require a walk. Unchanged siblings are never visited.
            for (id, old) in &before {
                let children: HashSet<_> = s.doc.records[*id].children.iter().copied().collect();
                let mut pending: Vec<_> = old
                    .children
                    .iter()
                    .filter(|n| !children.contains(n))
                    .copied()
                    .collect();
                let mut seen = HashSet::new();
                while let Some(n) = pending.pop() {
                    if !seen.insert(n) {
                        continue;
                    }
                    affected.insert(n);
                    pending.extend(
                        before
                            .get(&n)
                            .unwrap_or(&s.doc.records[n])
                            .children
                            .iter()
                            .copied(),
                    );
                }
            }
            let mut changes = vec![];
            for n in affected {
                let old = if n < start {
                    Some(before.get(&n).unwrap_or(&s.doc.records[n]).clone())
                } else {
                    None
                };
                let after_live = active(&s.doc, n).is_ok();
                // New attached nodes must meet the depth limit even if active() failed.
                if n >= start && !after_live {
                    let mut cursor = n;
                    let mut depth = 0;
                    while let Some(parent) = s.doc.records[cursor].parent {
                        if !s.doc.records[parent].children.contains(&cursor) {
                            break;
                        }
                        depth += 1;
                        if depth > crate::wire::Limits::default().max_depth
                            || (depth == crate::wire::Limits::default().max_depth
                                && matches!(s.doc.records[n].tag, 1..=4 | 6))
                        {
                            return Err(failure("Hierarchy depth exceeded"));
                        }
                        cursor = parent;
                    }
                }
                let before_live = s.live.get(n).copied().unwrap_or(false);
                if old.as_ref() != Some(&s.doc.records[n]) || before_live != after_live {
                    changes.push(Change {
                        id: n,
                        before: old,
                        after: s.doc.records[n].clone(),
                        before_live,
                        after_live,
                    });
                }
            }
            validate_changes(s, &changes)?;
            // Validate generated identities against the final batch, including raw GUID edits
            // and items removed earlier in the same transaction.
            if !new_guids.is_empty() {
                let affected: HashSet<_> = changes.iter().map(|c| c.id).collect();
                let mut counts = HashMap::<String, usize>::new();
                for c in changes.iter().filter(|c| c.after_live) {
                    if let Some(guid) = guid(&s.doc, &c.after) {
                        *counts.entry(guid).or_default() += 1;
                    }
                }
                for guid in new_guids {
                    let retained = s
                        .guids
                        .get(&guid)
                        .map(|nodes| nodes.iter().filter(|n| !affected.contains(n)).count())
                        .unwrap_or(0);
                    if retained + counts.get(&guid).copied().unwrap_or(0) > 1 {
                        return Err(failure("Duplicate item GUID"));
                    }
                }
            }
            Ok(changes)
        })();
        let changes = match changes {
            Ok(changes) => changes,
            Err(error) => {
                for (id, record) in before {
                    s.doc.records[id] = record;
                }
                s.doc.records.truncate(start);
                s.doc.types = types;
                s.doc.next_object_id.set(next_object);
                return Err(error);
            }
        };
        let patch = Patch {
            bytes: changes
                .iter()
                .map(|c| record_size(&c.after) + c.before.as_ref().map(record_size).unwrap_or(0))
                .sum(),
            changes,
            containers,
            general: !general_unchanged,
            invalidated,
            types_before: types,
            types_after: s.doc.types.clone(),
        };
        let changed: Vec<_> = patch
            .changes
            .iter()
            .filter(|c| c.after_live)
            .map(|c| c.id)
            .collect();
        let deleted: Vec<_> = patch
            .changes
            .iter()
            .filter(|c| c.before_live && !c.after_live)
            .map(|c| c.id)
            .collect();
        if !patch.changes.is_empty() {
            update_indexes(s, &patch, true);
            let touched: Vec<_> = patch.changes.iter().map(|c| c.id).collect();
            rehash(s, &touched);
            s.revision = next_revision;
            s.redo.clear();
        }
        let inventory = delta(s, &patch);
        let inventory_invalidated =
            patch.invalidated || (!patch.containers.is_empty() && inventory.is_none());
        let general = patch.general.then(|| crate::general::read(&s.doc));
        if !patch.changes.is_empty() {
            s.undo.push_back(patch);
            while s.undo.len() > 100
                || s.undo.iter().map(|p| p.bytes).sum::<usize>() > 64 * 1024 * 1024
            {
                s.undo.pop_front();
            }
        }
        Ok(
            json!({"summary":self.summary(id)?,"changed":changed,"deleted":deleted,"general":general,"inventory":inventory,"inventoryInvalidated":inventory_invalidated}),
        )
    }
    fn history(&mut self, id: u32, revision: u32, redo: bool) -> Result<Value, Error> {
        self.check_revision(id, revision)?;
        let next_revision = revision
            .checked_add(1)
            .ok_or_else(|| failure("Revision limit exceeded"))?;
        let s = self.sessions.get_mut(&id).unwrap();
        let patch = if redo {
            s.redo.pop()
        } else {
            s.undo.pop_back()
        }
        .ok_or_else(|| failure("No history available"))?;
        // Index GUID keys while all type definitions still exist, including on redo.
        s.doc.types.extend(patch.types_after.clone());
        update_indexes(s, &patch, redo);
        let mut changed = vec![];
        let mut deleted = vec![];
        for c in &patch.changes {
            let (record, live, old_live) = if redo {
                (Some(&c.after), c.after_live, c.before_live)
            } else {
                (c.before.as_ref(), c.before_live, c.after_live)
            };
            if let Some(record) = record {
                s.doc.records[c.id] = record.clone();
            }
            if live {
                changed.push(c.id);
            } else if old_live {
                deleted.push(c.id);
            }
        }
        s.doc.types = if redo {
            patch.types_after.clone()
        } else {
            patch.types_before.clone()
        };
        let touched: Vec<_> = patch.changes.iter().map(|c| c.id).collect();
        rehash(s, &touched);
        s.revision = next_revision;
        let inventory = delta(s, &patch);
        let invalidated =
            patch.invalidated || (!patch.containers.is_empty() && inventory.is_none());
        let general = patch.general.then(|| crate::general::read(&s.doc));
        if redo {
            s.undo.push_back(patch);
        } else {
            s.redo.push(patch);
        }
        Ok(
            json!({"summary":self.summary(id)?,"changed":changed,"deleted":deleted,"general":general,"inventory":inventory,"inventoryInvalidated":invalidated}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_indexes(workspace: &Workspace, id: u32) {
        let s = workspace.session(id).unwrap();
        let mut rebuilt = s.doc.clone();
        rebuilt.rebuild().unwrap();
        let ids = rebuilt.reachable().unwrap();
        let live: HashSet<_> = ids.iter().copied().collect();
        assert_eq!(s.live_count, ids.len());
        assert_eq!(s.doc.objects, rebuilt.objects);
        assert_eq!(
            s.live
                .iter()
                .enumerate()
                .filter_map(|(n, live)| live.then_some(n))
                .collect::<HashSet<_>>(),
            live
        );
        let mut references: HashMap<i32, HashSet<usize>> = HashMap::new();
        let mut guids: HashMap<String, HashSet<usize>> = HashMap::new();
        let mut hashes = vec![vec![]; rebuilt.records.len()];
        for n in ids.iter().rev().copied() {
            if let Some(target) = reference(&rebuilt.records[n]) {
                references.entry(target).or_default().insert(n);
            }
            if let Some(guid) = guid(&rebuilt, &rebuilt.records[n]) {
                guids.entry(guid).or_default().insert(n);
            }
            hashes[n] = hash_record(&rebuilt, n, &hashes);
        }
        assert_eq!(s.references, references);
        assert_eq!(s.guids, guids);
        assert_eq!(s.current, root_hash(&rebuilt, &hashes));
        Document::decode(&rebuilt.encode()).unwrap();
    }

    #[test]
    fn structural_journal_matches_full_rebuild_across_batches_and_history() {
        let bytes = vec![
            2, 46, 0, 0, 0, 0, 2, 46, 1, 0, 0, 0, 10, 1, 0, 0, 0, 24, 7, 0, 0, 0, 5, 10, 1, 0, 0,
            0, 5,
        ];
        let mut w = Workspace::default();
        let id = w.open(&bytes).unwrap().document_id;
        let mut seed = 1729u64;
        for step in 0..250 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let s = w.session(id).unwrap();
            let mut ids = s.doc.reachable().unwrap();
            ids.sort_unstable();
            let node = ids[(seed >> 16) as usize % ids.len()];
            let op = match step % 7 {
                0 => Operation::Insert {
                    parent: 0,
                    index: 0,
                    name: None,
                    kind: "i32".into(),
                    value: step.to_string(),
                },
                1 => Operation::Remove { node },
                2 => Operation::Duplicate { node },
                3 => Operation::Move { node, index: 0 },
                4 => Operation::Retarget { node, target: 0 },
                5 => Operation::Set {
                    node,
                    tag: s.doc.records[node].tag,
                    value: step.to_string(),
                },
                _ => Operation::Template {
                    parent: 0,
                    index: 0,
                    name: None,
                    template: "game-res-atom".into(),
                    values: BTreeMap::new(),
                },
            };
            let mut operations = vec![op];
            // Force a mixture of valid batches, rollback after mutation, and dangling references.
            if step % 11 == 0 {
                operations.push(Operation::Remove { node: 0 });
            }
            let revision = s.revision;
            let mut candidate = s.doc.clone();
            let expected = operations
                .iter()
                .try_for_each(|op| apply(&mut candidate, op.clone()))
                .and_then(|_| candidate.rebuild());
            let original = s.doc.encode();
            let result = w.transact(id, revision, operations);
            assert_eq!(
                result.is_ok(),
                expected.is_ok(),
                "step {step}: {result:?}, {expected:?}"
            );
            assert_eq!(
                w.session(id).unwrap().doc.encode(),
                if expected.is_ok() {
                    candidate.encode()
                } else {
                    original
                }
            );
            assert_indexes(&w, id);
            if step % 3 == 0 && w.summary(id).unwrap().can_undo {
                let edited = w.session(id).unwrap().doc.encode();
                w.history(id, w.summary(id).unwrap().revision, false)
                    .unwrap();
                assert_indexes(&w, id);
                w.history(id, w.summary(id).unwrap().revision, true)
                    .unwrap();
                assert_eq!(w.session(id).unwrap().doc.encode(), edited);
                assert_indexes(&w, id);
            }
        }
    }

    #[test]
    fn depth_limit_rejects_new_container_and_restores_journal() {
        let mut bytes = [4, 46].repeat(256);
        bytes.extend([24, 1, 0, 0, 0]);
        bytes.extend(vec![5; 256]);
        let mut w = Workspace::default();
        let id = w.open(&bytes).unwrap().document_id;
        let insert = |kind: &str| Operation::Insert {
            parent: 255,
            index: 0,
            name: None,
            kind: kind.into(),
            value: "2".into(),
        };
        assert!(w
            .transact(id, 1, vec![insert("i32"), insert("array")])
            .is_err());
        assert_eq!(w.export(id).unwrap(), bytes);
        assert_eq!(w.summary(id).unwrap().revision, 1);
        w.transact(id, 1, vec![insert("i32")]).unwrap();
        Document::decode(&w.export(id).unwrap()).unwrap();
        w.history(id, 2, false).unwrap();
        assert_eq!(w.export(id).unwrap(), bytes);
    }

    #[test]
    fn summary_size_is_lazy_and_noop_preserves_history_and_saved_state() {
        let bytes = [4, 46, 24, 1, 0, 0, 0, 5];
        let mut w = Workspace::default();
        let id = w.open(&bytes).unwrap().document_id;
        assert_eq!(
            serde_json::to_value(w.summary(id).unwrap()).unwrap()["encodedBytes"],
            bytes.len()
        );
        let edit = |value: &str| {
            vec![Operation::Set {
                node: 1,
                tag: 24,
                value: value.into(),
            }]
        };
        let result = w.transact(id, 1, edit("2")).unwrap();
        assert!(result["summary"]["encodedBytes"].is_null());
        assert!(!w.summary(id).unwrap().can_redo);
        let edited = w.export(id).unwrap();
        assert_eq!(w.summary(id).unwrap().encoded_bytes, Some(edited.len()));
        w.mark_saved(id, 2).unwrap();
        assert!(!w.summary(id).unwrap().dirty);
        w.transact(id, 2, edit("2")).unwrap();
        assert_eq!(w.summary(id).unwrap().revision, 2);
        assert_eq!(w.summary(id).unwrap().encoded_bytes, Some(edited.len()));
        w.history(id, 2, false).unwrap();
        assert!(w.summary(id).unwrap().dirty);
        assert_eq!(w.export(id).unwrap(), bytes);
        w.transact(id, 3, edit("1")).unwrap();
        assert!(w.summary(id).unwrap().can_redo);
        w.history(id, 3, true).unwrap();
        assert!(!w.summary(id).unwrap().dirty);
    }
}
