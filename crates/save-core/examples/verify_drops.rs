//! Read-only drop cleanup verification. The input save is never written.
use gk2_save_core::{drops, Command, Document, Operation, Workspace};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: verify_drops <save.dat>")?;
    let bytes = std::fs::read(path)?;
    let mut workspace = Workspace::default();
    let summary = workspace.open(&bytes)?;
    let query_started = Instant::now();
    let snapshot = workspace.request(summary.document_id, Command::Drops)?;
    let query_elapsed = query_started.elapsed();
    let drops = snapshot["drops"]
        .as_array()
        .ok_or("Drop query did not return an array")?;
    println!(
        "Found {} ordinary world drops in {:.2?}",
        drops.len(),
        query_elapsed
    );

    let nodes = drops
        .iter()
        .map(|drop| {
            drop["node"]
                .as_u64()
                .map(|node| node as usize)
                .ok_or("Drop query returned an invalid node")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !nodes.is_empty() {
        let edit_started = Instant::now();
        let result = workspace.request(
            summary.document_id,
            Command::Transact {
                revision: summary.revision,
                operations: vec![Operation::Drops {
                    action: drops::Edit::Remove { nodes },
                }],
            },
        )?;
        let edit_elapsed = edit_started.elapsed();
        let changed = workspace.export(summary.document_id)?;
        Document::decode(&changed)?;
        let revision = result["summary"]["revision"]
            .as_u64()
            .ok_or("Transaction did not return a revision")? as u32;
        workspace.request(summary.document_id, Command::Undo { revision })?;
        assert_eq!(bytes, workspace.export(summary.document_id)?);
        println!(
            "Deleted all drops in {:.2?}; undo restored the original bytes",
            edit_elapsed
        );
    }
    Ok(())
}
