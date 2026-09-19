import { asset } from "$app/paths";
import { AssetPack } from "./asset-pack";

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
let pack: AssetPack | undefined;

export function loadGeneralIcons(): Promise<Record<string, string>> {
  return (loading ??= (async () => {
    const loaded = await AssetPack.load(asset("/assets/game.gk2pack"));
    try {
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
          return [field, await loaded.imageUrl(icon.image)] as const;
        }),
      );
      pack = loaded;
      return Object.fromEntries(entries);
    } catch (error) {
      loaded.dispose();
      throw error;
    }
  })().catch((error) => {
    loading = undefined;
    throw error;
  }));
}

// The pack lives for the application session, not for an individual save view.
if (import.meta.hot) import.meta.hot.dispose(() => pack?.dispose());
