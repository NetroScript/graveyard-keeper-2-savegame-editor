//! Technology, inspiration and talent-tree state stored in the save.
use crate::service::view;
use crate::wire::{base_tag, Payload};
use crate::workspace::{apply, create_template, failure, field, resolve, Operation};
use crate::{Document, Error};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TechnologyRewards {
    #[serde(default)]
    pub crafts: Vec<String>,
    #[serde(default)]
    pub alchemy_formulas: Vec<String>,
    #[serde(default)]
    pub buildings: Vec<String>,
    #[serde(default)]
    pub town_buildings: Vec<String>,
    #[serde(default)]
    pub perks: Vec<PerkUnlock>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PerkUnlock {
    pub id: String,
    pub duration: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TalentUnlock {
    pub id: String,
    pub talent_value: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    UnlockTechnologies {
        ids: Vec<String>,
        rewards: TechnologyRewards,
    },
    UnlockTalentLevels {
        talent: String,
        levels: Vec<TalentUnlock>,
    },
    SetTalent {
        talent: String,
        field: String,
        value: String,
    },
    SetInspiration {
        talent: String,
        id: String,
        current_value: String,
        completion_goal_value: String,
    },
}

fn root_field(doc: &Document, name: &str) -> Result<usize, Error> {
    field(
        doc,
        *doc.roots.first().ok_or_else(|| failure("Missing root"))?,
        name,
    )
}
fn array(doc: &Document, list: usize) -> Result<usize, Error> {
    let list = resolve(doc, list)?;
    let arrays: Vec<_> = doc.records[list]
        .children
        .iter()
        .copied()
        .filter(|id| base_tag(doc.records[*id].tag) == 6)
        .collect();
    if arrays.len() != 1 {
        return Err(failure("Unsupported list layout"));
    }
    Ok(arrays[0])
}
fn text_value(doc: &Document, node: usize) -> Result<String, Error> {
    match &doc.records[node].payload {
        Payload::Text(value) => Ok(value.display()),
        _ => Err(failure("Expected a string")),
    }
}
fn string_list(doc: &Document, owner: usize, name: &str) -> Result<(usize, Vec<String>), Error> {
    let array = array(doc, field(doc, owner, name)?)?;
    let values = doc.records[array]
        .children
        .iter()
        .map(|id| text_value(doc, *id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((array, values))
}
fn append_strings(
    doc: &mut Document,
    owner: usize,
    name: &str,
    values: Vec<String>,
) -> Result<(), Error> {
    let (array, existing) = string_list(doc, owner, name)?;
    let mut seen: HashSet<_> = existing.into_iter().collect();
    for value in values {
        if value.is_empty() || value.len() > 256 {
            return Err(failure("Invalid progression ID"));
        }
        if seen.insert(value.clone()) {
            let index = doc.records[array].children.len();
            apply(
                doc,
                Operation::Insert {
                    parent: array,
                    index,
                    name: None,
                    kind: "string".into(),
                    value,
                },
            )?;
        }
    }
    Ok(())
}
fn talents(doc: &Document) -> Result<Vec<usize>, Error> {
    let system = root_field(doc, "talentSystemData")?;
    let items = array(doc, field(doc, system, "talentData")?)?;
    doc.records[items]
        .children
        .iter()
        .map(|id| resolve(doc, *id))
        .collect()
}
fn talent(doc: &Document, id: &str) -> Result<usize, Error> {
    talents(doc)?
        .into_iter()
        .find(|node| {
            field(doc, *node, "id")
                .ok()
                .and_then(|n| text_value(doc, n).ok())
                .as_deref()
                == Some(id)
        })
        .ok_or_else(|| failure("Unknown talent branch"))
}
fn set_i32(doc: &mut Document, owner: usize, name: &str, value: String) -> Result<(), Error> {
    let parsed: i32 = value
        .parse()
        .map_err(|_| failure("Enter a 32-bit integer"))?;
    if parsed < 0 {
        return Err(failure("Enter a nonnegative integer"));
    }
    let node = field(doc, owner, name)?;
    if base_tag(doc.records[node].tag) != 24 {
        return Err(failure("Progression field is not int32"));
    }
    apply(
        doc,
        Operation::Set {
            node,
            tag: doc.records[node].tag,
            value,
        },
    )
}

pub(crate) fn read(doc: &Document) -> Result<Value, Error> {
    let knowledge = root_field(doc, "knowledgeSystem")?;
    let (_, unlocked) = string_list(doc, knowledge, "unlockedTechs")?;
    let mut branches = Vec::new();
    for node in talents(doc)? {
        let id = text_value(doc, field(doc, node, "id")?)?;
        let value = |name| -> Result<String, Error> {
            Ok(view(doc, field(doc, node, name)?).value.unwrap_or_default())
        };
        let (_, studied) = string_list(doc, node, "studiedLevelUps")?;
        let progress_array = array(doc, field(doc, node, "inspirationsProgression")?)?;
        let mut inspirations = Vec::new();
        for entry in &doc.records[progress_array].children {
            let entry = resolve(doc, *entry)?;
            inspirations.push(json!({
                "id": text_value(doc, field(doc, entry, "id")?)?,
                "currentValue": view(doc, field(doc, entry, "currentValue")?).value.unwrap_or_default(),
                "completionGoalValue": view(doc, field(doc, entry, "completionGoalValue")?).value.unwrap_or_default()
            }));
        }
        branches.push(json!({"id":id,"curExp":value("curExp")?,"curTalentLevel":value("curTalentLevel")?,"talentExpPoints":value("talentExpPoints")?,"curTalentValue":value("curTalentValue")?,"studiedLevelUps":studied,"inspirations":inspirations}));
    }
    Ok(json!({"unlockedTechnologies":unlocked,"talents":branches}))
}

pub(crate) fn write(doc: &mut Document, edit: Edit) -> Result<(), Error> {
    match edit {
        Edit::UnlockTechnologies { ids, rewards } => {
            if ids.is_empty() || ids.len() > 500 {
                return Err(failure("Select 1..500 technologies"));
            }
            let knowledge = root_field(doc, "knowledgeSystem")?;
            append_strings(doc, knowledge, "unlockedTechs", ids)?;
            append_strings(doc, knowledge, "unlockedCrafts", rewards.crafts)?;
            append_strings(
                doc,
                knowledge,
                "unlockedAlchemyFormulas",
                rewards.alchemy_formulas,
            )?;
            append_strings(doc, knowledge, "unlockedBuildings", rewards.buildings)?;
            append_strings(
                doc,
                knowledge,
                "unlockedTownBuildings",
                rewards.town_buildings,
            )?;
            if !rewards.perks.is_empty() {
                let perks = root_field(doc, "perkSystemData")?;
                let array = array(doc, field(doc, perks, "activePerks")?)?;
                let existing: HashSet<_> = doc.records[array]
                    .children
                    .iter()
                    .filter_map(|n| resolve(doc, *n).ok())
                    .filter_map(|n| field(doc, n, "id").ok())
                    .filter_map(|n| text_value(doc, n).ok())
                    .collect();
                for perk in rewards
                    .perks
                    .into_iter()
                    .filter(|p| !existing.contains(&p.id))
                {
                    let mut values = BTreeMap::new();
                    values.insert("id".into(), perk.id);
                    values.insert("currentDuration".into(), perk.duration);
                    create_template(
                        doc,
                        array,
                        doc.records[array].children.len(),
                        None,
                        "perk-data",
                        values,
                    )?;
                }
            }
        }
        Edit::UnlockTalentLevels {
            talent: talent_id,
            levels,
        } => {
            if levels.is_empty() || levels.len() > 500 {
                return Err(failure("Select 1..500 talent levels"));
            }
            let branch = talent(doc, &talent_id)?;
            let (_, existing) = string_list(doc, branch, "studiedLevelUps")?;
            let existing: HashSet<_> = existing.into_iter().collect();
            let added: Vec<_> = levels
                .into_iter()
                .filter(|x| !existing.contains(&x.id))
                .collect();
            let increase: i32 = added.iter().try_fold(0i32, |sum, x| {
                sum.checked_add(x.talent_value)
                    .ok_or_else(|| failure("Talent value overflow"))
            })?;
            append_strings(
                doc,
                branch,
                "studiedLevelUps",
                added.iter().map(|x| x.id.clone()).collect(),
            )?;
            if increase > 0 {
                let node = field(doc, branch, "curTalentValue")?;
                let current: i32 = view(doc, node)
                    .value
                    .unwrap_or_default()
                    .parse()
                    .map_err(|_| failure("Invalid talent value"))?;
                set_i32(
                    doc,
                    branch,
                    "curTalentValue",
                    current
                        .checked_add(increase)
                        .ok_or_else(|| failure("Talent value overflow"))?
                        .to_string(),
                )?;
            }
        }
        Edit::SetTalent {
            talent: id,
            field: name,
            value,
        } => {
            if !matches!(
                name.as_str(),
                "curExp" | "curTalentLevel" | "talentExpPoints" | "curTalentValue"
            ) {
                return Err(failure("Unknown talent field"));
            }
            let branch = talent(doc, &id)?;
            set_i32(doc, branch, &name, value)?;
        }
        Edit::SetInspiration {
            talent: talent_id,
            id,
            current_value,
            completion_goal_value,
        } => {
            if id.is_empty() || id.len() > 256 {
                return Err(failure("Invalid inspiration ID"));
            }
            let branch = talent(doc, &talent_id)?;
            let list = array(doc, field(doc, branch, "inspirationsProgression")?)?;
            let existing = doc.records[list]
                .children
                .iter()
                .filter_map(|n| resolve(doc, *n).ok())
                .find(|n| {
                    field(doc, *n, "id")
                        .ok()
                        .and_then(|i| text_value(doc, i).ok())
                        .as_deref()
                        == Some(&id)
                });
            let entry = if let Some(entry) = existing {
                entry
            } else {
                create_template(
                    doc,
                    list,
                    doc.records[list].children.len(),
                    None,
                    "inspiration-progress",
                    BTreeMap::from([("id".into(), id)]),
                )?
            };
            set_i32(doc, entry, "currentValue", current_value)?;
            set_i32(doc, entry, "completionGoalValue", completion_goal_value)?;
        }
    }
    Ok(())
}
