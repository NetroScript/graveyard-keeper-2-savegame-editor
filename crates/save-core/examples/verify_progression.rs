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
        },
    )?;
    let unlocked = transact(
        &mut workspace,
        progression::Edit::UnlockTalentLevels {
            talent: "talent_red".into(),
            levels: vec![progression::TalentUnlock {
                id: "editor_verification_level".into(),
                talent_value: 3,
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
            }],
            perks: vec![],
        },
    )?;
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
        },
    )?;
    assert_eq!(bytes, workspace.export(summary.document_id)?);
    println!("Technology, perk and talent level unlock/lock round trip verified");

    // Locking existing progression removes it, and undo restores the original save.
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
    let perk = value["activePerks"][0].as_str().map(String::from);
    let locked_tech = technologies
        .last()
        .ok_or("no unlocked technologies")?
        .clone();
    let after = transact(
        &mut workspace,
        progression::Edit::LockTechnologies {
            ids: vec![locked_tech.clone()],
            rewards: progression::TechnologyLocks {
                crafts: vec![],
                alchemy_formulas: vec![],
                buildings: vec![],
                town_buildings: vec![],
                perks: perk.iter().cloned().collect(),
            },
        },
    )?;
    assert!(!after["unlockedTechnologies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id == locked_tech.as_str()));
    let after = transact(
        &mut workspace,
        progression::Edit::LockTalentLevels {
            talent: talent.clone(),
            levels: vec![progression::TalentUnlock {
                id: level.clone(),
                talent_value: 1,
            }],
            perks: vec![],
        },
    )?;
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
    if let Some(perk) = &perk {
        assert!(!after["activePerks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == perk.as_str()));
    }
    Document::decode(&workspace.export(summary.document_id)?)?;
    for _ in 0..2 {
        let result = workspace.request(summary.document_id, Command::Undo { revision })?;
        revision = result["summary"]["revision"].as_u64().unwrap() as u32;
    }
    assert_eq!(bytes, workspace.export(summary.document_id)?);
    println!(
        "Locked technology {locked_tech}, {talent} level {level} and active perk {perk:?}; undo verified"
    );
    Ok(())
}
