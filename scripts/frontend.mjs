// Cross-platform environment selection; avoids shell-specific VAR=value syntax.
import { spawn } from "node:child_process";
import { cpSync, existsSync, mkdirSync, watch } from "node:fs";
import { fileURLToPath } from "node:url";

const [target, action, ...args] = process.argv.slice(2);
if (
  !["browser", "desktop"].includes(target) ||
  !["dev", "build", "preview"].includes(action)
) {
  throw new Error(
    "Usage: frontend.mjs <browser|desktop> <dev|build|preview> [Vite options]",
  );
}
const source = fileURLToPath(
  new URL("../static/assets/game.gk2pack", import.meta.url),
);
const destination = fileURLToPath(
  new URL("../desktop-static/assets/game.gk2pack", import.meta.url),
);
function syncAssets() {
  if (target === "desktop" && existsSync(source)) {
    mkdirSync(
      fileURLToPath(new URL("../desktop-static/assets/", import.meta.url)),
      { recursive: true },
    );
    cpSync(source, destination);
  }
}
syncAssets();
let watcher;
let timer;
if (target === "desktop" && action === "dev" && existsSync(source)) {
  watcher = watch(
    fileURLToPath(new URL("../static/assets/", import.meta.url)),
    () => {
      clearTimeout(timer);
      timer = setTimeout(syncAssets, 150);
    },
  );
}
const result = spawn(
  process.execPath,
  [
    fileURLToPath(new URL("../node_modules/vite/bin/vite.js", import.meta.url)),
    action,
    "--mode",
    target,
    ...args,
  ],
  { stdio: "inherit", env: { ...process.env, SAVE_BUILD_TARGET: target } },
);
result.on("error", (error) => {
  watcher?.close();
  throw error;
});
result.on("exit", (code) => {
  clearTimeout(timer);
  watcher?.close();
  process.exit(code ?? 1);
});
