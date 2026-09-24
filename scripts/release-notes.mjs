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
const heading = new RegExp(
  `^## \\[?${escapedVersion}\\]?(?:\\s+-.*)?$`,
  "m",
);
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

const githubOutput = argument("--github-output");
if (githubOutput) {
  let delimiter = "GK2_RELEASE_NOTES_EOF";
  while (notes.includes(delimiter)) delimiter += "_1";
  await appendFile(
    githubOutput,
    `body<<${delimiter}\n${notes}\n${delimiter}\n`,
    "utf8",
  );
} else {
  process.stdout.write(`${notes}\n`);
}
