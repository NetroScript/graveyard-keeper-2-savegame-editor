//! Read-only progression verification. The input save is never written.
use gk2_save_core::{progression, Command, Document, Operation, Workspace};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(std::env::args().nth(1).ok_or("save path")?)?;
    let mut workspace = Workspace::default();
    let summary = workspace.open(&bytes)?;
    let value = workspace.request(summary.document_id, Command::Progression)?;
    println!(
        "{} technologies, {} talent branches",
        value["unlockedTechnologies"].as_array().unwrap().len(),
        value["talents"].as_array().unwrap().len()
    );
    println!("{}", serde_json::to_string_pretty(&value)?);
    let result = workspace.request(
        summary.document_id,
        Command::Transact {
            revision: summary.revision,
            operations: vec![Operation::Progression {
                action: progression::Edit::SetInspiration {
                    talent: "talent_red".into(),
                    id: "editor_verification".into(),
                    current_value: "2".into(),
                    completion_goal_value: "3".into(),
                },
            }],
        },
    )?;
    Document::decode(&workspace.export(summary.document_id)?)?;
    let undone = workspace.request(
        summary.document_id,
        Command::Undo {
            revision: result["summary"]["revision"].as_u64().unwrap() as u32,
        },
    )?;
    assert_eq!(bytes, workspace.export(summary.document_id)?);
    println!("Inspiration insertion and undo verified");

    let mut revision = undone["summary"]["revision"].as_u64().unwrap() as u32;
    let mut transact =
        |workspace: &mut Workspace, action| -> Result<Value, Box<dyn std::error::Error>> {
            let result = workspace.request(
                summary.document_id,
                Command::Transact {
                    revision,
                    operations: vec![Operation::Progression { action }],
                },
            )?;
            revision = result["summary"]["revision"].as_u64().unwrap() as u32;
            Ok(workspace.request(summary.document_id, Command::Progression)?)
        };

    // Unlocking and locking the same technology restores the original bytes.
    transact(
        &mut workspace,
        progression::Edit::UnlockTechnologies {
            ids: vec!["editor_verification".into()],
            rewards: progression::TechnologyRewards {
                crafts: vec!["editor_verification_craft".into()],
                alchemy_formulas: vec![],
                buildings: vec![],
                town_buildings: vec![],
                perks: vec![progression::PerkUnlock {
                    id: "editor_verification_perk".into(),
                    duration: "0".into(),
                }],
            },
            costs: costs("editor_verification", 5, 5, 5),
        },
    )?;
    let unlocked = transact(
        &mut workspace,
        progression::Edit::UnlockTalentLevels {
            talent: "talent_red".into(),
            levels: vec![progression::TalentUnlock {
                id: "editor_verification_level".into(),
                talent_value: 3,
                point_price: 0,
            }],
        },
    )?;
    assert!(unlocked["activePerks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == "editor_verification_perk"));
    transact(
        &mut workspace,
        progression::Edit::LockTalentLevels {
            talent: "talent_red".into(),
            levels: vec![progression::TalentUnlock {
                id: "editor_verification_level".into(),
                talent_value: 3,
                point_price: 0,
            }],
            perks: vec![],
        },
    )?;
    // Editor unlocks were free, so locking them refunds nothing even with a price.
    transact(
        &mut workspace,
        progression::Edit::LockTechnologies {
            ids: vec!["editor_verification".into()],
            rewards: progression::TechnologyLocks {
                crafts: vec!["editor_verification_craft".into()],
                alchemy_formulas: vec![],
                buildings: vec![],
                town_buildings: vec![],
                perks: vec!["editor_verification_perk".into()],
            },
            costs: costs("editor_verification", 5, 5, 5),
        },
    )?;
    assert_eq!(bytes, workspace.export(summary.document_id)?);
    println!("Free technology, perk and talent level unlock/lock round trip verified");

    // Locking progression from the opened save refunds it, re-unlocking charges it again,
    // and undo restores the original save.
    let technologies: Vec<String> = serde_json::from_value(value["unlockedTechnologies"].clone())?;
    let branch = value["talents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| !b["studiedLevelUps"].as_array().unwrap().is_empty())
        .ok_or("no studied talent levels")?;
    let talent = branch["id"].as_str().unwrap().to_string();
    let level = branch["studiedLevelUps"][0].as_str().unwrap().to_string();
    let mastery: i32 = branch["curTalentValue"].as_str().unwrap().parse()?;
    let perk_points: i32 = branch["talentExpPoints"].as_str().unwrap().parse()?;
    let tech_points = |workspace: &mut Workspace| -> Result<[f64; 3], Box<dyn std::error::Error>> {
        let general = workspace.request(summary.document_id, Command::General)?;
        let value = |key: &str| -> f64 {
            general
                .as_array()
                .unwrap()
                .iter()
                .find(|field| field["key"] == key)
                .and_then(|field| field["value"].as_str())
                .unwrap_or("0")
                .parse()
                .unwrap()
        };
        Ok([value("tech_red"), value("tech_green"), value("tech_blue")])
    };
    let points_before = tech_points(&mut workspace)?;
    let perk = value["activePerks"][0].as_str().map(String::from);
    let locked_tech = technologies
        .last()
        .ok_or("no unlocked technologies")?
        .clone();
    let lock_tech = |perks: Vec<String>| progression::Edit::LockTechnologies {
        ids: vec![locked_tech.clone()],
        rewards: progression::TechnologyLocks {
            crafts: vec![],
            alchemy_formulas: vec![],
            buildings: vec![],
            town_buildings: vec![],
            perks,
        },
        costs: costs(&locked_tech, 2, 1, 0),
    };
    let unlock_tech = |red| progression::Edit::UnlockTechnologies {
        ids: vec![locked_tech.clone()],
        rewards: progression::TechnologyRewards {
            crafts: vec![],
            alchemy_formulas: vec![],
            buildings: vec![],
            town_buildings: vec![],
            perks: vec![],
        },
        costs: costs(&locked_tech, red, 1, 0),
    };
    let delta = |workspace: &mut Workspace| -> Result<[f64; 3], Box<dyn std::error::Error>> {
        let now = tech_points(workspace)?;
        Ok([
            now[0] - points_before[0],
            now[1] - points_before[1],
            now[2] - points_before[2],
        ])
    };
    let mut edits = 0;
    let after = transact(&mut workspace, lock_tech(perk.iter().cloned().collect()))?;
    edits += 1;
    assert!(!after["unlockedTechnologies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == locked_tech.as_str()));
    assert_eq!(delta(&mut workspace)?, [2.0, 1.0, 0.0]);
    // Re-unlocking without enough points is rejected; with the real price it is charged.
    assert!(transact(&mut workspace, unlock_tech(1_000_000_000)).is_err());
    transact(&mut workspace, unlock_tech(2))?;
    edits += 1;
    assert_eq!(delta(&mut workspace)?, [0.0, 0.0, 0.0]);
    // Lock, unlock and lock again refunds the price exactly once.
    transact(&mut workspace, lock_tech(vec![]))?;
    edits += 1;
    assert_eq!(delta(&mut workspace)?, [2.0, 1.0, 0.0]);
    let level_edit = |price| progression::TalentUnlock {
        id: level.clone(),
        talent_value: 1,
        point_price: price,
    };
    let perk_points_now = |value: &Value| -> i32 {
        value["talents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["id"] == talent.as_str())
            .unwrap()["talentExpPoints"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    };
    let after = transact(
        &mut workspace,
        progression::Edit::LockTalentLevels {
            talent: talent.clone(),
            levels: vec![level_edit(3)],
            perks: vec![],
        },
    )?;
    edits += 1;
    assert_eq!(perk_points_now(&after), perk_points + 3);
    let relocked = transact(
        &mut workspace,
        progression::Edit::UnlockTalentLevels {
            talent: talent.clone(),
            levels: vec![level_edit(3)],
        },
    )?;
    edits += 1;
    assert_eq!(perk_points_now(&relocked), perk_points);
    let after = transact(
        &mut workspace,
        progression::Edit::LockTalentLevels {
            talent: talent.clone(),
            levels: vec![level_edit(3)],
            perks: vec![],
        },
    )?;
    edits += 1;
    let branch = after["talents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == talent.as_str())
        .unwrap();
    assert!(!branch["studiedLevelUps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == level.as_str()));
    assert_eq!(
        branch["curTalentValue"].as_str().unwrap(),
        (mastery - 1).max(0).to_string()
    );
    assert_eq!(
        branch["talentExpPoints"].as_str().unwrap(),
        (perk_points + 3).to_string()
    );
    if let Some(perk) = &perk {
        assert!(!after["activePerks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == perk.as_str()));
    }
    Document::decode(&workspace.export(summary.document_id)?)?;
    for _ in 0..edits {
        let result = workspace.request(summary.document_id, Command::Undo { revision })?;
        revision = result["summary"]["revision"].as_u64().unwrap() as u32;
    }
    assert_eq!(bytes, workspace.export(summary.document_id)?);
    println!(
        "Locked technology {locked_tech}, {talent} level {level} and active perk {perk:?}; refunds, charges and undo verified"
    );
    Ok(())
}

fn costs(
    id: &str,
    red: u32,
    green: u32,
    blue: u32,
) -> std::collections::BTreeMap<String, progression::TechnologyPoints> {
    std::collections::BTreeMap::from([(
        id.to_string(),
        progression::TechnologyPoints { red, green, blue },
    )])
}
