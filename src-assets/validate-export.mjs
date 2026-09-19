import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";

// Offline validation deliberately has no Unity, Rust, or frontend dependency.
export async function validateExport(directory) {
  const errors = [];
  const root = resolve(directory);
  const load = async (name) =>
    JSON.parse(await readFile(resolve(root, name), "utf8"));
  const [manifest, ids, items, families, rules, schema, icons, resources] =
    await Promise.all(
      [
        "manifest.json",
        "item-ids.json",
        "items.json",
        "quality-families.json",
        "inventory-rules.json",
        "item-definition-schema.json",
        "icons.json",
        "resources.json",
      ].map(load),
    );
  const check = (condition, message) => {
    if (!condition) errors.push(message);
  };
  check(manifest.schemaVersion === 1, "Unsupported schema version");
  check(
    manifest.status === "complete",
    "Export is partial; inspect manifest warnings",
  );
  check(
    ids.length > 0 && new Set(ids).size === ids.length,
    "Empty or duplicate item ID list",
  );
  check(
    Object.keys(items).length === ids.length,
    "Item definitions are missing",
  );
  for (const module of ["items", "resources", "inventory-rules", "font-icons"])
    check(
      manifest.completedModules.includes(module),
      `Missing module: ${module}`,
    );
  const sprite = (name, source) => {
    check(
      typeof name === "string" && !!icons.sprites[name]?.image,
      `${source}: missing sprite ${name}`,
    );
  };
  const font = (name, source) => {
    check(
      !!icons.fontIcons[name]?.length,
      `${source}: missing font icon ${name}`,
    );
  };
  for (const id of ids) {
    const item = items[id];
    if (!item) {
      errors.push(`Missing item: ${id}`);
      continue;
    }
    check(item.id === id && item.fields.id === id, `${id}: inconsistent ID`);
    for (const field of schema.fields)
      check(
        Object.hasOwn(item.fields, field.name),
        `${id}: missing field ${field.name}`,
      );
    sprite(item.sprite, id);
    if (item.quality.type === "Star") {
      sprite(item.quality.overlaySprite, id);
      check(
        families[item.quality.family]?.some(
          (v) => v.id === id && v.quality === item.quality.value,
        ),
        `${id}: missing quality family`,
      );
    }
    for (const name of item.fontIconNames) font(name, id);
    for (const perk of item.relatedPerks)
      sprite(perk.sprite, `${id}/${perk.id}`);
  }
  for (const [family, variants] of Object.entries(families))
    for (const variant of variants)
      check(
        items[variant.id]?.quality.family === family,
        `Invalid quality member: ${variant.id}`,
      );
  for (const [bag, rule] of Object.entries(rules.bags)) {
    check(
      !!items[bag]?.fields.isBag && rule.complete,
      `Incomplete/invalid bag rule: ${bag}`,
    );
    for (const id of rule.allowedItemIds)
      check(
        !!items[id] && !items[id].fields.isBag,
        `${bag}: invalid allowed item ${id}`,
      );
  }
  for (const id of ids)
    if (items[id]?.fields.isBag)
      check(!!rules.bags[id], `Missing bag rule: ${id}`);
  for (const resource of resources)
    for (const configuration of resource.configurations)
      font(configuration.iconName, resource.resource);
  for (const [name, entry] of Object.entries(icons.sprites))
    check(!!icons.images[entry.image], `${name}: missing image reference`);
  for (const [name, entries] of Object.entries(icons.fontIcons))
    for (const entry of entries)
      check(!!icons.images[entry.image], `${name}: missing image reference`);
  for (const [hash, image] of Object.entries(icons.images)) {
    const path = resolve(root, image.path);
    if (!path.startsWith(root + sep)) {
      errors.push(`Image path escapes export: ${image.path}`);
      continue;
    }
    try {
      const data = await readFile(path);
      check(
        createHash("sha256").update(data).digest("hex") === hash,
        `${image.path}: hash mismatch`,
      );
      check(
        data.length >= 24 &&
          data
            .subarray(0, 8)
            .equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
        `${image.path}: invalid PNG header`,
      );
      if (data.length >= 24)
        check(
          data.readUInt32BE(16) === image.width &&
            data.readUInt32BE(20) === image.height,
          `${image.path}: incorrect dimensions`,
        );
    } catch (error) {
      errors.push(`${image.path}: ${error.message}`);
    }
  }
  return {
    itemCount: ids.length,
    imageCount: Object.keys(icons.images).length,
    errors,
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    if (!process.argv[2])
      throw new Error(
        "Usage: node src-assets/validate-export.mjs <export-directory>",
      );
    const result = await validateExport(process.argv[2]);
    console.log(
      `${result.itemCount} items, ${result.imageCount} images, ${result.errors.length} errors`,
    );
    for (const error of result.errors) console.error(error);
    if (result.errors.length) process.exitCode = 1;
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
