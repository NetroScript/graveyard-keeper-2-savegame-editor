import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { packAssets } from "../../scripts/pack-assets.mjs";
import { parsePack, replaceBlue } from "../../src/lib/assets/pack.ts";

test("packs shared images once, drops controller icons, and rejects corruption", async () => {
  const root = await mkdtemp(join(tmpdir(), "gk2-pack-"));
  const input = join(root, "input");
  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j5fcAAAAASUVORK5CYII=",
    "base64",
  );
  const hash = createHash("sha256").update(png).digest("hex");
  try {
    await mkdir(join(input, "images"), { recursive: true });
    await writeFile(join(input, "images", hash + ".png"), png);
    const write = (file, data) =>
      writeFile(join(input, file + ".json"), JSON.stringify(data));
    await Promise.all([
      write("manifest", {
        schemaVersion: 1,
        status: "complete",
        completedModules: [
          "items",
          "resources",
          "progression",
          "inventory-rules",
          "font-icons",
        ],
      }),
      write("item-ids", ["test"]),
      write("items", {
        test: {
          id: "test",
          fields: { id: "test" },
          sprite: "item",
          quality: { type: "None" },
          fontIconNames: [],
          relatedPerks: [],
        },
      }),
      write("quality-families", {}),
      write("inventory-rules", { bags: {} }),
      write("item-definition-schema", { fields: [{ name: "id" }] }),
      write("resources", []),
      write("progression", {
        schemaVersion: 1,
        technology: {
          tabs: [{ id: "building", sprite: "item" }],
          nodes: [],
        },
        talents: {
          branches: [{ id: "talent", fontIcon: "energy" }],
          expLevels: [],
          inspirations: [],
          levelUps: [],
        },
      }),
      write("localization.en", { test: "Test item" }),
      write("icons", {
        images: { [hash]: { path: `images/${hash}.png`, width: 1, height: 1 } },
        sprites: { item: { image: hash }, alias: { image: hash } },
        fontIcons: {
          xbox_a: [{ image: hash }],
          switch_b: [{ image: hash }],
          ps_x: [{ image: hash }],
          energy: [{ image: hash }],
        },
      }),
    ]);
    const output = join(root, "game.gk2pack");
    assert.equal((await packAssets(input, output)).images, 1);
    const bytes = await readFile(output);
    const pack = parsePack(bytes);
    assert.deepEqual(Buffer.from(pack.imageBytes(hash)), png);
    assert.deepEqual(Object.keys(pack.metadata.catalogs.icons.fontIcons), [
      "energy",
    ]);
    assert.equal(pack.metadata.catalogs.items.test.fields.id, "test");
    assert.equal(pack.metadata.catalogs.icons.images[hash].path, undefined);
    await packAssets(input, output);
    assert.deepEqual(await readFile(output), bytes, "Packing is deterministic");
    assert.throws(() => parsePack(bytes.subarray(0, bytes.length - 1)));
    const bad = Buffer.from(bytes);
    bad.writeUInt32LE(99, 8);
    assert.throws(() => parsePack(bad), /version/);
    assert.throws(() => pack.imageBytes("missing"), /Unknown image/);
    await writeFile(join(input, "images", hash + ".png"), "corrupt");
    await assert.rejects(packAssets(input, output), /validation failed/);
    await assert.rejects(
      packAssets(input, output, { allowPartial: true }),
      /Corrupt image/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("outline replacement preserves alpha and all non-key colors", () => {
  const pixels = new Uint8ClampedArray([
    0, 0, 255, 128, 0, 1, 255, 255, 255, 0, 0, 255,
  ]);
  replaceBlue(pixels, [10, 20, 30]);
  assert.deepEqual(
    [...pixels],
    [10, 20, 30, 128, 0, 1, 255, 255, 255, 0, 0, 255],
  );
});
