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

/// Rewards to take back when technologies are locked. The caller omits rewards
/// that remaining technologies or perks still grant.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TechnologyLocks {
    #[serde(default)]
    pub crafts: Vec<String>,
    #[serde(default)]
    pub alchemy_formulas: Vec<String>,
    #[serde(default)]
    pub buildings: Vec<String>,
    #[serde(default)]
    pub town_buildings: Vec<String>,
    #[serde(default)]
    pub perks: Vec<String>,
}

/// A technology's price in the player's red, green and blue technology points.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TechnologyPoints {
    #[serde(default)]
    pub red: u32,
    #[serde(default)]
    pub green: u32,
    #[serde(default)]
    pub blue: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TalentUnlock {
    pub id: String,
    pub talent_value: i32,
    /// Perk point price. See [`Baseline`] for when it is charged or refunded.
    #[serde(default)]
    pub point_price: i32,
}

/// Technologies and talent levels that were unlocked when the save was opened.
///
/// The player paid for these in the game, so locking one refunds its price and
/// unlocking it again charges the price again. Anything else was unlocked by the
/// editor for free and neither costs nor refunds points. Repeated lock/unlock
/// cycles therefore never create points, and undo stays consistent because the
/// decision depends only on the current state and this baseline.
#[derive(Debug, Clone, Default)]
pub(crate) struct Baseline {
    technologies: HashSet<String>,
    levels: HashSet<String>,
}

pub(crate) fn baseline(doc: &Document) -> Baseline {
    let technologies = root_field(doc, "knowledgeSystem")
        .and_then(|knowledge| string_list(doc, knowledge, "unlockedTechs"))
        .map(|(_, ids)| ids.into_iter().collect())
        .unwrap_or_default();
    let levels = talents(doc)
        .map(|branches| {
            branches
                .into_iter()
                .filter_map(|branch| string_list(doc, branch, "studiedLevelUps").ok())
                .flat_map(|(_, ids)| ids)
                .collect()
        })
        .unwrap_or_default();
    Baseline {
        technologies,
        levels,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    /// `costs` holds the prices of the technologies; see [`Baseline`].
    UnlockTechnologies {
        ids: Vec<String>,
        rewards: TechnologyRewards,
        #[serde(default)]
        costs: BTreeMap<String, TechnologyPoints>,
    },
    LockTechnologies {
        ids: Vec<String>,
        rewards: TechnologyLocks,
        #[serde(default)]
        costs: BTreeMap<String, TechnologyPoints>,
    },
    UnlockTalentLevels {
        talent: String,
        levels: Vec<TalentUnlock>,
    },
    /// Removes studied levels, their mastery and the given linked active perks.
    LockTalentLevels {
        talent: String,
        levels: Vec<TalentUnlock>,
        #[serde(default)]
        perks: Vec<String>,
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
fn valid_ids(values: &[String]) -> Result<HashSet<&str>, Error> {
    if values
        .iter()
        .any(|value| value.is_empty() || value.len() > 256)
    {
        return Err(failure("Invalid progression ID"));
    }
    Ok(values.iter().map(String::as_str).collect())
}
/// Removes every entry of a string list that matches one of `values`; returns the removed values.
fn remove_strings(
    doc: &mut Document,
    owner: usize,
    name: &str,
    values: &[String],
) -> Result<HashSet<String>, Error> {
    let remove = valid_ids(values)?;
    let (array, _) = string_list(doc, owner, name)?;
    let mut removed = HashSet::new();
    for child in doc.records[array].children.clone().into_iter().rev() {
        let value = text_value(doc, child)?;
        if remove.contains(value.as_str()) {
            apply(doc, Operation::Remove { node: child })?;
            removed.insert(value);
        }
    }
    Ok(removed)
}
fn remove_perks(doc: &mut Document, ids: &[String]) -> Result<(), Error> {
    if ids.is_empty() {
        return Ok(());
    }
    let remove = valid_ids(ids)?;
    let perks = root_field(doc, "perkSystemData")?;
    let array = array(doc, field(doc, perks, "activePerks")?)?;
    for child in doc.records[array].children.clone().into_iter().rev() {
        let id = resolve(doc, child)
            .and_then(|entry| field(doc, entry, "id"))
            .and_then(|node| text_value(doc, node))?;
        if remove.contains(id.as_str()) {
            apply(doc, Operation::Remove { node: child })?;
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
fn int_field(doc: &Document, owner: usize, name: &str) -> Result<i32, Error> {
    view(doc, field(doc, owner, name)?)
        .value
        .unwrap_or_default()
        .parse()
        .map_err(|_| failure("Invalid progression value"))
}
/// Adds (refund) or subtracts (charge) the prices of `ids` that belong to the baseline.
fn settle_technologies(
    doc: &mut Document,
    ids: &HashSet<String>,
    costs: &BTreeMap<String, TechnologyPoints>,
    sign: f64,
) -> Result<(), Error> {
    let mut total = [0f64; 3];
    for id in ids {
        if let Some(cost) = costs.get(id) {
            total[0] += f64::from(cost.red);
            total[1] += f64::from(cost.green);
            total[2] += f64::from(cost.blue);
        }
    }
    for (key, amount) in ["tech_red", "tech_green", "tech_blue"]
        .into_iter()
        .zip(total)
    {
        crate::general::add(doc, key, sign * amount)?;
    }
    Ok(())
}
fn add_perk_points(doc: &mut Document, branch: usize, amount: i32) -> Result<(), Error> {
    if amount == 0 {
        return Ok(());
    }
    let current = int_field(doc, branch, "talentExpPoints")?;
    let next = current
        .checked_add(amount)
        .ok_or_else(|| failure("Perk point overflow"))?;
    if next < 0 {
        return Err(failure(&format!(
            "Not enough perk points: needs {}, has {current}",
            -amount
        )));
    }
    set_i32(doc, branch, "talentExpPoints", next.to_string())
}
fn level_points<'a>(
    levels: impl Iterator<Item = &'a TalentUnlock>,
    baseline: &Baseline,
) -> Result<i32, Error> {
    levels
        .filter(|x| baseline.levels.contains(&x.id))
        .try_fold(0i32, |sum, x| {
            sum.checked_add(x.point_price.max(0))
                .ok_or_else(|| failure("Perk point overflow"))
        })
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

pub(crate) fn read(doc: &Document, baseline: &Baseline) -> Result<Value, Error> {
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
    let perks = root_field(doc, "perkSystemData")?;
    let perk_array = array(doc, field(doc, perks, "activePerks")?)?;
    let active_perks = doc.records[perk_array]
        .children
        .iter()
        .map(|entry| text_value(doc, field(doc, resolve(doc, *entry)?, "id")?))
        .collect::<Result<Vec<_>, _>>()?;
    let mut paid_technologies: Vec<_> = baseline.technologies.iter().collect();
    let mut paid_levels: Vec<_> = baseline.levels.iter().collect();
    paid_technologies.sort();
    paid_levels.sort();
    Ok(json!({
        "unlockedTechnologies": unlocked,
        "talents": branches,
        "activePerks": active_perks,
        "paid": {"technologies": paid_technologies, "levels": paid_levels}
    }))
}

pub(crate) fn write(doc: &mut Document, baseline: &Baseline, edit: Edit) -> Result<(), Error> {
    match edit {
        Edit::UnlockTechnologies {
            ids,
            rewards,
            costs,
        } => {
            if ids.is_empty() || ids.len() > 500 {
                return Err(failure("Select 1..500 technologies"));
            }
            let knowledge = root_field(doc, "knowledgeSystem")?;
            let (_, existing) = string_list(doc, knowledge, "unlockedTechs")?;
            let existing: HashSet<_> = existing.into_iter().collect();
            // Re-unlocking a technology whose price was refunded charges it again.
            let charged: HashSet<_> = ids
                .iter()
                .filter(|id| !existing.contains(*id) && baseline.technologies.contains(*id))
                .cloned()
                .collect();
            settle_technologies(doc, &charged, &costs, -1.0)?;
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
        Edit::LockTechnologies {
            ids,
            rewards,
            costs,
        } => {
            if ids.is_empty() || ids.len() > 500 {
                return Err(failure("Select 1..500 technologies"));
            }
            let knowledge = root_field(doc, "knowledgeSystem")?;
            let removed = remove_strings(doc, knowledge, "unlockedTechs", &ids)?;
            let refunded: HashSet<_> = removed
                .into_iter()
                .filter(|id| baseline.technologies.contains(id))
                .collect();
            settle_technologies(doc, &refunded, &costs, 1.0)?;
            remove_strings(doc, knowledge, "unlockedCrafts", &rewards.crafts)?;
            remove_strings(
                doc,
                knowledge,
                "unlockedAlchemyFormulas",
                &rewards.alchemy_formulas,
            )?;
            remove_strings(doc, knowledge, "unlockedBuildings", &rewards.buildings)?;
            remove_strings(
                doc,
                knowledge,
                "unlockedTownBuildings",
                &rewards.town_buildings,
            )?;
            remove_perks(doc, &rewards.perks)?;
        }
        Edit::LockTalentLevels {
            talent: talent_id,
            levels,
            perks,
        } => {
            if levels.is_empty() || levels.len() > 500 {
                return Err(failure("Select 1..500 talent levels"));
            }
            let branch = talent(doc, &talent_id)?;
            let ids: Vec<_> = levels.iter().map(|x| x.id.clone()).collect();
            let removed = remove_strings(doc, branch, "studiedLevelUps", &ids)?;
            let decrease: i32 =
                levels
                    .iter()
                    .filter(|x| removed.contains(&x.id))
                    .try_fold(0i32, |sum, x| {
                        sum.checked_add(x.talent_value.max(0))
                            .ok_or_else(|| failure("Talent value overflow"))
                    })?;
            let refund = level_points(levels.iter().filter(|x| removed.contains(&x.id)), baseline)?;
            if decrease > 0 {
                let current = int_field(doc, branch, "curTalentValue")?;
                set_i32(
                    doc,
                    branch,
                    "curTalentValue",
                    current.saturating_sub(decrease).max(0).to_string(),
                )?;
            }
            add_perk_points(doc, branch, refund)?;
            remove_perks(doc, &perks)?;
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
            // Re-unlocking a level whose price was refunded charges it again.
            let charge = level_points(added.iter(), baseline)?;
            add_perk_points(doc, branch, -charge)?;
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
