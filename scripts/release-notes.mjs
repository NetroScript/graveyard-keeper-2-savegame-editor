import { appendFile, readFile } from "node:fs/promises";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const args = process.argv.slice(2);

function argument(name) {
  const index = args.indexOf(name);
  return index < 0 ? undefined : args[index + 1];
}

const packageInfo = JSON.parse(
  await readFile(resolve(root, "package.json"), "utf8"),
);
const version = String(argument("--version") ?? packageInfo.version);
const tag = argument("--tag");
if (tag && tag !== `v${version}`)
  throw new Error(
    `Release tag ${tag} does not match application version v${version}`,
  );

const changelog = await readFile(resolve(root, "CHANGELOG.md"), "utf8");
const escapedVersion = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const heading = new RegExp(`^## \\[?${escapedVersion}\\]?(?:\\s+-.*)?$`, "m");
const match = heading.exec(changelog);
if (!match)
  throw new Error(`CHANGELOG.md has no section for version ${version}`);

const contentStart = match.index + match[0].length;
const nextHeading = changelog.slice(contentStart).search(/^## /m);
const notes = changelog
  .slice(
    contentStart,
    nextHeading < 0 ? changelog.length : contentStart + nextHeading,
  )
  .trim();
if (!notes) throw new Error(`CHANGELOG.md section ${version} is empty`);

const repository = argument("--repository") ?? process.env.GITHUB_REPOSITORY;
let body = notes;
if (repository) {
  if (!/^[^/]+\/[^/]+$/.test(repository))
    throw new Error(`Invalid GitHub repository: ${repository}`);
  const [owner, name] = repository.split("/");
  const release = `https://github.com/${repository}/releases/download/v${version}`;
  const file = `graveyard-keeper-2-save-editor-v${version}`;
  const downloads = `### Downloads

- **Windows x64:** [Installer](${release}/${file}-windows-x86_64-installer.exe) — recommended for automatic updates · [Portable executable](${release}/${file}-windows-x86_64-portable.exe) — replace it manually to remain portable
- **Linux x64:** [AppImage](${release}/${file}-linux-x86_64.AppImage)
- **macOS Apple Silicon:** [DMG](${release}/${file}-macos-aarch64.dmg) · [Portable application ZIP](${release}/${file}-macos-aarch64-portable.zip)
- **macOS Intel:** [DMG](${release}/${file}-macos-x86_64.dmg) · [Portable application ZIP](${release}/${file}-macos-x86_64-portable.zip)
- **Web:** [Open the web editor](https://${owner.toLowerCase()}.github.io/${name}/)

The Windows portable executable checks for updates, but accepting one launches the installer. Download a newer portable executable manually if you do not want to install the application.`;
  body = `${downloads}\n\n---\n\n${notes}`;
}

const githubOutput = argument("--github-output");
if (githubOutput) {
  let delimiter = "GK2_RELEASE_NOTES_EOF";
  while (body.includes(delimiter)) delimiter += "_1";
  await appendFile(
    githubOutput,
    `body<<${delimiter}\n${body}\n${delimiter}\n`,
    "utf8",
  );
} else {
  process.stdout.write(`${body}\n`);
}
