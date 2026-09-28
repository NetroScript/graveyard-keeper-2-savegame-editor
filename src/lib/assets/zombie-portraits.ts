import { gameAssets } from "./game-assets";
import type { CompositeImageLayer } from "./asset-pack";

interface SpriteEntry {
  image?: string;
  pivot?: { x: number; y: number };
}

interface IconsCatalog {
  sprites: Record<string, SpriteEntry>;
}

interface TextureChoice {
  name: string;
  texture: string;
}

interface PortraitVariant {
  id: number;
  sprite: string;
  overlaySprite?: string | null;
}

interface ZombieAppearanceSet {
  id: string;
  bodyVariants: PortraitVariant[];
  headVariants: PortraitVariant[];
  bodyLuts: TextureChoice[];
  headLuts: TextureChoice[];
}

interface ZombiesCatalog {
  customization: {
    available: boolean;
    sets: ZombieAppearanceSet[];
    portrait: { stoneSprite?: string | null };
  };
}

export interface ZombiePortraitSelection {
  setId?: string;
  bodyId: number;
  headId: number;
  bodyLut?: string;
  headLut?: string;
  includeStone?: boolean;
  crop?: boolean;
}

/** Renders the same static-down layers used by the game's UIWorkerIcon. */
export async function loadZombiePortrait({
  setId = "zombie_worker",
  bodyId,
  headId,
  bodyLut,
  headLut,
  includeStone = true,
  crop = true,
}: ZombiePortraitSelection): Promise<string> {
  const pack = await gameAssets();
  const zombies = pack.catalog<ZombiesCatalog>("zombies");
  const icons = pack.catalog<IconsCatalog>("icons");
  if (!zombies.customization.available)
    throw new Error("Zombie appearance assets are unavailable");
  const set = zombies.customization.sets.find((entry) => entry.id === setId);
  if (!set) throw new Error(`Unknown zombie appearance set: ${setId}`);
  const body = set.bodyVariants.find((entry) => entry.id === bodyId);
  const head = set.headVariants.find((entry) => entry.id === headId);
  if (!body) throw new Error(`Unknown ${setId} body: ${bodyId}`);
  if (!head) throw new Error(`Unknown ${setId} head: ${headId}`);

  const textureHash = (choices: TextureChoice[], name?: string) => {
    const texture = name
      ? choices.find((entry) => entry.name === name)
      : choices[0];
    if (name && !texture) throw new Error(`Unknown ${setId} LUT: ${name}`);
    if (!texture) return undefined;
    const hash = icons.sprites[texture.texture]?.image;
    if (!hash) throw new Error(`Missing LUT image: ${texture.texture}`);
    return hash;
  };
  const layer = (
    name: string | null | undefined,
    lut?: string,
  ): CompositeImageLayer | undefined => {
    if (!name) return undefined;
    const sprite = icons.sprites[name];
    if (!sprite?.image || !sprite.pivot)
      throw new Error(`Missing zombie portrait sprite: ${name}`);
    return { hash: sprite.image, pivot: sprite.pivot, lut };
  };

  const bodyLutHash = textureHash(set.bodyLuts, bodyLut);
  const headLutHash = textureHash(set.headLuts, headLut);
  const layers = [
    layer(body.sprite, bodyLutHash),
    layer(body.overlaySprite),
    includeStone
      ? layer(zombies.customization.portrait.stoneSprite)
      : undefined,
    layer(head.sprite, headLutHash),
  ].filter((entry): entry is CompositeImageLayer => entry !== undefined);
  return pack.compositeImageUrl(layers, { crop });
}
