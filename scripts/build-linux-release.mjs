// tauri-action invokes this as "node ... build <args>". Verify the AppImage
// before returning control to the action, which then uploads artifacts.
import { readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
function run(script, args) {
  const result = spawnSync(process.execPath, [script, ...args], {
    cwd: root,
    stdio: "inherit",
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
if (process.platform !== "linux")
  throw new Error("This release wrapper requires Linux");
run(
  resolve(root, "node_modules/@tauri-apps/cli/tauri.js"),
  process.argv.slice(2),
);
run(resolve(root, "scripts/verify-appimage.test.mjs"), []);
const directory = resolve(root, "src-tauri/target/release/bundle/appimage");
const images = (await readdir(directory))
  .filter((name) => name.endsWith(".AppImage"))
  .map((name) => resolve(directory, name));
run(resolve(root, "scripts/verify-appimage.mjs"), images);
