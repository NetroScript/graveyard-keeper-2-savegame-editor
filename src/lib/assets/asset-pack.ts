import { applyStripLut, parsePack, replaceBlue } from "./pack";

export interface ImageRenderOptions {
  /** Replace the game's pure-blue shader mask with this CSS hex color. */
  outline?: string;
  /** Remove fully transparent outer rows and columns before creating the URL. */
  crop?: boolean;
  /** Apply a packed 1024x32 Unity LUT image before other color operations. */
  lut?: string;
}

export interface CompositeImageLayer {
  hash: string;
  pivot: { x: number; y: number };
  lut?: string;
}

export interface CompositeRenderOptions {
  crop?: boolean;
}

/** A small canvas pool bounds concurrent decoding while avoiding a serial queue. */
export class AssetPack {
  readonly pack: ReturnType<typeof parsePack>;
  private readonly canvasLimit = 4;
  private canvasCount = 0;
  private canvases: HTMLCanvasElement[] = [];
  private canvasWaiters: {
    resolve: (canvas: HTMLCanvasElement) => void;
    reject: (error: Error) => void;
  }[] = [];
  private cache = new Map<string, Promise<string>>();
  private layerPixels = new Map<string, Promise<ImageData>>();
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

  private acquireCanvas(): Promise<HTMLCanvasElement> {
    if (this.disposed) return Promise.reject(new Error("Asset pack disposed"));
    const available = this.canvases.pop();
    if (available) return Promise.resolve(available);
    if (this.canvasCount < this.canvasLimit) {
      this.canvasCount++;
      return Promise.resolve(document.createElement("canvas"));
    }
    return new Promise((resolve, reject) => {
      this.canvasWaiters.push({ resolve, reject });
    });
  }

  private releaseCanvas(canvas: HTMLCanvasElement) {
    if (this.disposed) {
      canvas.width = 0;
      canvas.height = 0;
      return;
    }
    const waiter = this.canvasWaiters.shift();
    if (waiter) waiter.resolve(canvas);
    else this.canvases.push(canvas);
  }

  /** String arguments remain supported for existing item-outline callers. */
  imageUrl(
    hash: string,
    options?: string | ImageRenderOptions,
  ): Promise<string> {
    if (this.disposed) return Promise.reject(new Error("Asset pack disposed"));
    const outline = typeof options === "string" ? options : options?.outline;
    const crop = typeof options === "object" && options.crop === true;
    const lut = typeof options === "object" ? options.lut : undefined;
    if (outline !== undefined && !/^#[0-9a-f]{6}$/i.test(outline))
      return Promise.reject(new Error("Outline must be #RRGGBB"));
    const color = outline?.toLowerCase();
    const key = `${hash}:${color ?? "original"}:${crop ? "crop" : "full"}:lut=${lut ?? "none"}`;
    const cached = this.cache.get(key);
    if (cached) return cached;
    const result = (async () => {
      if (this.disposed) throw new Error("Asset pack disposed");
      const source = new Blob([new Uint8Array(this.pack.imageBytes(hash))], {
        type: "image/png",
      });
      let blob = source;
      if (color || crop || lut) {
        const canvas = await this.acquireCanvas();
        let bitmap: ImageBitmap | undefined;
        let lutBitmap: ImageBitmap | undefined;
        try {
          bitmap = await createImageBitmap(source);
          if (lut) {
            const lutSource = new Blob(
              [new Uint8Array(this.pack.imageBytes(lut))],
              { type: "image/png" },
            );
            lutBitmap = await createImageBitmap(lutSource);
          }
          if (this.disposed) throw new Error("Asset pack disposed");
          canvas.width = Math.max(bitmap.width, lutBitmap?.width ?? 0);
          canvas.height = bitmap.height + (lutBitmap?.height ?? 0);
          const ctx = canvas.getContext("2d", { willReadFrequently: true });
          if (!ctx) throw new Error("Canvas rendering is unavailable");
          ctx.imageSmoothingEnabled = false;
          ctx.clearRect(0, 0, canvas.width, canvas.height);
          ctx.drawImage(bitmap, 0, 0);
          const pixels = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
          if (lutBitmap) {
            ctx.drawImage(lutBitmap, 0, bitmap.height);
            const lutPixels = ctx.getImageData(
              0,
              bitmap.height,
              lutBitmap.width,
              lutBitmap.height,
            );
            applyStripLut(
              pixels.data,
              lutPixels.data,
              lutBitmap.width,
              lutBitmap.height,
            );
          }
          if (color)
            replaceBlue(pixels.data, [
              parseInt(color.slice(1, 3), 16),
              parseInt(color.slice(3, 5), 16),
              parseInt(color.slice(5, 7), 16),
            ]);
          canvas.width = bitmap.width;
          canvas.height = bitmap.height;
          const sourceContext = canvas.getContext("2d");
          if (!sourceContext)
            throw new Error("Canvas rendering is unavailable");
          sourceContext.putImageData(pixels, 0, 0);
          let left = 0;
          let top = 0;
          let width = canvas.width;
          let height = canvas.height;
          if (crop) {
            let right = -1;
            let bottom = -1;
            left = canvas.width;
            top = canvas.height;
            for (let y = 0; y < canvas.height; y++)
              for (let x = 0; x < canvas.width; x++)
                if (pixels.data[(y * canvas.width + x) * 4 + 3] !== 0) {
                  left = Math.min(left, x);
                  top = Math.min(top, y);
                  right = Math.max(right, x);
                  bottom = Math.max(bottom, y);
                }
            if (right >= left && bottom >= top) {
              width = right - left + 1;
              height = bottom - top + 1;
            } else {
              left = 0;
              top = 0;
            }
          }
          const rendered = ctx.getImageData(left, top, width, height);
          canvas.width = width;
          canvas.height = height;
          const output = canvas.getContext("2d");
          if (!output) throw new Error("Canvas rendering is unavailable");
          output.putImageData(rendered, 0, 0);
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
          bitmap?.close();
          lutBitmap?.close();
          this.releaseCanvas(canvas);
        }
      }
      if (this.disposed) throw new Error("Asset pack disposed");
      const url = URL.createObjectURL(blob);
      this.urls.add(url);
      return url;
    })();
    this.cache.set(key, result);
    result.catch(() => {
      this.cache.delete(key);
    });
    return result;
  }

  /** Composes Unity sprite layers by their bottom-left pixel pivots. */
  compositeImageUrl(
    layers: readonly CompositeImageLayer[],
    options: CompositeRenderOptions = {},
  ): Promise<string> {
    if (this.disposed) return Promise.reject(new Error("Asset pack disposed"));
    if (!layers.length) return Promise.reject(new Error("No image layers"));
    const key = `composite:${layers.map((layer) => `${layer.hash}@${layer.pivot.x},${layer.pivot.y}:lut=${layer.lut ?? "none"}`).join("|")}:crop=${options.crop === true}`;
    const cached = this.cache.get(key);
    if (cached) return cached;
    const result = (async () => {
      const pixels = await Promise.all(
        layers.map((layer) => this.compositeLayerPixels(layer)),
      );
      const bitmaps = await Promise.all(
        pixels.map((layer) => createImageBitmap(layer)),
      );
      const left = Math.floor(
        Math.min(...layers.map((layer) => -layer.pivot.x)),
      );
      const right = Math.ceil(
        Math.max(
          ...layers.map((layer, index) => bitmaps[index].width - layer.pivot.x),
        ),
      );
      const bottom = Math.floor(
        Math.min(...layers.map((layer) => -layer.pivot.y)),
      );
      const top = Math.ceil(
        Math.max(
          ...layers.map(
            (layer, index) => bitmaps[index].height - layer.pivot.y,
          ),
        ),
      );
      const canvas = await this.acquireCanvas();
      try {
        if (this.disposed) throw new Error("Asset pack disposed");
        canvas.width = right - left;
        canvas.height = top - bottom;
        const ctx = canvas.getContext("2d");
        if (!ctx) throw new Error("Canvas rendering is unavailable");
        ctx.imageSmoothingEnabled = false;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        const anchorX = -left;
        const anchorY = top;
        for (let index = 0; index < layers.length; index++) {
          const layer = layers[index];
          const bitmap = bitmaps[index];
          ctx.drawImage(
            bitmap,
            anchorX - layer.pivot.x,
            anchorY - (bitmap.height - layer.pivot.y),
          );
        }
        if (options.crop) {
          const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height);
          let cropLeft = canvas.width;
          let cropTop = canvas.height;
          let cropRight = -1;
          let cropBottom = -1;
          for (let y = 0; y < canvas.height; y++)
            for (let x = 0; x < canvas.width; x++)
              if (pixels.data[(y * canvas.width + x) * 4 + 3] !== 0) {
                cropLeft = Math.min(cropLeft, x);
                cropTop = Math.min(cropTop, y);
                cropRight = Math.max(cropRight, x);
                cropBottom = Math.max(cropBottom, y);
              }
          if (cropRight >= cropLeft && cropBottom >= cropTop) {
            const cropped = ctx.getImageData(
              cropLeft,
              cropTop,
              cropRight - cropLeft + 1,
              cropBottom - cropTop + 1,
            );
            canvas.width = cropped.width;
            canvas.height = cropped.height;
            canvas.getContext("2d")?.putImageData(cropped, 0, 0);
          }
        }
        const blob = await new Promise<Blob>((resolve, reject) =>
          canvas.toBlob(
            (value) =>
              value ? resolve(value) : reject(new Error("PNG encoding failed")),
            "image/png",
          ),
        );
        if (this.disposed) throw new Error("Asset pack disposed");
        const url = URL.createObjectURL(blob);
        this.urls.add(url);
        return url;
      } finally {
        for (const bitmap of bitmaps) bitmap.close();
        this.releaseCanvas(canvas);
      }
    })();
    this.cache.set(key, result);
    result.catch(() => this.cache.delete(key));
    return result;
  }

  /** Cache decoded/LUT-applied layer pixels so appearance changes avoid PNG re-encoding. */
  private compositeLayerPixels(layer: CompositeImageLayer): Promise<ImageData> {
    const key = `${layer.hash}:lut=${layer.lut ?? "none"}`;
    const cached = this.layerPixels.get(key);
    if (cached) return cached;
    const result = (async () => {
      const source = new Blob(
        [new Uint8Array(this.pack.imageBytes(layer.hash))],
        { type: "image/png" },
      );
      const bitmap = await createImageBitmap(source);
      let lutBitmap: ImageBitmap | undefined;
      const canvas = await this.acquireCanvas();
      try {
        if (layer.lut) {
          const lutSource = new Blob(
            [new Uint8Array(this.pack.imageBytes(layer.lut))],
            { type: "image/png" },
          );
          lutBitmap = await createImageBitmap(lutSource);
        }
        canvas.width = Math.max(bitmap.width, lutBitmap?.width ?? 0);
        canvas.height = bitmap.height + (lutBitmap?.height ?? 0);
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (!ctx) throw new Error("Canvas rendering is unavailable");
        ctx.imageSmoothingEnabled = false;
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.drawImage(bitmap, 0, 0);
        const output = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
        if (lutBitmap) {
          ctx.drawImage(lutBitmap, 0, bitmap.height);
          const lut = ctx.getImageData(
            0,
            bitmap.height,
            lutBitmap.width,
            lutBitmap.height,
          );
          applyStripLut(
            output.data,
            lut.data,
            lutBitmap.width,
            lutBitmap.height,
          );
        }
        return output;
      } finally {
        bitmap.close();
        lutBitmap?.close();
        this.releaseCanvas(canvas);
      }
    })();
    this.layerPixels.set(key, result);
    result.catch(() => this.layerPixels.delete(key));
    return result;
  }

  dispose() {
    this.disposed = true;
    for (const url of this.urls) URL.revokeObjectURL(url);
    this.urls.clear();
    this.cache.clear();
    this.layerPixels.clear();
    for (const waiter of this.canvasWaiters)
      waiter.reject(new Error("Asset pack disposed"));
    this.canvasWaiters = [];
    for (const canvas of this.canvases) {
      canvas.width = 0;
      canvas.height = 0;
    }
    this.canvases = [];
  }
}
