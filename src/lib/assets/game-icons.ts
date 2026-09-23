import { gameAssets } from "./game-assets";
import type { ImageRenderOptions } from "./asset-pack";

interface Icons {
  sprites: Record<string, { image?: string }>;
  fontIcons: Record<string, { assetId: string; image: string }[]>;
  spriteAssets: { id: string; isDefault: boolean }[];
}

const cache = new Map<string, Promise<string | undefined>>();

export function loadGameIcon(
  name: string,
  kind: "font" | "sprite" = "font",
  options: ImageRenderOptions = {},
): Promise<string | undefined> {
  const outline = options.outline?.toLowerCase();
  const key = `${kind}:${name}:${outline ?? "original"}:${options.crop ? "crop" : "full"}`;
  const existing = cache.get(key);
  if (existing) return existing;
  const result = (async () => {
    const pack = await gameAssets();
    const icons = pack.catalog<Icons>("icons");
    let hash: string | undefined;
    if (kind === "sprite") hash = icons.sprites[name]?.image;
    else {
      const candidates = icons.fontIcons[name] ?? [];
      const defaultId = icons.spriteAssets.find((entry) => entry.isDefault)?.id;
      hash =
        candidates.find((entry) => entry.assetId === defaultId)?.image ??
        (candidates.length === 1 ? candidates[0].image : undefined);
    }
    return hash ? pack.imageUrl(hash, options) : undefined;
  })().catch(() => undefined);
  cache.set(key, result);
  return result;
}

export interface AssetManifest {
  gameVersion?: string;
  product?: string;
  status?: string;
  exportedAtUtc?: string;
}

export async function loadAssetManifest(): Promise<AssetManifest> {
  return (await gameAssets()).catalog<AssetManifest>("manifest");
}

const dayIcons = [
  "day_pride",
  "day_envy",
  "day_gluttony",
  "day_lust",
  "day_wrath",
  "day_sloth",
];

export function gameDayIcon(day: unknown): string {
  const value = Number(day);
  return Number.isFinite(value)
    ? dayIcons[
        (((Math.trunc(value) - 1) % dayIcons.length) + dayIcons.length) %
          dayIcons.length
      ]
    : "day_pride";
}
