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

/**
 * The packer gzips the whole pack, mostly for the MessagePack metadata; PNG
 * payloads barely shrink. Uncompressed packs are returned unchanged.
 */
export async function inflatePack(bytes: Uint8Array): Promise<Uint8Array> {
  if (bytes[0] !== 0x1f || bytes[1] !== 0x8b) return bytes;
  if (typeof DecompressionStream === "undefined") {
    // WebKitGTK before 2.48 and Safari before 16.4 lack DecompressionStream.
    const { gunzipSync } = await import("fflate");
    return gunzipSync(bytes);
  }
  const stream = new Blob([new Uint8Array(bytes)])
    .stream()
    .pipeThrough(new DecompressionStream("gzip"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
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

/**
 * Applies Unity's flattened 3D LUT layout: blue slices run horizontally, red
 * runs within a slice, and green runs upward. PNG/canvas rows run downward.
 */
export function applyStripLut(
  pixels: Uint8ClampedArray,
  lut: Uint8ClampedArray,
  width: number,
  height: number,
) {
  const size = height;
  if (
    !Number.isInteger(size) ||
    size < 2 ||
    width !== size * size ||
    lut.length !== width * height * 4
  )
    throw new Error("Invalid horizontal 3D LUT");
  const last = size - 1;
  const sample = (red: number, green: number, blue: number, channel: number) => {
    const index =
      (((last - green) * width + blue * size + red) * 4) + channel;
    return lut[index];
  };
  for (let offset = 0; offset < pixels.length; offset += 4) {
    if (pixels[offset + 3] === 0) continue;
    const red = (pixels[offset] / 255) * last;
    const green = (pixels[offset + 1] / 255) * last;
    const blue = (pixels[offset + 2] / 255) * last;
    const r0 = Math.floor(red), r1 = Math.min(last, r0 + 1), rf = red - r0;
    const g0 = Math.floor(green), g1 = Math.min(last, g0 + 1), gf = green - g0;
    const b0 = Math.floor(blue), b1 = Math.min(last, b0 + 1), bf = blue - b0;
    for (let channel = 0; channel < 3; channel++) {
      const c000 = sample(r0, g0, b0, channel);
      const c100 = sample(r1, g0, b0, channel);
      const c010 = sample(r0, g1, b0, channel);
      const c110 = sample(r1, g1, b0, channel);
      const c001 = sample(r0, g0, b1, channel);
      const c101 = sample(r1, g0, b1, channel);
      const c011 = sample(r0, g1, b1, channel);
      const c111 = sample(r1, g1, b1, channel);
      const c00 = c000 + (c100 - c000) * rf;
      const c10 = c010 + (c110 - c010) * rf;
      const c01 = c001 + (c101 - c001) * rf;
      const c11 = c011 + (c111 - c011) * rf;
      const c0 = c00 + (c10 - c00) * gf;
      const c1 = c01 + (c11 - c01) * gf;
      pixels[offset + channel] = Math.round(c0 + (c1 - c0) * bf);
    }
  }
}
