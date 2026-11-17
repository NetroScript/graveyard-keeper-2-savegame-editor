// Check the packaged filesystem, including permissions for users other than
// the build owner. Never change a completed AppImage: it may already be signed.
import assert from "node:assert/strict";
import { mkdtemp, readdir, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

if (process.platform !== "linux")
  throw new Error("AppImage verification requires Linux");
const images = process.argv.slice(2);
if (!images.length)
  throw new Error(
    "Usage: node scripts/verify-appimage.mjs <file.AppImage> [...]",
  );
for (const image of images) {
  const temporary = await mkdtemp(join(tmpdir(), "gk2-appimage-"));
  try {
    const result = spawnSync(resolve(image), ["--appimage-extract"], {
      cwd: temporary,
      stdio: ["ignore", "ignore", "pipe"],
      encoding: "utf8",
    });
    if (result.error) throw result.error;
    assert.equal(result.status, 0, result.stderr);
    const root = join(temporary, "squashfs-root");
    async function checkDirectory(directory) {
      // The AppImage runtime extracts directories as 0700, independently of
      // their SquashFS modes. Only file permissions survive this extraction.
      for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);
        const info = await stat(path); // Follow symlinks; broken links fail too.
        if (entry.isDirectory()) await checkDirectory(path);
        else
          assert.ok(
            info.mode & 0o004,
            `File is not readable by everyone: ${path}`,
          );
      }
    }
    for (const file of [
      "AppRun",
      "AppRun.wrapped",
      "usr/bin/graveyard-keeper-2-savegame-editor",
    ]) {
      const info = await stat(join(root, file));
      assert.equal(
        info.mode & 0o005,
        0o005,
        `Launcher is not executable by everyone: ${file}`,
      );
    }
    await checkDirectory(root);
    console.log(`Verified AppImage permissions: ${image}`);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}
