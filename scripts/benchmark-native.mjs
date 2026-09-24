// Benchmarks native Rust with the same local catalog used by the browser editor.
// The Rust example reads the real save but edits and exports only in memory.
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { decode } from "@msgpack/msgpack";

const [savePath, packPath = "static/assets/game.gk2pack"] = process.argv.slice(2);
if (!savePath) throw new Error("Usage: node scripts/benchmark-native.mjs <save.dat> [game.gk2pack]");
const pack = readFileSync(packPath);
const catalogs = decode(pack.subarray(16, 16 + pack.readUInt32LE(12))).catalogs;
const equipment = new Map(
  catalogs["inventory-rules"].equipment.map((entry) => [entry.id, entry]),
);
const catalog = {
  items: Object.fromEntries(Object.values(catalogs.items)
    .filter((item) => item.fields.stackCount >= 1)
    .map((item) => [item.id, {
      stack: item.fields.stackCount,
      family: item.quality.family,
      size: item.fields.itemSize,
      groups: item.fields.itemGroupIds,
      isBag: item.fields.isBag,
      capacity: item.fields.isBag ? item.fields.bagSize : item.fields.inventorySize,
      durability: item.fields.hasDurability,
      toolBelt:
        item.id === "hand_tool" || !!equipment.get(item.id)?.canBeEquipped,
      equipmentType: equipment.get(item.id)?.type ?? item.fields.type,
      allowed: catalogs["inventory-rules"].bags[item.id]?.complete
        ? catalogs["inventory-rules"].bags[item.id].allowedItemIds : null,
    }])),
};
const result = spawnSync("cargo", ["run", "-p", "gk2-save-core", "--example", "benchmark_edits", "--", savePath, "--catalog-stdin"], {
  input: JSON.stringify(catalog), encoding: "utf8", windowsHide: true,
});
if (result.error) throw result.error;
process.stdout.write(result.stdout);
process.stderr.write(result.stderr);
process.exitCode = result.status ?? 1;
