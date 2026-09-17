// Cross-platform environment selection; avoids shell-specific VAR=value syntax.
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const [target, action, ...args] = process.argv.slice(2);
if (!["browser", "desktop"].includes(target) || !["dev", "build", "preview"].includes(action)) {
  throw new Error("Usage: frontend.mjs <browser|desktop> <dev|build|preview> [Vite options]");
}
const result = spawnSync(process.execPath, [
  fileURLToPath(new URL("../node_modules/vite/bin/vite.js", import.meta.url)), action, "--mode", target, ...args,
], { stdio: "inherit", env: { ...process.env, SAVE_BUILD_TARGET: target } });
if (result.error) throw result.error;
process.exit(result.status ?? 1);
