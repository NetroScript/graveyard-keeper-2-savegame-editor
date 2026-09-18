use gk2_save_core::{Command, Document, Workspace};
use serde_json::{json, Value};
fn fixture() -> Vec<u8> {
    let mut b = vec![2, 46, 0, 0, 0, 0, 6];
    b.extend(1i64.to_le_bytes());
    b.extend([24, 7, 0, 0, 0, 7, 5]);
    b
}
fn request(w: &mut Workspace, id: u32, v: Value) -> Value {
    w.request(id, serde_json::from_value(v).unwrap()).unwrap()
}
#[test]
fn independent_atomic_history() {
    let bytes = fixture();
    let mut w = Workspace::default();
    let a = w.open(&bytes).unwrap().document_id;
    let b = w.open(&bytes).unwrap().document_id;
    request(
        &mut w,
        a,
        json!({"op":"transact","revision":1,"operations":[{"op":"set","node":2,"tag":24,"value":"9"}]}),
    );
    assert_eq!(w.export(b).unwrap(), bytes);
    assert_ne!(w.export(a).unwrap(), bytes);
    let before = w.export(a).unwrap();
    for revision in [1, 2] {
        let cmd=serde_json::from_value(json!({"op":"transact","revision":revision,"operations":[{"op":"set","node":2,"tag":24,"value":"10"},{"op":"remove","node":0}]})).unwrap();
        assert!(w.request(a, cmd).is_err());
        assert_eq!(w.export(a).unwrap(), before);
    }
    request(&mut w, a, json!({"op":"undo","revision":2}));
    assert_eq!(w.export(a).unwrap(), bytes);
    request(&mut w, a, json!({"op":"redo","revision":3}));
    assert_eq!(w.export(a).unwrap(), before);
    w.request(a, Command::Close).unwrap();
    assert!(w.export(a).is_err());
    assert_eq!(w.export(b).unwrap(), bytes);
}
#[test]
fn array_insert_duplicate_remove_and_undo() {
    let bytes = fixture();
    let mut w = Workspace::default();
    let id = w.open(&bytes).unwrap().document_id;
    request(
        &mut w,
        id,
        json!({"op":"transact","revision":1,"operations":[{"op":"duplicate","node":1},{"op":"insert","parent":1,"index":1,"name":null,"kind":"i32","value":"42"}]}),
    );
    Document::decode(&w.export(id).unwrap()).unwrap();
    request(&mut w, id, json!({"op":"undo","revision":2}));
    assert_eq!(w.export(id).unwrap(), bytes);
    request(
        &mut w,
        id,
        json!({"op":"transact","revision":3,"operations":[{"op":"insert","parent":1,"index":1,"name":null,"kind":"i32","value":"15"}]}),
    );
    Document::decode(&w.export(id).unwrap()).unwrap();
}
#[test]
fn reject_dangling_reference() {
    let bytes = vec![
        2, 46, 0, 0, 0, 0, 2, 46, 1, 0, 0, 0, 10, 1, 0, 0, 0, 5, 10, 1, 0, 0, 0, 5,
    ];
    let mut w = Workspace::default();
    let id = w.open(&bytes).unwrap().document_id;
    let cmd = serde_json::from_value(
        json!({"op":"transact","revision":1,"operations":[{"op":"remove","node":1}]}),
    )
    .unwrap();
    assert!(w.request(id, cmd).is_err());
    assert_eq!(w.export(id).unwrap(), bytes);
    request(
        &mut w,
        id,
        json!({"op":"transact","revision":1,"operations":[{"op":"remove","node":1},{"op":"retarget","node":4,"target":0}]}),
    );
    Document::decode(&w.export(id).unwrap()).unwrap();
    request(&mut w, id, json!({"op":"undo","revision":2}));
    assert_eq!(w.export(id).unwrap(), bytes);
}
fn string(s: &str) -> Vec<u8> {
    let mut b = vec![0];
    b.extend((s.len() as i32).to_le_bytes());
    b.extend(s.as_bytes());
    b
}
fn value_node(name: &str, children: Vec<u8>) -> Vec<u8> {
    let mut b = vec![if name.is_empty() { 4 } else { 3 }];
    if !name.is_empty() {
        b.extend(string(name));
    }
    b.push(46);
    b.extend(children);
    b.push(5);
    b
}
#[test]
fn clone_cycles_shared_references_and_relocate_type_declaration() {
    let mut child = vec![2, 47];
    child.extend(0i32.to_le_bytes());
    child.extend(string("Example, Tests"));
    child.extend(1i32.to_le_bytes());
    child.extend([10, 1, 0, 0, 0, 10, 0, 0, 0, 0, 5]);
    let second = vec![2, 48, 0, 0, 0, 0, 2, 0, 0, 0, 10, 1, 0, 0, 0, 5];
    let mut bytes = vec![2, 46, 0, 0, 0, 0];
    bytes.extend(child);
    bytes.extend(second);
    bytes.push(5);
    let mut w = Workspace::default();
    let id = w.open(&bytes).unwrap().document_id;
    let roots = request(
        &mut w,
        id,
        json!({"op":"children","parent":0,"offset":0,"limit":100}),
    );
    let first = roots["nodes"][0]["id"].as_u64().unwrap();
    let second = roots["nodes"][1]["id"].as_u64().unwrap();
    request(
        &mut w,
        id,
        json!({"op":"transact","revision":1,"operations":[{"op":"duplicate","node":first}]}),
    );
    let siblings = request(
        &mut w,
        id,
        json!({"op":"children","parent":0,"offset":0,"limit":100}),
    );
    let clone = siblings["nodes"][1]["id"].as_u64().unwrap();
    let refs = request(
        &mut w,
        id,
        json!({"op":"children","parent":clone,"offset":0,"limit":100}),
    );
    assert_eq!(refs["nodes"][0]["referenceTarget"], clone);
    assert_eq!(refs["nodes"][1]["referenceTarget"], 0);
    Document::decode(&w.export(id).unwrap()).unwrap();
    request(&mut w, id, json!({"op":"undo","revision":2}));
    assert_eq!(w.export(id).unwrap(), bytes);
    let refs = request(
        &mut w,
        id,
        json!({"op":"children","parent":second,"offset":0,"limit":100}),
    );
    let reference = refs["nodes"][0]["id"].as_u64().unwrap();
    request(
        &mut w,
        id,
        json!({"op":"transact","revision":3,"operations":[{"op":"remove","node":first},{"op":"retarget","node":reference,"target":0}]}),
    );
    let edited = w.export(id).unwrap();
    Document::decode(&edited).unwrap();
    assert!(edited
        .windows(b"Example, Tests".len())
        .any(|w| w == b"Example, Tests"));
    request(&mut w, id, json!({"op":"undo","revision":4}));
    assert_eq!(w.export(id).unwrap(), bytes);
}
#[test]
fn general_resource_insert_float_precision_and_malformed_mapping() {
    let mut hp = vec![23];
    hp.extend(string("hp"));
    hp.extend(75i32.to_le_bytes());
    hp.push(23);
    hp.extend(string("maxHpValue"));
    hp.extend(100i32.to_le_bytes());
    let array = vec![6, 0, 0, 0, 0, 0, 0, 0, 0, 7];
    let res = [
        value_node("resType", array.clone()),
        value_node("resValues", array),
    ]
    .concat();
    let bytes = value_node(
        "",
        value_node(
            "playerData",
            [value_node("hpComponent", hp), value_node("res", res)].concat(),
        ),
    );
    let mut w = Workspace::default();
    let id = w.open(&bytes).unwrap().document_id;
    request(
        &mut w,
        id,
        json!({"op":"transact","revision":1,"operations":[{"op":"general","values":{"money":"12345","energy":"0.1","hp":"110"}}]}),
    );
    let fields = request(&mut w, id, json!({"op":"general"}));
    assert!(fields
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["error"].is_null()));
    let energy = fields
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == "energy")
        .unwrap();
    assert_eq!(
        energy["value"]
            .as_str()
            .unwrap()
            .parse::<f32>()
            .unwrap()
            .to_bits(),
        0.1f32.to_bits()
    );
    let encoded = w.export(id).unwrap();
    let reopened = w.open(&encoded).unwrap().document_id;
    assert_eq!(
        request(&mut w, reopened, json!({"op":"general"}))
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["key"] == "money")
            .unwrap()["value"],
        "12345"
    );
    let invalid=serde_json::from_value(json!({"op":"transact","revision":2,"operations":[{"op":"general","values":{"money":"55","max_hp":"0"}}]})).unwrap();
    assert!(w.request(id, invalid).is_err());
    assert_eq!(w.export(id).unwrap(), encoded);
    request(&mut w, id, json!({"op":"undo","revision":2}));
    assert_eq!(w.export(id).unwrap(), bytes);
}
