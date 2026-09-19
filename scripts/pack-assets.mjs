import { readFile, readdir, mkdir, writeFile } from "node:fs/promises";
import { resolve, dirname, relative, isAbsolute } from "node:path";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";
import { encode } from "@msgpack/msgpack";
import { validateExport } from "../src-assets/validate-export.mjs";
import {
  PACK_MAGIC,
  PACK_VERSION,
  HEADER_SIZE,
  MAX_METADATA,
  MAX_PACK,
  parsePack,
} from "../src/lib/assets/pack.ts";

export async function packAssets(input, output, { allowPartial = false } = {}) {
  const validation = await validateExport(input);
  if (validation.errors.length && !allowPartial)
    throw new Error(
      `Export validation failed:\n${validation.errors.join("\n")}`,
    );
  const catalogs = {};
  for (const name of (await readdir(input))
    .filter((n) => n.endsWith(".json"))
    .sort())
    catalogs[name.slice(0, -5)] = JSON.parse(
      await readFile(resolve(input, name), "utf8"),
    );
  if (validation.errors.length) catalogs.packWarnings = validation.errors;
  const icons = catalogs.icons;
  // Also remove controller prompts from dumps made by older exporter versions.
  for (const name of Object.keys(icons.fontIcons))
    if (/^(xbox_|switch_|ps_)/i.test(name)) delete icons.fontIcons[name];
  const used = new Set(
    [
      ...Object.values(icons.sprites).map((s) => s.image),
      ...Object.values(icons.fontIcons).flatMap((entries) =>
        entries.map((e) => e.image),
      ),
    ].filter(Boolean),
  );
  const images = {};
  const chunks = [];
  let offset = 0;
  for (const hash of [...used].sort()) {
    const entry = icons.images[hash];
    if (!entry || !/^[a-f0-9]{64}$/.test(hash))
      throw new Error(`Missing image ${hash}`);
    // Validation above checks source paths; repeat confinement even in partial mode.
    const path = resolve(input, entry.path);
    const rel = relative(resolve(input), path);
    if (
      isAbsolute(rel) ||
      rel === ".." ||
      rel.startsWith("../") ||
      rel.startsWith("..\\")
    )
      throw new Error("Image path escapes export");
    const bytes = await readFile(path);
    if (createHash("sha256").update(bytes).digest("hex") !== hash)
      throw new Error(`Corrupt image ${hash}`);
    images[hash] = {
      offset,
      length: bytes.length,
      width: entry.width,
      height: entry.height,
    };
    chunks.push(bytes);
    offset += bytes.length;
  }
  // No source filenames or loose images are required at runtime.
  icons.images = Object.fromEntries(
    Object.entries(images).map(([hash, image]) => [
      hash,
      { width: image.width, height: image.height },
    ]),
  );
  const metadata = encode(
    { schemaVersion: 1, catalogs, images },
    { sortKeys: true },
  );
  if (
    metadata.length > MAX_METADATA ||
    HEADER_SIZE + metadata.length + offset > MAX_PACK
  )
    throw new Error("Asset pack exceeds size limit");
  const header = Buffer.alloc(HEADER_SIZE);
  header.set(PACK_MAGIC);
  header.writeUInt32LE(PACK_VERSION, 8);
  header.writeUInt32LE(metadata.length, 12);
  const result = Buffer.concat([header, metadata, ...chunks]);
  parsePack(result); // Check with the same reader used by the application.
  await mkdir(dirname(resolve(output)), { recursive: true });
  await writeFile(output, result);
  return {
    bytes: result.length,
    images: chunks.length,
    warnings: validation.errors,
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    const args = process.argv.slice(2);
    const paths = args.filter((a) => a !== "--allow-partial");
    if (paths.length < 1 || paths.length > 2)
      throw new Error(
        "Usage: pnpm pack:assets <export-directory> [output.gk2pack] [--allow-partial]",
      );
    const result = await packAssets(
      paths[0],
      paths[1] ?? "static/assets/game.gk2pack",
      { allowPartial: args.includes("--allow-partial") },
    );
    console.log(
      `Packed ${result.images} images into ${result.bytes} bytes. Warnings: ${result.warnings.length}`,
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
