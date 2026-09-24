import FuzzySearch from "fz-search";
import type { AssetPack } from "../assets/asset-pack";

export interface ItemDefinition {
  id: string;
  name: string;
  description: string;
  sprite: string | null;
  quality: {
    type: string;
    value: number;
    family: string | null;
    overlaySprite: string | null;
  };
  fields: {
    stackCount: number;
    itemSize: string;
    isBag: boolean;
    bagSize: number;
    inventorySize: number;
    hasDurability: boolean;
    itemGroupIds: string[];
    type: string;
    isMainOrgan?: boolean;
    redSkulls: number;
    whiteSkulls: number;
  };
}
export interface InventoryItem {
  node: number;
  id: string;
  count: string;
  durability: string | null;
}
export interface InventoryData {
  node: number;
  kind: string;
  title: string;
  parentNode?: number | null;
  displayDepth?: number;
  capacity: string;
  items: InventoryItem[];
  ruleId: number;
  location?: {
    player?: boolean;
    scene?: string | null;
    world?: string | null;
    position?: string[];
  };
}
export interface InventoryRule {
  allowed: string[];
  error: string | null;
}
export interface InventorySnapshot {
  inventories: InventoryData[];
  rules: Record<string, InventoryRule>;
}
export interface InventoryDelta {
  upsert: InventoryData[];
  removed: number[];
  rules: Record<string, InventoryRule>;
}
export interface SpriteCatalog {
  sprites: Record<
    string,
    { image: string | null; width: number; height: number }
  >;
  fontIcons: Record<string, { image: string; assetId: string }[]>;
  spriteAssets: { id: string; isDefault: boolean }[];
}
export interface Family {
  id: string;
  name: string;
  description: string;
  category: string;
  variants: ItemDefinition[];
}
export const plainText = (value: string) => value.replace(/<[^>]*>/g, "");
export class ItemCatalog {
  readonly items: Record<string, ItemDefinition>;
  readonly sprites: SpriteCatalog;
  readonly rules: { items: Record<string, unknown> };
  readonly families: Family[];
  constructor(readonly pack: AssetPack) {
    this.items = pack.catalog("items");
    this.sprites = pack.catalog("icons");
    const inventoryRules = pack.catalog<{
      bags: Record<string, { complete: boolean; allowedItemIds: string[] }>;
      equipment?: {
        id: string;
        canBeEquipped: boolean;
        type: string;
      }[];
    }>("inventory-rules");
    const equipment = new Map(
      (inventoryRules.equipment ?? []).map((entry) => [entry.id, entry]),
    );
    this.rules = {
      items: Object.fromEntries(
        Object.values(this.items)
          .filter((i) => i.fields.stackCount >= 1)
          .map((i) => [
            i.id,
            {
              stack: i.fields.stackCount,
              family: i.fields.isMainOrgan
                ? (i.quality.family ?? i.id.split(":")[0]).replace(
                    /_\d+_\d+$/,
                    "",
                  )
                : i.quality.family,
              size: i.fields.itemSize,
              groups: i.fields.itemGroupIds,
              isBag: i.fields.isBag,
              capacity: i.fields.isBag
                ? i.fields.bagSize
                : i.fields.inventorySize,
              durability: i.fields.hasDurability,
              allowed: inventoryRules.bags[i.id]?.complete
                ? inventoryRules.bags[i.id].allowedItemIds
                : null,
              toolBelt: i.id === "hand_tool" || !!equipment.get(i.id)?.canBeEquipped,
              equipmentType: equipment.get(i.id)?.type ?? i.fields.type,
            },
          ]),
      ),
    };
    const families = new Map<string, ItemDefinition[]>();
    for (const item of Object.values(this.items)) {
      // Star families are explicit. Colon suffixes also encode organ/skull variants.
      const base = item.quality.family ?? item.id.split(":")[0];
      const key = item.fields.isMainOrgan
        ? base.replace(/_\d+_\d+$/, "")
        : base;
      const entries = families.get(key) ?? [];
      entries.push(item);
      families.set(key, entries);
    }
    this.families = [...families]
      .map(([id, variants]) => {
        variants.sort(
          (a, b) =>
            b.fields.whiteSkulls - a.fields.whiteSkulls ||
            a.fields.redSkulls - b.fields.redSkulls ||
            b.quality.value - a.quality.value ||
            a.id.localeCompare(b.id),
        );
        const base = this.items[id] ?? variants[0];
        const localized = (
          pack.pack.metadata.catalogs["localization.en"] as
            Record<string, string> | undefined
        )?.[id];
        return {
          id,
          name: plainText(localized || base.name || id),
          description: plainText(base.description || ""),
          category: base.fields.type,
          variants,
        };
      })
      .sort((a, b) => a.name.localeCompare(b.name));
  }
  async sprite(name: string | null, outline?: string) {
    const hash = name && this.sprites.sprites[name]?.image;
    return hash ? this.pack.imageUrl(hash, outline) : undefined;
  }
  async icon(name: string) {
    const candidates = this.sprites.fontIcons[name] ?? [];
    const preferred = this.sprites.spriteAssets.find((s) => s.isDefault)?.id;
    const icon =
      candidates.find((i) => i.assetId === preferred) ?? candidates[0];
    return icon ? this.pack.imageUrl(icon.image) : undefined;
  }
  forInventory(allowed: string[]) {
    const valid = new Set(allowed);
    return this.families
      .map((f) => ({
        ...f,
        variants: f.variants.filter((i) => valid.has(i.id)),
      }))
      .filter((f) => f.variants.length);
  }
}
export function itemSearch(families: Family[]) {
  const source = families.map((f) => ({
    id: f.id,
    name: f.name,
    description: f.description,
    category: f.category,
  }));
  // Separate indexes make priority explicit: name matches precede ID, then descriptive matches.
  const indexes = [["name"], ["id"], ["description", "category"]].map(
    (keys) =>
      new FuzzySearch({
        source,
        keys,
        output_map: "root.item",
        output_limit: 0,
      }),
  );
  const byId = new Map(families.map((f) => [f.id, f]));
  return (query: string): Family[] => {
    if (!query.trim()) return families;
    const found = new Set<string>();
    for (const index of indexes)
      for (const result of index.search(query) as { id: string }[])
        found.add(result.id);
    return [...found].map((id) => byId.get(id)!);
  };
}
export function variantLabel(item: ItemDefinition) {
  if (item.fields.isMainOrgan)
    return `${item.fields.whiteSkulls} white / ${item.fields.redSkulls} red skulls · Quality ${item.quality.value}`;
  if (item.quality.type === "Star")
    return (
      (
        { 0: "No star", 1: "Bronze", 2: "Silver", 3: "Gold" } as Record<
          number,
          string
        >
      )[item.quality.value] ?? `Quality ${item.quality.value}`
    );
  if (item.fields.whiteSkulls || item.fields.redSkulls)
    return `${item.fields.whiteSkulls} white / ${item.fields.redSkulls} red skulls`;
  return (
    plainText(item.name || item.id) +
    (item.id.includes(":") ? ` (${item.id.split(":").slice(1).join(":")})` : "")
  );
}
