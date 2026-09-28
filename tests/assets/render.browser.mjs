import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { chromium } from "@playwright/test";
import { createServer } from "vite";
import { encode } from "@msgpack/msgpack";
import { PACK_MAGIC } from "../../src/lib/assets/pack.ts";

test("a bounded canvas pool renders cached cropped variants and releases URLs", async () => {
  const server = await createServer({
    configFile: false,
    server: { host: "127.0.0.1", port: 0 },
    optimizeDeps: { include: ["@msgpack/msgpack"] },
    plugins: [
      {
        name: "asset-test-page",
        configureServer(server) {
          server.middlewares.use("/__asset_test__", (_req, res) => {
            res.setHeader("Content-Type", "text/html");
            res.end("<!doctype html><title>Asset test</title>");
          });
        },
      },
    ],
  });
  let browser;
  try {
    await server.listen();
    browser = await chromium.launch({
      channel: process.env.PLAYWRIGHT_CHANNEL || "chrome",
    });
    const page = await browser.newPage();
    await page.goto(
      `http://127.0.0.1:${server.httpServer.address().port}/__asset_test__`,
    );
    const encoded = await page.evaluate(() => {
      const canvas = document.createElement("canvas");
      canvas.width = 4;
      canvas.height = 3;
      const ctx = canvas.getContext("2d");
      ctx.fillStyle = "#0000ff";
      ctx.fillRect(1, 1, 1, 1);
      ctx.fillStyle = "#ff0000";
      ctx.fillRect(2, 1, 1, 1);
      const source = canvas.toDataURL().split(",")[1];
      canvas.width = 4;
      canvas.height = 2;
      const lut = canvas.getContext("2d");
      const pixels = lut.createImageData(4, 2);
      for (let green = 0; green < 2; green++)
        for (let blue = 0; blue < 2; blue++)
          for (let red = 0; red < 2; red++) {
            const offset = (((1 - green) * 4 + blue * 2 + red) * 4);
            pixels.data.set(
              [red * 255, green * 255, blue * 255, 255],
              offset,
            );
          }
      lut.putImageData(pixels, 0, 0);
      return { source, lut: canvas.toDataURL().split(",")[1] };
    });
    const png = Buffer.from(encoded.source, "base64");
    const lutPng = Buffer.from(encoded.lut, "base64");
    const hash = createHash("sha256").update(png).digest("hex");
    const lutHash = createHash("sha256").update(lutPng).digest("hex");
    const metadata = encode({
      schemaVersion: 1,
      catalogs: { perks: { example: { name: "Example perk" } } },
      images: {
        [hash]: { offset: 0, length: png.length, width: 4, height: 3 },
        [lutHash]: {
          offset: png.length,
          length: lutPng.length,
          width: 4,
          height: 2,
        },
      },
    });
    const header = Buffer.alloc(16);
    header.set(PACK_MAGIC);
    header.writeUInt32LE(1, 8);
    header.writeUInt32LE(metadata.length, 12);
    const result = await page.evaluate(
      async ({ bytes, hash, lutHash }) => {
        const { AssetPack } = await import("/src/lib/assets/asset-pack.ts");
        const original = document.createElement.bind(document);
        let canvases = 0;
        document.createElement = function (...args) {
          if (args[0] === "canvas") canvases++;
          return original(...args);
        };
        const assets = new AssetPack(new Uint8Array(bytes));
        const perkName = assets.catalog("perks").example.name;
        let missingCatalog = false;
        try {
          assets.catalog("missing");
        } catch {
          missingCatalog = true;
        }
        const first = assets.imageUrl(hash, {
          outline: "#101112",
          crop: true,
        });
        const samePromise =
          first ===
          assets.imageUrl(hash, { outline: "#101112", crop: true });
        const [normal, hover] = await Promise.all([
          first,
          assets.imageUrl(hash, { outline: "#abcdef", crop: true }),
        ]);
        const lutUrl = await assets.imageUrl(hash, { lut: lutHash, crop: true });
        const composite = await assets.compositeImageUrl([
          { hash, pivot: { x: 0, y: 0 }, lut: lutHash },
        ]);
        const rendererCanvases = canvases;
        async function pixels(url) {
          const bitmap = await createImageBitmap(
            await (await fetch(url)).blob(),
          );
          const canvas = original("canvas");
          canvas.width = bitmap.width;
          canvas.height = bitmap.height;
          const ctx = canvas.getContext("2d");
          ctx.drawImage(bitmap, 0, 0);
          bitmap.close();
          return {
            width: canvas.width,
            height: canvas.height,
            values: [...ctx.getImageData(0, 0, canvas.width, canvas.height).data],
          };
        }
        const normalPixels = await pixels(normal);
        const hoverPixels = await pixels(hover);
        const lutPixels = await pixels(lutUrl);
        const compositePixels = await pixels(composite);
        assets.dispose();
        let revoked = false;
        try {
          await fetch(normal);
        } catch {
          revoked = true;
        }
        let disposed = false;
        try {
          await assets.imageUrl(hash);
        } catch {
          disposed = true;
        }
        document.createElement = original;
        return {
          perkName,
          missingCatalog,
          samePromise,
          rendererCanvases,
          normalPixels,
          hoverPixels,
          lutPixels,
          compositePixels,
          revoked,
          disposed,
        };
      },
      {
        bytes: [...Buffer.concat([header, metadata, png, lutPng])],
        hash,
        lutHash,
      },
    );
    assert.deepEqual(result, {
      perkName: "Example perk",
      missingCatalog: true,
      samePromise: true,
      rendererCanvases: 2,
      normalPixels: {
        width: 2,
        height: 1,
        values: [16, 17, 18, 255, 255, 0, 0, 255],
      },
      hoverPixels: {
        width: 2,
        height: 1,
        values: [171, 205, 239, 255, 255, 0, 0, 255],
      },
      lutPixels: {
        width: 2,
        height: 1,
        values: [0, 0, 255, 255, 255, 0, 0, 255],
      },
      compositePixels: {
        width: 4,
        height: 3,
        values: [
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
          0, 0, 0, 0, 0, 0, 255, 255, 255, 0, 0, 255, 0, 0, 0, 0,
          0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
      },
      revoked: true,
      disposed: true,
    });
  } finally {
    await browser?.close();
    await server.close();
  }
});
