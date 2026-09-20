//! Read-only native edit benchmark. All changes and exports stay in memory.
use gk2_save_core::{Command, Workspace};
use serde_json::{json, Value};
use std::time::Instant;

fn request(w: &mut Workspace, id: u32, value: Value) -> Value {
    w.request(id, serde_json::from_value(value).unwrap())
        .unwrap()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: benchmark_edits <save.dat> [--catalog-stdin]")?;
    let bytes = std::fs::read(path)?;
    let mut w = Workspace::default();
    let start = Instant::now();
    let id = w.open(&bytes)?.document_id;
    println!("Open: {:.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    let catalog: Value = if std::env::args().any(|a| a == "--catalog-stdin") {
        serde_json::from_reader(std::io::stdin())?
    } else {
        json!({"items":{}})
    };
    request(
        &mut w,
        id,
        json!({"op":"inventory_catalog","catalog":catalog}),
    );
    let start = Instant::now();
    let snapshot = request(&mut w, id, json!({"op":"inventories"}));
    println!(
        "Discover inventories: {:.2} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
    let player = snapshot["inventories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["kind"] == "Player")
        .unwrap();
    let container = player["node"].as_u64().unwrap();
    let item = player["items"][0]["node"].as_u64().unwrap();
    let capacity: i32 = player["capacity"].as_str().unwrap().parse()?;
    let inventory = |action| json!({"op":"inventory","container":container,"out_of_bounds":true,"action":action});
    let mut scenarios = vec![
        (
            "Delete item",
            vec![inventory(json!({"kind":"remove","node":item}))],
        ),
        (
            "Capacity",
            vec![inventory(
                json!({"kind":"capacity","value":(capacity+1).to_string()}),
            )],
        ),
        (
            "Inspector duplicate",
            vec![json!({"op":"duplicate","node":item})],
        ),
        ("Inspector remove", vec![json!({"op":"remove","node":item})]),
    ];
    if catalog["items"].get("cheese").is_some() {
        scenarios.push(("Insert item", vec![
            inventory(json!({"kind":"capacity","value":(capacity+1).to_string()})),
            inventory(json!({"kind":"put","node":null,"item":"cheese","count":"1","guid":"11111111-1111-4111-8111-111111111111"})),
        ]));
        let old_id = player["items"][0]["id"].as_str().unwrap();
        if catalog["items"].get(old_id).is_some() {
            scenarios.push((
                "Item count",
                vec![inventory(
                    json!({"kind":"put","node":item,"item":old_id,"count":"2","guid":"unused"}),
                )],
            ));
        }
        let replacement = if old_id == "cheese" {
            "mushroom_brown"
        } else {
            "cheese"
        };
        if catalog["items"].get(replacement).is_some() {
            scenarios.push(("Replace item", vec![inventory(json!({"kind":"put","node":item,"item":replacement,"count":"1","guid":"22222222-2222-4222-8222-222222222222"}))]));
        }
    }
    let general = request(&mut w, id, json!({"op":"general"}));
    if let Some(hp) = general
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["key"] == "hp" && v["error"].is_null())
    {
        let value = hp["value"].as_str().unwrap().parse::<i32>()? + 1;
        scenarios.push((
            "General health",
            vec![json!({"op":"general","values":{"hp":value.to_string()}})],
        ));
    }
    for (label, operations) in scenarios {
        // A raw Inspector edit deliberately invalidates the inventory view. Refresh outside edit timing.
        request(&mut w, id, json!({"op":"inventories"}));
        let mut times = [Vec::new(), Vec::new(), Vec::new()];
        for _ in 0..10 {
            for (index, op) in ["transact", "undo", "redo", "undo"].iter().enumerate() {
                let revision = w.summary(id)?.revision;
                let command: Command = serde_json::from_value(if *op == "transact" {
                    json!({"op":op,"revision":revision,"operations":operations})
                } else {
                    json!({"op":op,"revision":revision})
                })?;
                let start = Instant::now();
                let result = w.request(id, command)?;
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                if index < 3 {
                    times[index].push(ms);
                }
                assert!(result["summary"]["encodedBytes"].is_null());
            }
            assert!(!w.summary(id)?.dirty);
        }
        for (suffix, mut samples) in ["edit", "undo", "redo"].into_iter().zip(times) {
            samples.sort_by(f64::total_cmp);
            println!(
                "{label} {suffix}: median {:.3} ms, max {:.3} ms",
                samples[5], samples[9]
            );
        }
    }
    let start = Instant::now();
    assert_eq!(w.export(id)?, bytes);
    assert_eq!(w.summary(id)?.encoded_bytes, Some(bytes.len()));
    println!(
        "Validated export: {:.2} ms; all edits undone byte-identically. No files written.",
        start.elapsed().as_secs_f64() * 1000.0
    );
    Ok(())
}
