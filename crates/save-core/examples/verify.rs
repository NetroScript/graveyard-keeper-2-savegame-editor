//! Read-only fixture verification. Never writes or replaces an input save.
use gk2_save_core::{Document, EditRequest, Request, Response, Service};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("Usage: verify <save.dat>")?;
    let bytes = std::fs::read(path)?;
    let document = Document::decode(&bytes)?;
    let encoded = document.encode();
    assert_eq!(bytes.len(), encoded.len());
    if let Some(offset) = bytes.iter().zip(&encoded).position(|(a, b)| a != b) {
        return Err(format!("Mismatch at {offset}").into());
    }
    println!(
        "Exact round trip: {} bytes, {} records, {} types, {} objects",
        bytes.len(),
        document.record_count(),
        document.type_count(),
        document.object_count()
    );
    let mut service = Service::default();
    let summary = service.open(&bytes)?;
    let Response::Children(roots) = service.request(Request::Children {
        parent: None,
        offset: 0,
        limit: 200,
    })?
    else {
        unreachable!()
    };
    let Response::Children(fields) = service.request(Request::Children {
        parent: Some(roots.nodes[0].id),
        offset: 0,
        limit: 200,
    })?
    else {
        unreachable!()
    };
    for node in &fields.nodes {
        println!(
            "{}: {:?} {} {:?}",
            node.id, node.name, node.kind, node.type_name
        );
    }
    let field = fields
        .nodes
        .iter()
        .find(|n| n.kind == "bool")
        .ok_or("No root boolean for edit probe")?;
    let original = field.value.clone().unwrap();
    let edited = if original == "true" { "false" } else { "true" }.to_string();
    service.request(Request::SetValue {
        edit: EditRequest {
            node: field.id,
            expected_tag: field.tag,
            revision: summary.revision,
            value: edited,
        },
    })?;
    let changed = service.export()?;
    Document::decode(&changed)?;
    assert_eq!(
        bytes.iter().zip(&changed).filter(|(a, b)| a != b).count(),
        1
    );
    let revision = service.summary()?.revision;
    service.request(Request::SetValue {
        edit: EditRequest {
            node: field.id,
            expected_tag: field.tag,
            revision,
            value: original,
        },
    })?;
    assert_eq!(bytes, service.export()?);
    println!("In-memory scalar edit changed one byte; restoring it recovered the original bytes. No files written.");
    Ok(())
}
