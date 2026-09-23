import { gameAssets } from "./game-assets";

interface Icons {
  fontIcons: Record<string, { assetId: string; image: string }[]>;
  spriteAssets: { id: string; isDefault: boolean }[];
}
interface Resource {
  resource: string;
  configurations: { iconType: number; iconName: string }[];
}

// Stamina has no dedicated icon in the current catalog; use the energy symbol.
const resources: Record<string, string> = {
  hp: "hp",
  max_hp: "hp",
  energy: "energy",
  stamina: "energy",
  gold: "gld",
  silver: "slv",
  bronze: "brz",
  tech_red: "tech_red",
  tech_green: "tech_green",
  tech_blue: "tech_blue",
  insanity: "insanity",
  happiness: "happiness",
};

let loading: Promise<Record<string, string>> | undefined;

export function loadGeneralIcons(): Promise<Record<string, string>> {
  return (loading ??= (async () => {
    const loaded = await gameAssets();
    const icons = loaded.catalog<Icons>("icons");
    const configurations = loaded.catalog<Resource[]>("resources");
    const defaultId = icons.spriteAssets.find((asset) => asset.isDefault)?.id;
    const entries = await Promise.all(
      Object.entries(resources).map(async ([field, resource]) => {
        const name =
          configurations
            .find((entry) => entry.resource === resource)
            ?.configurations.find((config) => config.iconType === 0)
            ?.iconName ?? resource;
        const candidates = icons.fontIcons[name] ?? [];
        const icon =
          candidates.find((candidate) => candidate.assetId === defaultId) ??
          (candidates.length === 1 ? candidates[0] : undefined);
        if (!icon) throw new Error(`Missing General icon: ${name}`);
        return [field, await loaded.imageUrl(icon.image, { crop: true })] as const;
      }),
    );
    return Object.fromEntries(entries);
  })().catch((error) => {
    loading = undefined;
    throw error;
  }));
}
