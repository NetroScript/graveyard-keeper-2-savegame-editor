import { decode } from "@msgpack/msgpack";

export const PACK_MAGIC = new Uint8Array([71, 75, 50, 80, 65, 67, 75, 0]);
export const PACK_VERSION = 1;
export const HEADER_SIZE = 16;
export const MAX_METADATA = 64 * 1024 * 1024;
export const MAX_PACK = 512 * 1024 * 1024;

export interface PackedImage {
  offset: number;
  length: number;
  width: number;
  height: number;
}

export interface PackMetadata {
  schemaVersion: 1;
  catalogs: Record<string, unknown>;
  images: Record<string, PackedImage>;
}

export function parsePack(bytes: Uint8Array) {
  if (bytes.length < HEADER_SIZE || bytes.length > MAX_PACK)
    throw new Error("Invalid asset pack size");
  if (!PACK_MAGIC.every((value, i) => bytes[i] === value))
    throw new Error("Invalid asset pack signature");
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (view.getUint32(8, true) !== PACK_VERSION)
    throw new Error("Unsupported asset pack version");
  const length = view.getUint32(12, true);
  const start = HEADER_SIZE + length;
  if (!length || length > MAX_METADATA || start > bytes.length)
    throw new Error("Invalid asset metadata length");
  const metadata = decode(bytes.subarray(HEADER_SIZE, start), {
    maxStrLength: MAX_METADATA,
    maxBinLength: MAX_METADATA,
    maxArrayLength: 1_000_000,
    maxMapLength: 1_000_000,
  }) as PackMetadata;
  if (
    !metadata ||
    metadata.schemaVersion !== 1 ||
    !metadata.catalogs ||
    typeof metadata.catalogs !== "object" ||
    Array.isArray(metadata.catalogs) ||
    !metadata.images ||
    typeof metadata.images !== "object" ||
    Array.isArray(metadata.images)
  )
    throw new Error("Invalid asset metadata schema");
  for (const [hash, image] of Object.entries(metadata.images)) {
    if (
      !/^[a-f0-9]{64}$/.test(hash) ||
      !image ||
      ![image.offset, image.length, image.width, image.height].every(
        Number.isSafeInteger,
      ) ||
      image.offset < 0 ||
      image.length < 24 ||
      image.offset + image.length > bytes.length - start ||
      image.width <= 0 ||
      image.height <= 0 ||
      image.width * image.height > 16_777_216
    )
      throw new Error(`Invalid packed image: ${hash}`);
    const png = bytes.subarray(
      start + image.offset,
      start + image.offset + image.length,
    );
    if (![137, 80, 78, 71, 13, 10, 26, 10].every((v, i) => png[i] === v))
      throw new Error(`Invalid PNG: ${hash}`);
    const header = new DataView(png.buffer, png.byteOffset, png.byteLength);
    if (
      header.getUint32(16) !== image.width ||
      header.getUint32(20) !== image.height
    )
      throw new Error(`Incorrect image dimensions: ${hash}`);
  }
  return {
    metadata,
    imageBytes(hash: string) {
      if (!Object.hasOwn(metadata.images, hash))
        throw new Error(`Unknown image: ${hash}`);
      const image = metadata.images[hash];
      return bytes.subarray(
        start + image.offset,
        start + image.offset + image.length,
      );
    },
  };
}

// Explicit editor rule, not a claim to reproduce the game's unavailable shader.
// Apply only to sprites known to use blue replacement; never globally to TMP icons.
export function replaceBlue(
  pixels: Uint8ClampedArray,
  color: readonly [number, number, number],
) {
  for (let i = 0; i < pixels.length; i += 4) {
    if (pixels[i] === 0 && pixels[i + 1] === 0 && pixels[i + 2] === 255) {
      pixels[i] = color[0];
      pixels[i + 1] = color[1];
      pixels[i + 2] = color[2];
    }
  }
}
