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
fn text_value(name: &str, value: &str) -> Vec<u8> {
    let mut b = vec![39];
    b.extend(string(name));
    b.extend(string(value));
    b
}
fn object_node(name: &str, type_name: &str, object: i32, children: Vec<u8>) -> Vec<u8> {
    let mut b = vec![1];
    b.extend(string(name));
    b.push(47);
    b.extend(0i32.to_le_bytes());
    b.extend(string(type_name));
    b.extend(object.to_le_bytes());
    b.extend(children);
    b.push(5);
    b
}
fn internal_reference(name: &str, object: i32) -> Vec<u8> {
    let mut b = vec![9];
    b.extend(string(name));
    b.extend(object.to_le_bytes());
    b
}

fn search_count(workspace: &mut Workspace, id: u32, query: &str) -> u64 {
    let started = request(
        workspace,
        id,
        json!({"op":"search_start","revision":1,"query":query,"caseSensitive":false}),
    );
    let search_id = started["searchId"].as_u64().unwrap();
    request(
        workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":search_id}),
    )["discovered"]
        .as_u64()
        .unwrap()
}

#[test]
fn inspector_search_filters_pages_locations_and_invalidates_on_edit() {
    let mut count = vec![27];
    count.extend(string("count"));
    count.extend(9_223_372_036_854_775_000i64.to_le_bytes());
    let item = object_node(
        "entry",
        "Game.Items.Item, Assembly-CSharp",
        1,
        [text_value("id", "simple_iron_parts"), count].concat(),
    );
    let bytes = value_node(
        "",
        [
            value_node("playerInventory", item),
            value_node(
                "environmentData",
                text_value("timeOfDayPresetName", "indoor"),
            ),
            value_node(
                "worlds",
                [
                    text_value("worldId", "Prison"),
                    text_value("otherworldId", "Prison"),
                    text_value("worldId", "RuinedTemple"),
                ]
                .concat(),
            ),
        ]
        .concat(),
    );
    let mut workspace = Workspace::default();
    let id = workspace.open(&bytes).unwrap().document_id;
    let started = request(
        &mut workspace,
        id,
        json!({
            "op":"search_start",
            "revision":1,
            "query":"path:*playerInventory* ancestor:Item type:string name:id value:*iron*",
            "caseSensitive":false
        }),
    );
    let search_id = started["searchId"].as_u64().unwrap();
    let status = request(
        &mut workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":search_id}),
    );
    assert_eq!(status["status"], "complete");
    assert_eq!(status["discovered"], 1);
    let page = request(
        &mut workspace,
        id,
        json!({"op":"search_page","revision":1,"searchId":search_id,"offset":0,"limit":100}),
    );
    assert_eq!(page["results"][0]["name"], "id");
    assert!(page["results"][0]["path"]
        .as_str()
        .unwrap()
        .contains("playerInventory"));
    let node = page["results"][0]["node"].as_u64().unwrap();
    let location = request(
        &mut workspace,
        id,
        json!({"op":"node_location","revision":1,"node":node}),
    );
    assert_eq!(
        location["trail"].as_array().unwrap().last().unwrap()["node"],
        node
    );

    let numeric = request(
        &mut workspace,
        id,
        json!({"op":"search_start","revision":1,"query":"type:int value>9223372036854774999","caseSensitive":false}),
    );
    let numeric_id = numeric["searchId"].as_u64().unwrap();
    let numeric = request(
        &mut workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":numeric_id}),
    );
    assert_eq!(numeric["discovered"], 1);

    let ranked = request(
        &mut workspace,
        id,
        json!({"op":"search_start","revision":1,"query":"iron","caseSensitive":false}),
    );
    let ranked_id = ranked["searchId"].as_u64().unwrap();
    request(
        &mut workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":ranked_id}),
    );
    let ranked = request(
        &mut workspace,
        id,
        json!({"op":"search_page","revision":1,"searchId":ranked_id,"offset":0,"limit":100}),
    );
    assert!(ranked["results"].as_array().unwrap().len() >= 2);
    assert_eq!(ranked["results"][0]["name"], "id");
    assert_eq!(ranked["results"][0]["matchField"], "value");
    assert!(ranked["results"]
        .as_array()
        .unwrap()
        .iter()
        .any(|result| result["matchField"] == "path"));

    let exact = request(
        &mut workspace,
        id,
        json!({"op":"search_start","revision":1,"query":"name==worldId value==(Prison | RuinedTemple)","caseSensitive":false}),
    );
    let exact_id = exact["searchId"].as_u64().unwrap();
    let exact = request(
        &mut workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":exact_id}),
    );
    assert_eq!(exact["discovered"], 2);
    let exact_page = request(
        &mut workspace,
        id,
        json!({"op":"search_page","revision":1,"searchId":exact_id,"offset":0,"limit":100}),
    );
    assert!(exact_page["results"]
        .as_array()
        .unwrap()
        .iter()
        .all(|result| result["name"] == "worldId"));

    let substring = request(
        &mut workspace,
        id,
        json!({"op":"search_start","revision":1,"query":"name:worldId value:Prison","caseSensitive":false}),
    );
    let substring_id = substring["searchId"].as_u64().unwrap();
    let substring = request(
        &mut workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":substring_id}),
    );
    assert_eq!(substring["discovered"], 2);
    let substring_page = request(
        &mut workspace,
        id,
        json!({"op":"search_page","revision":1,"searchId":substring_id,"offset":0,"limit":100}),
    );
    assert!(substring_page["results"]
        .as_array()
        .unwrap()
        .iter()
        .any(|result| result["name"] == "otherworldId"));

    request(
        &mut workspace,
        id,
        json!({"op":"transact","revision":1,"operations":[{"op":"set","node":node,"tag":39,"value":"copper"}]}),
    );
    let stale: Command = serde_json::from_value(
        json!({"op":"search_page","revision":1,"searchId":ranked_id,"offset":0,"limit":100}),
    )
    .unwrap();
    assert!(workspace.request(id, stale).is_err());
}

#[test]
fn inspector_search_follows_reference_routes_and_stops_cycles() {
    let item = object_node(
        "storedItem",
        "Item, Assembly-CSharp",
        1,
        [text_value("id", "iron_part"), internal_reference("self", 1)].concat(),
    );
    let bytes = value_node("", [item, internal_reference("linkedItem", 1)].concat());
    let mut workspace = Workspace::default();
    let id = workspace.open(&bytes).unwrap().document_id;
    let started = request(
        &mut workspace,
        id,
        json!({"op":"search_start","revision":1,"query":"path:*linkedItem* ancestor:Item name:id value:*iron*","caseSensitive":false}),
    );
    let search_id = started["searchId"].as_u64().unwrap();
    let status = request(
        &mut workspace,
        id,
        json!({"op":"search_step","revision":1,"searchId":search_id}),
    );
    assert_eq!(status["status"], "complete");
    assert_eq!(status["discovered"], 1);
    assert!(status["visited"].as_u64().unwrap() < 20);
    let page = request(
        &mut workspace,
        id,
        json!({"op":"search_page","revision":1,"searchId":search_id,"offset":0,"limit":100}),
    );
    assert!(page["results"][0]["path"]
        .as_str()
        .unwrap()
        .contains("linkedItem"));
}

#[test]
fn inspector_search_matches_children_descendants_and_parents() {
    let bytes = value_node(
        "",
        [
            value_node("direct", text_value("worldId", "Prison")),
            value_node(
                "nested",
                value_node("details", text_value("worldId", "RuinedTemple")),
            ),
            value_node(
                "palace",
                [
                    text_value("id", "PalaceSewer"),
                    text_value("member", "gate"),
                ]
                .concat(),
            ),
        ]
        .concat(),
    );
    let mut workspace = Workspace::default();
    let id = workspace.open(&bytes).unwrap().document_id;

    assert_eq!(
        search_count(
            &mut workspace,
            id,
            "name==direct child:(name==worldId value==(Prison | RuinedTemple))"
        ),
        1
    );
    assert_eq!(
        search_count(
            &mut workspace,
            id,
            "name==nested child:(name==worldId value==RuinedTemple)"
        ),
        0
    );
    assert_eq!(
        search_count(
            &mut workspace,
            id,
            "name==nested descendant:(name==worldId value==RuinedTemple)"
        ),
        1
    );
    assert_eq!(
        search_count(
            &mut workspace,
            id,
            "name==member parent:(child:(name==id value==PalaceSewer))"
        ),
        1
    );
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
    // This fixture has no inventories, so only the item currencies are unavailable.
    assert!(fields
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["error"].is_null() != (f["key"] == "faith" || f["key"] == "science")));
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
