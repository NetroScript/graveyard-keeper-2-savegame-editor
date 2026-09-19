import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { validateExport } from "./validate-export.mjs";

test("checks catalog coverage, bag references, PNG integrity and partial exports", async () => {
  const root = await mkdtemp(join(tmpdir(), "gk2-export-test-"));
  const write = (name, data) =>
    writeFile(join(root, name), JSON.stringify(data));
  const png = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j5fcAAAAASUVORK5CYII=",
    "base64",
  );
  const hash = createHash("sha256").update(png).digest("hex");
  const manifest = {
    schemaVersion: 1,
    status: "complete",
    completedModules: ["items", "resources", "inventory-rules", "font-icons"],
  };
  const item = {
    id: "test",
    fields: { id: "test", isBag: false },
    sprite: "test-icon",
    quality: { type: "None" },
    fontIconNames: ["energy"],
    relatedPerks: [],
  };
  try {
    await mkdir(join(root, "images"));
    await writeFile(join(root, "images", hash + ".png"), png);
    await Promise.all([
      write("manifest.json", manifest),
      write("item-ids.json", ["test"]),
      write("items.json", { test: item }),
      write("quality-families.json", {}),
      write("inventory-rules.json", { bags: {} }),
      write("item-definition-schema.json", {
        fields: [{ name: "id" }, { name: "isBag" }],
      }),
      write("resources.json", [
        { resource: "energy", configurations: [{ iconName: "energy" }] },
      ]),
      write("icons.json", {
        images: { [hash]: { path: `images/${hash}.png`, width: 1, height: 1 } },
        sprites: { "test-icon": { image: hash } },
        fontIcons: { energy: [{ image: hash }] },
      }),
    ]);
    assert.deepEqual((await validateExport(root)).errors, []);
    await write("items.json", {});
    assert.ok(
      (await validateExport(root)).errors.includes("Missing item: test"),
    );
    await write("items.json", { test: item });
    await write("inventory-rules.json", {
      bags: { test: { complete: false, allowedItemIds: ["missing"] } },
    });
    assert.ok(
      (await validateExport(root)).errors.some((e) =>
        e.includes("invalid allowed item"),
      ),
    );
    await write("inventory-rules.json", { bags: {} });
    await write("manifest.json", { ...manifest, status: "partial" });
    assert.ok(
      (await validateExport(root)).errors.some((e) => e.includes("partial")),
    );
    await writeFile(join(root, "images", hash + ".png"), "broken");
    assert.ok(
      (await validateExport(root)).errors.some((e) =>
        e.includes("hash mismatch"),
      ),
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
