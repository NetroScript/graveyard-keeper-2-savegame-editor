use crate::service::view;
use crate::wire::{base_tag, Payload};
use crate::workspace::{apply, create_template, failure, field, resolve, Operation};
use crate::{Document, Error};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const RESOURCES: &[&str] = &[
    "energy",
    "stamina",
    "money",
    "insanity",
    "happiness",
    "tech_red",
    "tech_green",
    "tech_blue",
];
fn player(doc: &Document) -> Result<usize, Error> {
    field(
        doc,
        *doc.roots.first().ok_or_else(|| failure("Missing root"))?,
        "playerData",
    )
}
fn array(doc: &Document, list: usize) -> Result<usize, Error> {
    let list = resolve(doc, list)?;
    let ids: Vec<_> = doc.records[list]
        .children
        .iter()
        .copied()
        .filter(|id| doc.records[*id].tag == 6)
        .collect();
    if ids.len() != 1 {
        return Err(failure("Unsupported resource list layout"));
    }
    Ok(ids[0])
}
fn resources(doc: &Document) -> Result<(usize, usize, BTreeMap<String, usize>), Error> {
    let res = field(doc, player(doc)?, "res")?;
    let types = array(doc, field(doc, res, "resType")?)?;
    let values = array(doc, field(doc, res, "resValues")?)?;
    if doc.records[types].children.len() != doc.records[values].children.len() {
        return Err(failure("Resource lists have different lengths"));
    }
    let mut mapping = BTreeMap::new();
    for (&t, &v) in doc.records[types]
        .children
        .iter()
        .zip(&doc.records[values].children)
    {
        let Payload::Text(name) = &doc.records[t].payload else {
            return Err(failure("Resource name is not a string"));
        };
        let name = name.display();
        let atom_type = field(doc, v, "type")?;
        if view(doc, atom_type).value.as_deref() != Some(&name) {
            return Err(failure("Resource lists disagree"));
        }
        let value = field(doc, v, "value")?;
        if base_tag(doc.records[value].tag) != 32 {
            return Err(failure("Resource value is not float32"));
        }
        if mapping.insert(name, value).is_some() {
            return Err(failure("Duplicate resource names"));
        }
    }
    Ok((types, values, mapping))
}
fn health(doc: &Document, name: &str) -> Result<usize, Error> {
    let id = field(
        doc,
        field(doc, player(doc)?, "hpComponent")?,
        if name == "hp" { "hp" } else { "maxHpValue" },
    )?;
    if base_tag(doc.records[id].tag) != 24 {
        return Err(failure("Health field is not int32"));
    }
    Ok(id)
}
pub(crate) fn read(doc: &Document) -> Value {
    let resources = resources(doc);
    let fields:Vec<_>=["hp","max_hp"].into_iter().chain(RESOURCES.iter().copied()).map(|key|{
        let result=if key=="hp"||key=="max_hp"{health(doc,key).map(Some)}else{resources.as_ref().map(|(_,_,m)|m.get(key).copied()).map_err(Clone::clone)};
        match result {Ok(node)=>json!({"key":key,"node":node,"value":node.and_then(|id|view(doc,id).value).unwrap_or_else(||"0".into()),"error":null}),Err(e)=>json!({"key":key,"node":null,"value":null,"error":e.to_string()})}
    }).collect();
    json!(fields)
}
pub(crate) fn write(doc: &mut Document, values: BTreeMap<String, String>) -> Result<(), Error> {
    for (key, value) in values {
        let number: f64 = value
            .parse()
            .map_err(|_| failure("Enter a finite nonnegative number"))?;
        if !number.is_finite() || number < 0.0 || (key == "max_hp" && number <= 0.0) {
            return Err(failure(
                "Enter a finite nonnegative value; maximum health must be positive",
            ));
        }
        let node = if key == "hp" || key == "max_hp" {
            health(doc, &key)?
        } else {
            if !RESOURCES.contains(&key.as_str()) {
                return Err(failure("Unknown General field"));
            }
            let (types, atoms, mapping) = resources(doc)?;
            if let Some(id) = mapping.get(&key) {
                *id
            } else {
                let index = doc.records[atoms].children.len();
                let atom = create_template(
                    doc,
                    atoms,
                    index,
                    None,
                    "game-res-atom",
                    BTreeMap::from([("type".into(), key.clone())]),
                )?;
                apply(
                    doc,
                    Operation::Insert {
                        parent: types,
                        index,
                        name: None,
                        kind: "string".into(),
                        value: key,
                    },
                )?;
                field(doc, atom, "value")?
            }
        };
        apply(
            doc,
            Operation::Set {
                node,
                tag: doc.records[node].tag,
                value,
            },
        )?;
    }
    Ok(())
}
