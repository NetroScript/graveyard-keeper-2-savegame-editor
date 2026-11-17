// Tauri 2.11.4 copies its cached AppRun with mode 0770 into the AppImage.
// Prepare that same launcher with permissions suitable for other users.
import { createHash } from "node:crypto";
import {
  chmod,
  mkdir,
  mkdtemp,
  readFile,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
import { homedir } from "node:os";
import { isAbsolute, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const launcherUrl =
  "https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64";
const launcherHash =
  "f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f";

export async function prepareAppImage(toolsDirectory) {
  await mkdir(toolsDirectory, { recursive: true });
  const launcher = join(toolsDirectory, "AppRun-x86_64");
  let bytes;
  try {
    bytes = await readFile(launcher);
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const cached = !!bytes;
  if (!cached) {
    const response = await fetch(launcherUrl);
    if (!response.ok)
      throw new Error(`AppRun download failed: ${response.status}`);
    bytes = Buffer.from(await response.arrayBuffer());
  }
  if (createHash("sha256").update(bytes).digest("hex") !== launcherHash)
    throw new Error(`Unexpected AppRun checksum: ${launcher}`);
  if (!cached) {
    const temporary = await mkdtemp(join(toolsDirectory, "apprun-"));
    try {
      const staged = join(temporary, "AppRun");
      await writeFile(staged, bytes, { mode: 0o755 });
      await chmod(staged, 0o755);
      await rename(staged, launcher);
    } finally {
      await rm(temporary, { recursive: true, force: true });
    }
  }
  await chmod(launcher, 0o755);
  console.log(`Prepared AppImage launcher: ${launcher} (0755)`);
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  if (process.platform === "linux") {
    const arch = process.env.TAURI_ENV_ARCH ?? process.arch;
    if (arch !== "x86_64" && arch !== "x64")
      throw new Error(`AppImage launcher preparation does not support ${arch}`);
    const cache = process.env.XDG_CACHE_HOME;
    await prepareAppImage(
      join(
        cache && isAbsolute(cache) ? cache : join(homedir(), ".cache"),
        "tauri",
      ),
    );
  }
}
