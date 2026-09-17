use gk2_save_core::{Document, EditRequest, Limits, Request, Response, Service};

fn string(value: &[u8], wide: bool) -> Vec<u8> {
    let mut bytes = vec![u8::from(wide)];
    bytes.extend_from_slice(&((value.len() / if wide { 2 } else { 1 }) as i32).to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}
fn root(children: &[u8]) -> Vec<u8> {
    let mut bytes = vec![2, 47];
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend(string(b"FutureSave, FutureAssembly", false));
    bytes.extend_from_slice(&0i32.to_le_bytes());
    bytes.extend(children);
    bytes.push(5);
    bytes
}
fn round_trip(bytes: &[u8]) {
    assert_eq!(Document::decode(bytes).unwrap().encode(), bytes);
}

#[test]
fn all_scalar_tokens_names_and_references_are_lossless() {
    let mut children = vec![];
    for tag in 9u8..=46 {
        children.push(tag);
        if tag % 2 == 1 {
            children.extend(string(b"duplicate-name", false));
        }
        let base = if tag % 2 == 1 { tag + 1 } else { tag };
        let width = match base {
            10 | 12 | 24 | 26 | 32 => 4,
            14 | 36 | 42 => 16,
            16 | 18 | 44 => 1,
            20 | 22 | 38 => 2,
            28 | 30 | 34 => 8,
            40 => {
                children.extend(string(&[0, 0xd8], true));
                continue;
            } // unpaired surrogate
            46 => 0,
            _ => unreachable!(),
        };
        children.extend(vec![0; width]); // references to the root (cycle)
    }
    for tag in [50, 51] {
        children.push(tag);
        if tag == 50 {
            children.extend(string(b"external", false));
        }
        children.extend(string(b"asset-key", false));
    }
    round_trip(&root(&children));
}

#[test]
fn arrays_type_references_and_terminator() {
    let mut children = vec![6];
    children.extend_from_slice(&2i64.to_le_bytes());
    children.extend([4, 48]);
    children.extend_from_slice(&0i32.to_le_bytes());
    children.push(5);
    children.extend([4, 46, 8]);
    children.extend_from_slice(&2i32.to_le_bytes());
    children.extend_from_slice(&4i32.to_le_bytes());
    children.extend_from_slice(&[0xff; 8]);
    children.extend([5, 7]);
    let mut bytes = root(&children);
    bytes.push(49);
    round_trip(&bytes);
}

#[test]
fn float_bits_and_string_encodings_survive() {
    for bits in [0x80000000u32, 0x7fc01234, 0xff800000] {
        let mut bytes = vec![32];
        bytes.extend_from_slice(&bits.to_le_bytes());
        round_trip(&bytes);
    }
    for (value, wide) in [
        (vec![0xe9, 0], false),
        (vec![0xe9, 0, 0, 0], true),
        (vec![0x3d, 0xd8, 0, 0xde], true),
    ] {
        let mut bytes = vec![40];
        bytes.extend(string(&value, wide));
        round_trip(&bytes);
    }
}

#[test]
fn truncated_and_malformed_files_are_rejected() {
    let bytes = root(&[24, 1, 0, 0, 0]);
    for end in 0..bytes.len() {
        assert!(Document::decode(&bytes[..end]).is_err(), "prefix {end}");
    }
    for invalid in [
        vec![0],
        vec![255],
        vec![5],
        vec![4, 46, 7],
        vec![4, 46],
        vec![40, 2, 0, 0, 0, 0],
        vec![40, 0, 255, 255, 255, 255],
        vec![10, 1, 0, 0, 0],
        vec![44, 2],
        vec![46, 46],
        vec![49],
        vec![46, 49, 0],
        vec![4, 48, 0, 0, 0, 0, 5],
    ] {
        assert!(Document::decode(&invalid).is_err(), "{invalid:?}");
    }
    let mut count_mismatch = vec![6];
    count_mismatch.extend_from_slice(&1i64.to_le_bytes());
    count_mismatch.push(7);
    assert!(Document::decode(&count_mismatch).is_err());
    assert!(Document::decode_with_limits(
        &bytes,
        Limits {
            max_bytes: 2,
            ..Limits::default()
        }
    )
    .is_err());
    assert!(Document::decode_with_limits(
        &bytes,
        Limits {
            max_records: 1,
            ..Limits::default()
        }
    )
    .is_err());
    assert!(Document::decode_with_limits(
        &bytes,
        Limits {
            max_depth: 0,
            ..Limits::default()
        }
    )
    .is_err());
}

fn edit(
    service: &mut Service,
    node: usize,
    tag: u8,
    value: &str,
) -> Result<Response, gk2_save_core::Error> {
    service.request(Request::SetValue {
        edit: EditRequest {
            node,
            expected_tag: tag,
            revision: service.summary().unwrap().revision,
            value: value.into(),
        },
    })
}

#[test]
fn variable_length_edit_and_64_bit_values_are_exact() {
    // Use an actual UTF-16 string rather than relying on UTF-8 conversion in the codec.
    let mut children = vec![40];
    children.extend(string(&[b'a', 0, b'b', 0], true));
    children.push(30);
    children.extend_from_slice(&u64::MAX.to_le_bytes());
    let bytes = root(&children);
    let mut service = Service::default();
    service.open(&bytes).unwrap();
    edit(&mut service, 1, 40, "longer 🪦 name").unwrap();
    let output = service.export().unwrap();
    round_trip(&output);
    assert_ne!(output, bytes);
    edit(&mut service, 1, 40, "ab").unwrap();
    assert_eq!(service.export().unwrap(), bytes);
    assert!(edit(&mut service, 2, 30, "18446744073709551616").is_err());
    edit(&mut service, 2, 30, "18446744073709551614").unwrap();
    let Response::Children(page) = service
        .request(Request::Children {
            parent: Some(0),
            offset: 1,
            limit: 1,
        })
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(page.total, 2);
    assert_eq!(page.nodes[0].value.as_deref(), Some("18446744073709551614"));
}

#[test]
fn failed_operations_preserve_document_and_revisions() {
    let bytes = root(&[44, 0]);
    let mut service = Service::default();
    let revision = service.open(&bytes).unwrap().revision;
    assert!(service.open(&[255]).is_err());
    assert!(edit(&mut service, 1, 44, "yes").is_err());
    assert!(edit(&mut service, 1, 24, "1").is_err());
    assert!(edit(&mut service, 0, 2, "root").is_err());
    assert_eq!(service.summary().unwrap().revision, revision);
    assert_eq!(service.export().unwrap(), bytes);
    edit(&mut service, 1, 44, "true").unwrap();
    assert!(service
        .request(Request::SetValue {
            edit: EditRequest {
                node: 1,
                expected_tag: 44,
                revision,
                value: "false".into()
            }
        })
        .is_err());
    service.request(Request::Close).unwrap();
    assert!(service.export().is_err());
}

#[test]
fn deterministic_malformed_corpus_never_panics() {
    let mut seed = 0x12345678u32;
    for length in 0..512 {
        let bytes: Vec<_> = (0..length)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        let _ = Document::decode(&bytes);
    }
}

#[test]
fn transport_json_contract() {
    let mut service = Service::default();
    service.open(&root(&[44, 0])).unwrap();
    let request: Request =
        serde_json::from_str(r#"{"op":"children","parent":0,"offset":0,"limit":10}"#).unwrap();
    let json = serde_json::to_value(service.request(request).unwrap()).unwrap();
    assert_eq!(json["data"]["nodes"][0]["tag"], 44);
    assert_eq!(json["data"]["nodes"][0]["value"], "false");
    let request: Request = serde_json::from_str(
        r#"{"op":"set_value","edit":{"node":1,"expectedTag":44,"revision":1,"value":"true"}}"#,
    )
    .unwrap();
    assert!(matches!(
        service.request(request).unwrap(),
        Response::SetValue(_)
    ));
}
