import { cp, mkdir, readFile, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { spawn } from "node:child_process";

const args = process.argv.slice(2);
const value = (name) => {
  const index = args.indexOf(name);
  return index < 0 ? undefined : args[index + 1];
};
if (args.includes("--help")) {
  console.log(
    "Usage: pnpm release:local [--asset game.gk2pack] [--web-only|--desktop-only] [--portable-only] [--output directory]",
  );
  process.exit(0);
}
if (args.includes("--web-only") && args.includes("--desktop-only"))
  throw new Error("Choose either --web-only or --desktop-only");
const portableOnly = args.includes("--portable-only");
if (portableOnly && process.platform !== "linux")
  throw new Error("--portable-only is supported only on Linux");
if (portableOnly && args.includes("--web-only"))
  throw new Error("--portable-only requires a desktop build");
const buildWeb = !args.includes("--desktop-only");
const buildDesktop = !args.includes("--web-only");

const root = resolve(import.meta.dirname, "..");
const packageInfo = JSON.parse(
  await readFile(resolve(root, "package.json"), "utf8"),
);
const output = resolve(
  value("--output") ?? resolve(root, "release", packageInfo.version),
);
const asset = value("--asset");
if (asset) {
  await mkdir(resolve(root, "static", "assets"), { recursive: true });
  await cp(resolve(asset), resolve(root, "static", "assets", "game.gk2pack"));
}
if (!existsSync(resolve(root, "static", "assets", "game.gk2pack")))
  throw new Error("No asset pack found. Pass --asset <game.gk2pack>.");

function run(command, commandArgs, env = {}) {
  return new Promise((resolvePromise, reject) => {
    const pnpmCli = command === "pnpm" ? process.env.npm_execpath : undefined;
    const scriptCli = pnpmCli && /\.(?:c|m)?js$/i.test(pnpmCli);
    const executable = scriptCli ? process.execPath : pnpmCli || command;
    const childArgs = scriptCli ? [pnpmCli, ...commandArgs] : commandArgs;
    const child = spawn(executable, childArgs, {
      cwd: root,
      stdio: "inherit",
      env: { ...process.env, ...env },
    });
    child.on("error", reject);
    child.on("exit", (code) =>
      code === 0
        ? resolvePromise()
        : reject(new Error(`${command} exited with code ${code}`)),
    );
  });
}

const platformName =
  process.platform === "win32"
    ? "windows"
    : process.platform === "darwin"
      ? "macos"
      : process.platform;
const desktopOutput = resolve(output, `${platformName}-${process.arch}`);
if (buildWeb && buildDesktop)
  await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
if (buildWeb) {
  await rm(resolve(output, "web"), { recursive: true, force: true });
  await run("pnpm", ["build:web"]);
  await cp(resolve(root, "build", "web"), resolve(output, "web"), {
    recursive: true,
  });
}
if (buildDesktop) {
  await rm(desktopOutput, { recursive: true, force: true });
  await mkdir(desktopOutput, { recursive: true });
  if (process.platform === "linux") {
    const portableTarget = resolve(root, "target", "portable");
    await run("pnpm", ["tauri", "build", "--no-bundle"], {
      CARGO_TARGET_DIR: portableTarget,
    });
    await run("tar", [
      "-C",
      resolve(portableTarget, "release"),
      "-czf",
      resolve(
        desktopOutput,
        `graveyard-keeper-2-save-editor-v${packageInfo.version}-linux-x86_64-system-webkit.tar.gz`,
      ),
      "graveyard-keeper-2-savegame-editor",
    ]);
  }
  if (!portableOnly) {
    const key = resolve(root, ".tauri-private-key");
    const bundles = resolve(root, "src-tauri", "target", "release", "bundle");
    await rm(bundles, { recursive: true, force: true });
    const bundleTargets =
      process.platform === "win32"
        ? "nsis"
        : process.platform === "darwin"
          ? "app,dmg"
          : "appimage,deb,rpm";
    await run(
      "pnpm",
      ["tauri", "build", "--bundles", bundleTargets],
      existsSync(key)
        ? {
            TAURI_SIGNING_PRIVATE_KEY: await readFile(key, "utf8"),
            TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "",
          }
        : {},
    );
    if (existsSync(bundles))
      await cp(bundles, desktopOutput, {
        recursive: true,
      });
  }
}
console.log(`Release output: ${output}`);
