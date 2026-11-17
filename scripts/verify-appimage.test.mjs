import assert from "node:assert/strict";
import { chmod, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

test(
  "AppImage checks reject owner-only launchers and unreadable content",
  {
    skip: process.platform !== "linux",
  },
  async () => {
    const directory = await mkdtemp(join(tmpdir(), "gk2-appimage-test-"));
    const image = join(directory, "fixture.AppImage");
    try {
      // A small extraction fixture exercises the same filesystem checks as an
      // AppImage without downloading or committing a game application bundle.
      async function verify(launcherMode, dataMode = "644") {
        await writeFile(
          image,
          `#!/bin/sh
set -eu
test "$1" = --appimage-extract
mkdir -p squashfs-root/usr/bin squashfs-root/data
printf '#!/bin/sh\\nexit 0\\n' > squashfs-root/AppRun
cp squashfs-root/AppRun squashfs-root/AppRun.wrapped
cp squashfs-root/AppRun squashfs-root/usr/bin/graveyard-keeper-2-savegame-editor
printf data > squashfs-root/data/catalog
chmod 755 squashfs-root squashfs-root/usr squashfs-root/usr/bin squashfs-root/AppRun squashfs-root/usr/bin/graveyard-keeper-2-savegame-editor
chmod ${launcherMode} squashfs-root/AppRun.wrapped
chmod ${dataMode} squashfs-root/data/catalog
chmod 700 squashfs-root/data
`,
        );
        await chmod(image, 0o755);
        return spawnSync(
          process.execPath,
          [resolve(import.meta.dirname, "verify-appimage.mjs"), image],
          { encoding: "utf8" },
        );
      }
      const broken = await verify("770");
      assert.notEqual(broken.status, 0);
      assert.match(
        broken.stderr,
        /Launcher is not executable by everyone: AppRun.wrapped/,
      );
      const fixed = await verify("755");
      assert.equal(fixed.status, 0, fixed.stderr);
      const unreadable = await verify("755", "640");
      assert.notEqual(unreadable.status, 0);
      assert.match(unreadable.stderr, /File is not readable by everyone/);
      const notExecutable = await verify("774");
      assert.notEqual(notExecutable.status, 0);
      assert.match(
        notExecutable.stderr,
        /Launcher is not executable by everyone/,
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  },
);
