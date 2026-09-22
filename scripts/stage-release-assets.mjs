import { spawn } from "node:child_process";
import { resolve } from "node:path";
import { existsSync } from "node:fs";

const tag = process.argv[2] ?? "asset-pack-input";
const asset = resolve(process.argv[3] ?? "static/assets/game.gk2pack");
if (!existsSync(asset)) throw new Error(`Asset pack not found: ${asset}`);

function gh(args, allowFailure = false) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(process.platform === "win32" ? "gh.exe" : "gh", args, {
      stdio: "inherit",
    });
    child.on("error", reject);
    child.on("exit", (code) =>
      code === 0 || allowFailure
        ? resolvePromise(code)
        : reject(new Error(`gh exited with code ${code}`)),
    );
  });
}

const exists = (await gh(["release", "view", tag], true)) === 0;
if (!exists)
  await gh([
    "release",
    "create",
    tag,
    "--draft",
    "--title",
    "Private asset-pack input",
    "--notes",
    "Private input for the release workflow. Do not publish this draft.",
  ]);
await gh(["release", "upload", tag, `${asset}#game.gk2pack`, "--clobber"]);
console.log(`Staged game.gk2pack on private draft release ${tag}.`);
