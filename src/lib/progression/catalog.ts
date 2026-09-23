import { gameAssets } from "../assets/game-assets";
import { loadGameIcon } from "../assets/game-icons";

export interface Reward {
  type: string;
  id: string;
  name: string;
  description: string;
  sprite: string | null;
  duration?: number;
  craftedAt?: string[];
  ingredients?: { id: string; name: string; count: number; sprite: string | null }[];
}
export interface TechnologyNode {
  id: string;
  name: string;
  description: string;
  tab: string;
  x: number;
  y: number;
  parents: string[];
  lockType: string;
  availableAtStart: boolean;
  hiddenAtStart: boolean;
  type?: "Common" | "CharRep" | "DisRep";
  requirements?: Record<string, number>;
  additionalEffects?: { addResources: Record<string, number>; setResources: Record<string, number>; expressions: string[] };
  gate?: { name: string; resource: string; value: number; sprite: string | null } | null;
  icon: string | null;
  price: Record<string, number>;
  rewards: Reward[];
}
export interface TechnologyTab { id: string; name: string; sprite: string }
export interface TalentBranch { id: string; name: string; description: string; fontIcon: string; color: string }
export interface InspirationDef {
  id: string; baseId: string; name: string; description: string; talent: string;
  level: number; levelFrame: number; completionPrice: number; completionGoal: number;
  completionExp: number; inspirationLocks: string[]; techLocks: string[]; questLocks: string[]; sprite: string;
}
export interface TalentLevelNode {
  id: string; name: string; description: string; talent: string; x: number; y: number;
  parents: string[]; lockType: string; availableAtStart: boolean; hidden: boolean;
  unknown: boolean; freeCoordinates: boolean; talentValue: number; pointPrice: number;
  sprite: string; perk: Reward | null;
}
export interface ProgressionCatalog {
  schemaVersion: number;
  technology: { tabs: TechnologyTab[]; nodes: TechnologyNode[] };
  talents: { branches: TalentBranch[]; expLevels: Record<string, number>[]; inspirations: InspirationDef[]; levelUps: TalentLevelNode[] };
}
export interface InspirationState { id: string; currentValue: string; completionGoalValue: string }
export interface TalentState {
  id: string; curExp: string; curTalentLevel: string; talentExpPoints: string;
  curTalentValue: string; studiedLevelUps: string[]; inspirations: InspirationState[];
}
export interface ProgressionState { unlockedTechnologies: string[]; talents: TalentState[] }

export async function loadProgressionCatalog() {
  return (await gameAssets()).catalog<ProgressionCatalog>("progression");
}

let progressionPreload: Promise<void> | undefined;

function idle() {
  return new Promise<void>((resolve) => {
    if ("requestIdleCallback" in window)
      window.requestIdleCallback(() => resolve(), { timeout: 100 });
    else globalThis.setTimeout(resolve, 0);
  });
}

/** Warm all progression graphics in small background batches after a save opens. */
export function preloadProgressionAssets() {
  return (progressionPreload ??= (async () => {
    const catalog = await loadProgressionCatalog();
    const requests = new Map<
      string,
      { name: string; kind: "font" | "sprite"; recolor: boolean }
    >();
    const add = (
      name: string | null | undefined,
      kind: "font" | "sprite" = "sprite",
      recolor = kind === "sprite",
    ) => {
      if (name) requests.set(`${kind}:${name}:${recolor}`, { name, kind, recolor });
    };
    for (const tab of catalog.technology.tabs) add(tab.sprite);
    for (const node of catalog.technology.nodes) {
      add(node.icon);
      add(node.gate?.sprite);
      for (const name of Object.keys(node.price)) add(name, "font", false);
      for (const reward of node.rewards) {
        add(reward.sprite);
        for (const ingredient of reward.ingredients ?? []) add(ingredient.sprite);
      }
    }
    for (const branch of catalog.talents.branches)
      add(branch.fontIcon, "font", false);
    for (const inspiration of catalog.talents.inspirations)
      add(inspiration.sprite);
    for (const level of catalog.talents.levelUps) {
      add(level.sprite, "sprite", false);
      add(level.perk?.sprite, "sprite", false);
    }
    const pending = [...requests.values()];
    await idle();
    for (let start = 0; start < pending.length; start += 8) {
      await Promise.allSettled(
        pending.slice(start, start + 8).map(({ name, kind, recolor }) =>
          loadGameIcon(name, kind, {
            crop: true,
            outline: recolor ? "#17181d" : undefined,
          }),
        ),
      );
      await idle();
    }
  })().catch(() => {
    progressionPreload = undefined;
  }));
}

export function dependencyClosure<T extends { id: string; parents: string[] }>(target: T, nodes: T[], unlocked: Set<string>) {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const result: T[] = [];
  const visiting = new Set<string>();
  const visit = (node: T) => {
    if (unlocked.has(node.id) || visiting.has(node.id)) return;
    visiting.add(node.id);
    for (const id of node.parents) { const parent = byId.get(id); if (parent) visit(parent); }
    result.push(node);
  };
  visit(target);
  return result;
}

export function localized(value: string | null | undefined, key: string) {
  return value && value !== key ? value : null;
}

export function readableId(id: string) {
  return id.replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
}
