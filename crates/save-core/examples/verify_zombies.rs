//! Read-only zombie discovery verification. The input save is never written.
use gk2_save_core::{Command, Workspace};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(std::env::args().nth(1).ok_or("save path")?)?;
    let mut workspace = Workspace::default();
    let summary = workspace.open(&bytes)?;
    let started = Instant::now();
    let value = match workspace.request(summary.document_id, Command::Zombies) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Zombie read failed: {error}");
            let mut status = workspace.request(
                summary.document_id,
                Command::SearchStart {
                    revision: summary.revision,
                    query: "name:gameResStr ancestor:ZombieWgoData".into(),
                    case_sensitive: false,
                },
            )?;
            let search_id = status["searchId"].as_u64().unwrap() as u32;
            while status["status"] == "searching" {
                status = workspace.request(
                    summary.document_id,
                    Command::SearchStep {
                        revision: summary.revision,
                        search_id,
                    },
                )?;
            }
            let page = workspace.request(
                summary.document_id,
                Command::SearchPage {
                    revision: summary.revision,
                    search_id,
                    offset: 0,
                    limit: 10,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&page)?);
            for result in page["results"].as_array().into_iter().flatten() {
                let node = result["node"].as_u64().unwrap() as usize;
                let children = workspace.request(
                    summary.document_id,
                    Command::Children {
                        parent: Some(node),
                        offset: 0,
                        limit: 200,
                    },
                )?;
                println!("{}", serde_json::to_string_pretty(&children)?);
                for child in children["nodes"].as_array().into_iter().flatten() {
                    let child_node = child["id"].as_u64().unwrap() as usize;
                    let grandchildren = workspace.request(
                        summary.document_id,
                        Command::Children {
                            parent: Some(child_node),
                            offset: 0,
                            limit: 200,
                        },
                    )?;
                    println!("{}", serde_json::to_string_pretty(&grandchildren)?);
                }
            }
            return Err(error.into());
        }
    };
    println!(
        "{} zombies; initial query: {:?}",
        value["zombies"].as_array().map_or(0, Vec::len),
        started.elapsed(),
    );
    for zombie in value["zombies"].as_array().into_iter().flatten() {
        println!(
            "{}: id={}, role={}, appearance={}",
            zombie["name"].as_str().unwrap_or("<unnamed>"),
            zombie["id"].as_str().unwrap_or("<unknown>"),
            zombie["role"].as_str().unwrap_or("<unknown>"),
            serde_json::to_string(&zombie["appearance"])?
        );
        if std::env::args().any(|arg| arg == "--dump-inventories") {
            let node = zombie["node"].as_u64().ok_or("zombie node")? as usize;
            let children = workspace.request(
                summary.document_id,
                Command::Children {
                    parent: Some(node),
                    offset: 0,
                    limit: 200,
                },
            )?;
            for child in children["nodes"].as_array().into_iter().flatten() {
                if matches!(
                    child["name"].as_str(),
                    Some("inventory" | "craftInventory" | "porterInventory" | "zombieItem")
                ) {
                    println!("  {}", serde_json::to_string(child)?);
                    let inventory_children = workspace.request(
                        summary.document_id,
                        Command::Children {
                            parent: Some(child["id"].as_u64().ok_or("inventory node")? as usize),
                            offset: 0,
                            limit: 20,
                        },
                    )?;
                    for inventory_child in
                        inventory_children["nodes"].as_array().into_iter().flatten()
                    {
                        println!("    {}", serde_json::to_string(inventory_child)?);
                        if inventory_child["name"] == "inventoryItem" {
                            let item_children = workspace.request(
                                summary.document_id,
                                Command::Children {
                                    parent: Some(
                                        inventory_child["id"]
                                            .as_u64()
                                            .ok_or("inventory item node")?
                                            as usize,
                                    ),
                                    offset: 0,
                                    limit: 20,
                                },
                            )?;
                            if let Some(list) = item_children["nodes"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .find(|node| node["name"] == "inventory")
                            {
                                println!("      {}", serde_json::to_string(list)?);
                                let list_children = workspace.request(
                                    summary.document_id,
                                    Command::Children {
                                        parent: Some(
                                            list["id"].as_u64().ok_or("inventory list")? as usize
                                        ),
                                        offset: 0,
                                        limit: 5,
                                    },
                                )?;
                                for list_child in
                                    list_children["nodes"].as_array().into_iter().flatten()
                                {
                                    println!("        {}", serde_json::to_string(list_child)?);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let started = Instant::now();
    let cached = workspace.request(summary.document_id, Command::Zombies)?;
    println!(
        "cached query: {:?}; identical: {}",
        started.elapsed(),
        cached == value
    );
    Ok(())
}
