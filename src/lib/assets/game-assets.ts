import { asset } from "$app/paths";
import { AssetPack } from "./asset-pack";

let loading: Promise<AssetPack> | undefined;
let loaded: AssetPack | undefined;
export function gameAssets(): Promise<AssetPack> {
  return (loading ??= AssetPack.load(asset("/assets/game.gk2pack"))
    .then((pack) => (loaded = pack))
    .catch((error) => {
      loading = undefined;
      throw error;
    }));
}
if (import.meta.hot) import.meta.hot.dispose(() => loaded?.dispose());
