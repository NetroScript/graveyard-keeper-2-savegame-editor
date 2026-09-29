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
  const [
    manifest,
    ids,
    items,
    families,
    rules,
    schema,
    icons,
    resources,
    progression,
    zombies,
  ] = await Promise.all(
    [
      "manifest.json",
      "item-ids.json",
      "items.json",
      "quality-families.json",
      "inventory-rules.json",
      "item-definition-schema.json",
      "icons.json",
      "resources.json",
      "progression.json",
      "zombies.json",
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
  for (const module of [
    "items",
    "resources",
    "progression",
    "zombies",
    "inventory-rules",
    "font-icons",
  ])
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
  check(
    progression.schemaVersion === 1,
    "Unsupported progression schema version",
  );
  const techIds = new Set(progression.technology.nodes.map((node) => node.id));
  const tabIds = new Set(progression.technology.tabs.map((tab) => tab.id));
  check(
    techIds.size === progression.technology.nodes.length,
    "Duplicate technology IDs",
  );
  for (const tab of progression.technology.tabs)
    sprite(tab.sprite, `technology tab ${tab.id}`);
  for (const node of progression.technology.nodes) {
    check(
      tabIds.has(node.tab),
      `${node.id}: unknown technology tab ${node.tab}`,
    );
    for (const parent of node.parents)
      check(
        techIds.has(parent),
        `${node.id}: unknown technology parent ${parent}`,
      );
    if (node.icon) sprite(node.icon, node.id);
    for (const reward of node.rewards)
      if (reward.sprite) sprite(reward.sprite, `${node.id}/${reward.id}`);
  }
  const branchIds = new Set(
    progression.talents.branches.map((branch) => branch.id),
  );
  const perkIds = new Set(Object.keys(progression.perks ?? {}));
  for (const [id, perk] of Object.entries(progression.perks ?? {})) {
    check(perk.id === id, `${id}: inconsistent perk ID`);
    sprite(perk.sprite, `perk ${id}`);
  }
  for (const branch of progression.talents.branches)
    font(branch.fontIcon, branch.id);
  const inspirationIds = new Set(
    progression.talents.inspirations.map((item) => item.id),
  );
  check(
    inspirationIds.size === progression.talents.inspirations.length,
    "Duplicate inspiration IDs",
  );
  for (const item of progression.talents.inspirations) {
    check(
      branchIds.has(item.talent),
      `${item.id}: unknown talent branch ${item.talent}`,
    );
    sprite(item.sprite, item.id);
  }
  const levelIds = new Set(progression.talents.levelUps.map((item) => item.id));
  check(
    levelIds.size === progression.talents.levelUps.length,
    "Duplicate talent level IDs",
  );
  for (const item of progression.talents.levelUps) {
    check(
      branchIds.has(item.talent),
      `${item.id}: unknown talent branch ${item.talent}`,
    );
    for (const parent of item.parents)
      check(
        levelIds.has(parent),
        `${item.id}: unknown talent parent ${parent}`,
      );
    sprite(item.sprite, item.id);
    if (item.perk?.sprite)
      sprite(item.perk.sprite, `${item.id}/${item.perk.id}`);
  }
  const zombieLevels = progression.talents.zombieLevelUps ?? [];
  const zombieLevelIds = new Set(zombieLevels.map((item) => item.id));
  check(
    zombieLevelIds.size === zombieLevels.length,
    "Duplicate zombie talent level IDs",
  );
  for (const item of zombieLevels) {
    check(
      branchIds.has(item.talent),
      `${item.id}: unknown zombie talent branch ${item.talent}`,
    );
    for (const parent of item.parents)
      check(
        zombieLevelIds.has(parent),
        `${item.id}: unknown zombie talent parent ${parent}`,
      );
    if (item.perkId)
      check(
        perkIds.has(item.perkId),
        `${item.id}: unknown zombie perk ${item.perkId}`,
      );
    sprite(item.sprite, item.id);
    if (item.perk?.sprite)
      sprite(item.perk.sprite, `${item.id}/${item.perk.id}`);
  }
  check(zombies.schemaVersion === 1, "Unsupported zombies schema version");
  check(
    Array.isArray(zombies.bodies) && zombies.bodies.length > 0,
    "Missing body definitions",
  );
  check(
    Array.isArray(zombies.crafts) && zombies.crafts.length > 0,
    "Missing craft definitions",
  );
  const knownItemOrGroup = (reference) =>
    reference?.groupType !== "None" ||
    !!items[reference?.id] ||
    !!families[reference?.id];
  for (const body of zombies.bodies) {
    check(
      !!items[body.linkedBodyItemId],
      `${body.id}: unknown linked body item ${body.linkedBodyItemId}`,
    );
    for (const id of [...body.parts, ...body.pockets, ...body.burialRewards])
      check(!!items[id], `${body.id}: unknown body item ${id}`);
    for (const id of [body.armorId, body.handsId].filter(Boolean))
      check(!!items[id], `${body.id}: unknown starting equipment ${id}`);
  }
  for (const craft of zombies.crafts) {
    for (const perk of craft.linkedPerks)
      check(perkIds.has(perk), `${craft.id}: unknown linked perk ${perk}`);
    for (const need of [
      ...craft.needs,
      ...craft.needsFromWgo,
      ...craft.removeItemsFromWgo,
    ])
      check(
        knownItemOrGroup(need),
        `${craft.id}: unknown needed item/group ${need.id}`,
      );
    for (const modifier of craft.zombieSpeedItems)
      check(
        !!items[modifier.type],
        `${craft.id}: unknown zombie speed item ${modifier.type}`,
      );
  }
  for (const station of zombies.workstations)
    if (station.craftIcon)
      sprite(station.craftIcon, `workstation ${station.id}`);
  if (zombies.customization.available) {
    for (const set of zombies.customization.sets) {
      check(
        set.bodyIds.length > 0 && set.headIds.length > 0,
        `${set.id}: empty zombie appearance IDs`,
      );
      check(
        set.bodyVariants.length === set.bodyIds.length,
        `${set.id}: incomplete body portrait variants`,
      );
      check(
        set.headVariants.length === set.headIds.length,
        `${set.id}: incomplete head portrait variants`,
      );
      for (const variant of set.bodyVariants) {
        check(
          set.bodyIds.includes(variant.id),
          `${set.id}: unknown body portrait ID ${variant.id}`,
        );
        sprite(variant.sprite, `${set.id} body ${variant.id}`);
        if (variant.overlaySprite)
          sprite(variant.overlaySprite, `${set.id} body overlay ${variant.id}`);
      }
      for (const variant of set.headVariants) {
        check(
          set.headIds.includes(variant.id),
          `${set.id}: unknown head portrait ID ${variant.id}`,
        );
        sprite(variant.sprite, `${set.id} head ${variant.id}`);
      }
      for (const lut of [...set.bodyLuts, ...set.headLuts])
        sprite(lut.texture, `${set.id}/${lut.name}`);
    }
    if (zombies.customization.portrait.stoneSprite)
      sprite(
        zombies.customization.portrait.stoneSprite,
        "zombie portrait stone layer",
      );
    for (const palette of [
      ...zombies.customization.fighterBodyPalettes,
      ...zombies.customization.fighterArmorPalettes,
    ])
      for (const texture of palette.textures)
        sprite(texture.texture, `fighter palette ${texture.name}`);
  } else check(false, "Zombie customization data is unavailable");
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
