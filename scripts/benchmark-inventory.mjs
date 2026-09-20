// Read-only benchmark against a locally supplied save and asset pack.
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { decode } from "@msgpack/msgpack";
import { initSync, SaveSession } from "../static/wasm/gk2_save_wasm.js";

const [savePath, packPath = "static/assets/game.gk2pack"] =
  process.argv.slice(2);
if (!savePath)
  throw new Error(
    "Usage: node scripts/benchmark-inventory.mjs <save.dat> [game.gk2pack]",
  );
initSync({ module: readFileSync("static/wasm/gk2_save_wasm_bg.wasm") });
const bytes = readFileSync(savePath);
const pack = readFileSync(packPath);
const catalogs = decode(pack.subarray(16, 16 + pack.readUInt32LE(12))).catalogs;
const catalog = {
  items: Object.fromEntries(
    Object.values(catalogs.items)
      .filter((i) => i.fields.stackCount >= 1)
      .map((i) => [
        i.id,
        {
          stack: i.fields.stackCount,
          family: i.quality.family,
          size: i.fields.itemSize,
          groups: i.fields.itemGroupIds,
          isBag: i.fields.isBag,
          capacity: i.fields.isBag ? i.fields.bagSize : i.fields.inventorySize,
          durability: i.fields.hasDurability,
          allowed: catalogs["inventory-rules"].bags[i.id]?.complete
            ? catalogs["inventory-rules"].bags[i.id].allowedItemIds
            : null,
        },
      ]),
  ),
};
const session = new SaveSession();
const id = JSON.parse(session.open(bytes)).documentId;
function request(request, label) {
  const start = performance.now();
  const response = session.request(id, JSON.stringify(request));
  if (label)
    console.log(
      `${label}: ${(performance.now() - start).toFixed(2)} ms, ${Buffer.byteLength(response)} bytes`,
    );
  return JSON.parse(response);
}
request({ op: "inventory_catalog", catalog });
const snapshot = request({ op: "inventories" }, "Initial inventories");
const player = snapshot.inventories.find((i) => i.kind === "Player");
let revision = request({ op: "summary" }).revision;
function edit(action, label) {
  const result = request(
    {
      op: "transact",
      revision,
      operations: [
        {
          op: "inventory",
          container: player.node,
          out_of_bounds: true,
          action,
        },
      ],
    },
    label,
  );
  revision = result.summary.revision;
  assert.equal(result.inventory.upsert.length, 1);
  assert.equal(result.inventoryInvalidated, false);
  assert.equal(result.general, null);
  assert.equal(Object.keys(result.inventory.rules).length, 0);
  return result;
}
edit(
  { kind: "capacity", value: String(Number(player.capacity) + 1) },
  "Capacity",
);
const inserted = edit(
  {
    kind: "put",
    node: null,
    item: "cheese",
    count: "1",
    guid: crypto.randomUUID(),
  },
  "Insert item",
);
const item = inserted.inventory.upsert[0].items.at(-1);
edit(
  { kind: "put", node: item.node, item: "cheese", count: "2", guid: "unused" },
  "Item count",
);
const edited = session.export(id);
assert.equal(edited.length, request({ op: "summary" }).encodedBytes);
const reopened = JSON.parse(session.open(edited)).documentId;
assert.ok(reopened !== id);
for (let i = 0; i < 3; i++)
  revision = request({ op: "undo", revision }, "Undo").summary.revision;
assert.deepEqual(Buffer.from(session.export(id)), bytes);
console.log("Edited export reopens; undo restores original bytes.");
