import { gameAssets } from "../assets/game-assets";

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
