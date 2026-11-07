import { shell } from "./pt-br/shell";
import { settings } from "./pt-br/settings";
import { loadSaves } from "./pt-br/load-saves";
import { about } from "./pt-br/about";
import { general } from "./pt-br/general";
import { inventory } from "./pt-br/inventory";
import { progression } from "./pt-br/progression";
import { zombies } from "./pt-br/zombies";
import { inspector } from "./pt-br/inspector";

/**
 * Brazilian Portuguese interface strings, keyed by the English source text.
 * Grouped by editor area; later entries win when areas share a string.
 */
export const ptBR: Record<string, string> = {
  ...shell,
  ...settings,
  ...loadSaves,
  ...about,
  ...general,
  ...inventory,
  ...progression,
  ...zombies,
  ...inspector,
};
