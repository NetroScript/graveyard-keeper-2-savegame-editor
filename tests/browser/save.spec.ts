import { test, expect, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";

async function exported(page: Page) {
  const downloadReady = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export copy" }).click();
  const download = await downloadReady;
  const stream = await download.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream!) chunks.push(Buffer.from(chunk));
  return Buffer.concat(chunks);
}

test("worker load, scalar edit, byte-identical export and malformed input", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  const original = Buffer.from([4, 46, 44, 0, 5]);
  await page.getByLabel("Open save", { exact: true }).setInputFiles({ name: "synthetic.dat", mimeType: "application/octet-stream", buffer: original });
  await expect(page.getByText("Loaded. Export without edits preserves the original bytes.")).toBeVisible();
  expect(await exported(page)).toEqual(original);
  await page.getByRole("button", { name: "Open (1)", exact: true }).click();
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await page.getByLabel("bool (bool)").fill("true");
  await page.getByRole("button", { name: "Apply edit" }).click();
  await expect(page.getByText("Updated bool in memory.", { exact: false })).toBeVisible();
  expect(await exported(page)).toEqual(Buffer.from([4, 46, 44, 1, 5]));
  await page.getByLabel("Open save", { exact: true }).setInputFiles({ name: "broken.dat", mimeType: "application/octet-stream", buffer: Buffer.from([255]) });
  await expect(page.getByRole("alert")).toContainText("unsupported token");
  expect(await exported(page)).toEqual(Buffer.from([4, 46, 44, 1, 5]));
  expect(errors).toEqual([]);
});

test("local real save round trip through the browser", async ({ page }) => {
  const fixture = process.env.GK2_SAVE_FIXTURE;
  test.skip(!fixture, "Set GK2_SAVE_FIXTURE to a private .dat file to run this test");
  const original = await readFile(fixture!);
  await page.goto("/");
  await page.getByLabel("Open save", { exact: true }).setInputFiles(fixture!);
  await expect(page.getByText("Loaded. Export without edits preserves the original bytes.")).toBeVisible({ timeout: 30_000 });
  expect(await exported(page)).toEqual(original);
});
