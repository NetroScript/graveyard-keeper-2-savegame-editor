import { parsePack, replaceBlue } from "./pack";

/** One canvas per loaded pack. Generated URLs remain valid until dispose(). */
export class AssetPack {
  readonly pack: ReturnType<typeof parsePack>;
  private canvas: HTMLCanvasElement | undefined;
  private queue: Promise<unknown> = Promise.resolve();
  private cache = new Map<string, Promise<string>>();
  private urls = new Set<string>();
  private disposed = false;

  constructor(bytes: Uint8Array) {
    this.pack = parsePack(bytes);
  }

  static async load(url: string, signal?: AbortSignal) {
    const response = await fetch(url, { signal });
    if (!response.ok)
      throw new Error(`Could not load assets (${response.status})`);
    return new AssetPack(new Uint8Array(await response.arrayBuffer()));
  }

  /** Catalog names match exporter JSON basenames, including future modules. */
  catalog<T = unknown>(name: string): T {
    if (!Object.hasOwn(this.pack.metadata.catalogs, name))
      throw new Error(`Unknown asset catalog: ${name}`);
    return this.pack.metadata.catalogs[name] as T;
  }

  /** Pass #RRGGBB for item outlines; omit for unmodified icons. */
  imageUrl(hash: string, outline?: string): Promise<string> {
    if (this.disposed) return Promise.reject(new Error("Asset pack disposed"));
    if (outline !== undefined && !/^#[0-9a-f]{6}$/i.test(outline))
      return Promise.reject(new Error("Outline must be #RRGGBB"));
    const color = outline?.toLowerCase();
    const key = `${hash}:${color ?? "original"}`;
    const cached = this.cache.get(key);
    if (cached) return cached;
    // Serialize use of the canvas, including asynchronous PNG encoding.
    const result = this.queue.then(async () => {
      if (this.disposed) throw new Error("Asset pack disposed");
      const source = new Blob([new Uint8Array(this.pack.imageBytes(hash))], {
        type: "image/png",
      });
      let blob = source;
      if (color) {
        const bitmap = await createImageBitmap(source);
        try {
          if (this.disposed) throw new Error("Asset pack disposed");
          const canvas = (this.canvas ??= document.createElement("canvas"));
          canvas.width = bitmap.width;
          canvas.height = bitmap.height;
          const ctx = canvas.getContext("2d", { willReadFrequently: true });
          if (!ctx) throw new Error("Canvas rendering is unavailable");
          ctx.imageSmoothingEnabled = false;
          ctx.drawImage(bitmap, 0, 0);
          const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height);
          replaceBlue(pixels.data, [
            parseInt(color.slice(1, 3), 16),
            parseInt(color.slice(3, 5), 16),
            parseInt(color.slice(5, 7), 16),
          ]);
          ctx.putImageData(pixels, 0, 0);
          blob = await new Promise<Blob>((resolve, reject) =>
            canvas.toBlob(
              (value) =>
                value
                  ? resolve(value)
                  : reject(new Error("PNG encoding failed")),
              "image/png",
            ),
          );
        } finally {
          bitmap.close();
        }
      }
      if (this.disposed) throw new Error("Asset pack disposed");
      const url = URL.createObjectURL(blob);
      this.urls.add(url);
      return url;
    });
    this.cache.set(key, result);
    this.queue = result.catch(() => {
      this.cache.delete(key);
    });
    return result;
  }

  dispose() {
    this.disposed = true;
    for (const url of this.urls) URL.revokeObjectURL(url);
    this.urls.clear();
    this.cache.clear();
    if (this.canvas) {
      this.canvas.width = 0;
      this.canvas.height = 0;
      this.canvas = undefined;
    }
  }
}
