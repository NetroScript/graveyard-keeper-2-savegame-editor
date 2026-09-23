//! Read-only progression verification. The input save is never written.
use gk2_save_core::{progression, Command, Document, Operation, Workspace};

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
    workspace.request(
        summary.document_id,
        Command::Undo {
            revision: result["summary"]["revision"].as_u64().unwrap() as u32,
        },
    )?;
    assert_eq!(bytes, workspace.export(summary.document_id)?);
    println!("Inspiration insertion and undo verified");
    Ok(())
}
