import { gameAssets } from "../assets/game-assets";
import type { ItemCatalog } from "../inventory/catalog";

export const ZOMBIE_WORKER_APPEARANCE_SET = "zombie_worker";
export const ZOMBIE_ASSISTANT_APPEARANCE_SET = "zombie_assistant";

export function zombieAppearanceSetId(zombieId: string) {
  return zombieId === ZOMBIE_ASSISTANT_APPEARANCE_SET
    ? ZOMBIE_ASSISTANT_APPEARANCE_SET
    : ZOMBIE_WORKER_APPEARANCE_SET;
}

export interface ZombiePerkEffect {
  type: string;
  value: number;
}

export interface ZombiePerk {
  id: string;
  name: string;
  description: string;
  sprite: string;
  craftMasteryBonus: number;
  craftStartTicks: number;
  craftTotalProgressTicksBonus: number;
  effects: {
    setResourcesOnAdd: ZombiePerkEffect[];
    setResourcesOnRemove: ZombiePerkEffect[];
  };
}

export interface ZombieTalentNode {
  id: string;
  name: string;
  description: string;
  talent: string;
  x: number;
  y: number;
  parents: string[];
  lockType: string;
  availableAtStart: boolean;
  hidden: boolean;
  talentValue: number;
  technologyPrice: { red: number; green: number; blue: number };
  sprite: string;
  perkId: string;
  perk: ZombiePerk | null;
}

export interface ZombieAppearanceSet {
  id: string;
  bodyIds: number[];
  headIds: number[];
  bodyVariants: { id: number; sprite: string; overlaySprite: string | null }[];
  headVariants: { id: number; sprite: string }[];
  bodyLuts: { name: string; texture: string }[];
  headLuts: { name: string; texture: string }[];
}

export interface ZombieAssetCatalog {
  bodies: unknown[];
  crafts: unknown[];
  workstations: {
    id: string;
    name: string;
    canInsertZombie: boolean;
    talent: string;
    actionableTool: string;
  }[];
  fighters: unknown[];
  customization: {
    available: boolean;
    sets: ZombieAppearanceSet[];
    portrait: { stoneSprite: string | null };
  };
}

export interface ZombieBranch {
  id: string;
  name: string;
  description: string;
  fontIcon: string;
  color: string;
}

export interface ZombieCatalog {
  assets: ZombieAssetCatalog;
  branches: ZombieBranch[];
  nodes: ZombieTalentNode[];
  names: Record<string, string>;
  backend: {
    talents: Record<string, unknown>;
    appearances: Record<string, unknown>;
  };
}

export interface ZombieBodyItem {
  node: number;
  id: string;
  count: string;
  guid: string;
  equipped: "hand" | "armor" | "collar" | null;
}

export interface ZombieTalentState {
  id: string;
  value: string;
  studied: string[];
}

export interface ZombieState {
  node: number;
  id: string;
  guid: string;
  name: string;
  role: string;
  roleValue: number;
  world: string;
  attachedGuid: string | null;
  body: number;
  bodyCapacity: string;
  items: ZombieBodyItem[];
  cargo: {
    node: number;
    capacity: string;
    items: ZombieBodyItem[];
  } | null;
  carriedItem: {
    label: string;
    item: ZombieBodyItem;
  } | null;
  redSkulls: string;
  whiteSkulls: string;
  usedTalentSlots: number;
  points: { red: string; green: string; blue: string };
  appearance: {
    set: string;
    body: number;
    head: number;
    bodyLut: string;
    headLut: string;
  };
  talents: ZombieTalentState[];
  disabledTalents: string[];
  activePerks: string[];
}

export interface ZombieSnapshot {
  zombies: ZombieState[];
}

export async function loadZombieCatalog(): Promise<ZombieCatalog> {
  const pack = await gameAssets();
  const assets = pack.catalog<ZombieAssetCatalog>("zombies");
  const progression = pack.catalog<{
    talents: { branches: ZombieBranch[]; zombieLevelUps: ZombieTalentNode[] };
  }>("progression");
  const nodes = progression.talents.zombieLevelUps ?? [];
  const localization = pack.catalog<Record<string, string>>("localization.en");
  return {
    assets,
    branches: progression.talents.branches,
    nodes,
    names: Object.fromEntries(
      Object.entries(localization).filter(([id]) =>
        id.startsWith("zombie_name_"),
      ),
    ),
    backend: {
      talents: Object.fromEntries(
        nodes.map((node) => [
          node.id,
          {
            talent: node.talent,
            parents: node.parents,
            lockType: node.lockType,
            availableAtStart: node.availableAtStart,
            talentValue: node.talentValue,
            red: node.technologyPrice.red,
            green: node.technologyPrice.green,
            blue: node.technologyPrice.blue,
            perk: node.perk
              ? {
                  id: node.perk.id,
                  resourcesOnAdd: node.perk.effects?.setResourcesOnAdd ?? [],
                  resourcesOnRemove:
                    node.perk.effects?.setResourcesOnRemove ?? [],
                }
              : null,
          },
        ]),
      ),
      appearances: Object.fromEntries(
        assets.customization.sets
          .filter(
            (set) =>
              set.id === ZOMBIE_WORKER_APPEARANCE_SET ||
              set.id === ZOMBIE_ASSISTANT_APPEARANCE_SET,
          )
          .map((set) => [
            set.id,
            {
              bodyIds: set.bodyIds,
              headIds: set.headIds,
              bodyLuts: set.bodyLuts.map((entry) => entry.name),
              headLuts: set.headLuts.map((entry) => entry.name),
            },
          ]),
      ),
    },
  };
}

export function bestBodyItems(catalog: ItemCatalog) {
  const organTypes = ["bones", "brain", "heart", "guts", "skin", "skull"];
  const organs = organTypes
    .map(
      (type) =>
        Object.values(catalog.items)
          .filter(
            (item) =>
              item.fields.isMainOrgan &&
              !item.fields.isOrganMistake &&
              item.id.toLowerCase().startsWith(`${type}_`),
          )
          .sort(
            (a, b) =>
              b.fields.redSkulls - a.fields.redSkulls ||
              b.fields.whiteSkulls - a.fields.whiteSkulls ||
              b.quality.value - a.quality.value,
          )[0],
    )
    .filter(Boolean);
  const embalming = Object.values(catalog.items)
    .filter((item) => item.fields.type === "Embalm")
    .sort(
      (a, b) =>
        b.fields.redSkulls - a.fields.redSkulls ||
        b.fields.whiteSkulls - a.fields.whiteSkulls,
    )[0];
  const collar = Object.values(catalog.items)
    .filter((item) => item.fields.type === "Collar")
    .sort(
      (a, b) =>
        (b.fields.redSkullsMaxCollar ?? 0) - (a.fields.redSkullsMaxCollar ?? 0),
    )[0];
  return { organs, embalming, collar };
}

export function bestEquipment(catalog: ItemCatalog, talent: string) {
  return Object.values(catalog.items)
    .filter(
      (item) =>
        (item.fields.talentIds ?? []).includes(talent) &&
        (item.fields.talentBonus ?? 0) > 0,
    )
    .sort((a, b) => (b.fields.talentBonus ?? 0) - (a.fields.talentBonus ?? 0));
}
