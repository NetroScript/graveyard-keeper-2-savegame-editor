import { test, expect, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
function str(s: string) {
  return Buffer.concat([
    Buffer.from([0]),
    i32(s.length),
    Buffer.from(s, "latin1"),
  ]);
}
function i32(n: number) {
  const b = Buffer.alloc(4);
  b.writeInt32LE(n);
  return b;
}
function scalar(name: string, n: number) {
  return Buffer.concat([Buffer.from([23]), str(name), i32(n)]);
}
function node(name: string, children: Buffer[]) {
  return Buffer.concat([
    Buffer.from([name ? 3 : 4]),
    ...(name ? [str(name)] : []),
    Buffer.from([46]),
    ...children,
    Buffer.from([5]),
  ]);
}
function fixture() {
  return node("", [
    node("playerData", [
      node("hpComponent", [scalar("hp", 75), scalar("maxHpValue", 100)]),
      node("res", [
        node("resType", [Buffer.from([6, 0, 0, 0, 0, 0, 0, 0, 0, 7])]),
        node("resValues", [Buffer.from([6, 0, 0, 0, 0, 0, 0, 0, 0, 7])]),
      ]),
    ]),
    Buffer.concat([
      Buffer.from([3]),
      str("position"),
      Buffer.from([47]),
      i32(0),
      str("UnityEngine.Vector3, UnityEngine.CoreModule"),
      Buffer.from([32, 0, 0, 128, 63, 32, 0, 0, 0, 64, 32, 0, 0, 64, 64, 5]),
    ]),
  ]);
}
async function open(page: Page, name = "one.dat") {
  await page.getByLabel("Select save files").setInputFiles({
    name,
    mimeType: "application/octet-stream",
    buffer: fixture(),
  });
  await expect(
    page.getByRole("heading", { name: new RegExp(name) }),
  ).toBeVisible();
  await expect(
    page.getByRole("spinbutton", { name: "Current health", exact: true }),
  ).toHaveValue("75");
}
async function exported(page: Page) {
  const ready = page.waitForEvent("download");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  const stream = await (await ready).createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream!) chunks.push(Buffer.from(chunk));
  return Buffer.concat(chunks);
}
test("multiple saves preserve drafts, navigation, edits, undo and downloads", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await open(page);
  expect(await exported(page)).toEqual(fixture());
  await page
    .getByRole("spinbutton", { name: "Current health", exact: true })
    .fill("88");
  await page.getByRole("button", { name: "Load Saves", exact: true }).click();
  await open(page, "two.dat");
  await page.getByRole("button", { name: "one.dat", exact: true }).click();
  await expect(
    page.getByRole("spinbutton", { name: "Current health", exact: true }),
  ).toHaveValue("88");
  await page
    .locator("form")
    .filter({
      has: page.getByRole("spinbutton", {
        name: "Current health",
        exact: true,
      }),
    })
    .getByRole("button", { name: "Apply", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Undo", exact: true }),
  ).toBeEnabled();
  expect(await exported(page)).not.toEqual(fixture());
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(
    page.getByRole("spinbutton", { name: "Current health", exact: true }),
  ).toHaveValue("75");
  expect(await exported(page)).toEqual(fixture());
  await page
    .getByRole("button", { name: "Close one.dat", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "one.dat", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "two.dat", exact: true }).click();
  await expect(
    page.getByRole("spinbutton", { name: "Current health", exact: true }),
  ).toHaveValue("75");
  expect(errors).toEqual([]);
});
test("General insertion and Inspector share values; vector registration is active", async ({
  page,
}) => {
  await page.goto("/");
  await open(page);
  await page
    .getByRole("spinbutton", { name: "Money · base units", exact: true })
    .fill("12345");
  await page
    .locator("form")
    .filter({
      has: page.getByRole("spinbutton", {
        name: "Money · base units",
        exact: true,
      }),
    })
    .getByRole("button", { name: "Apply", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Undo", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Save Inspector", exact: true })
    .click();
  const treePane = page.locator(".tree-pane");
  const grabber = page.getByRole("button", {
    name: "Resize inspector panes",
  });
  const beforeResize = await treePane.boundingBox();
  const divider = await grabber.boundingBox();
  expect(beforeResize).not.toBeNull();
  expect(divider).not.toBeNull();
  await page.mouse.move(divider!.x + divider!.width / 2, divider!.y + 20);
  await page.mouse.down();
  await page.mouse.move(divider!.x + 80, divider!.y + 20);
  await page.mouse.up();
  const afterResize = await treePane.boundingBox();
  expect(afterResize!.width).toBeGreaterThan(beforeResize!.width + 50);
  await page
    .getByRole("button", { name: "Expand struct", exact: true })
    .click();
  await page
    .getByRole("button", { name: "position 3 fields", exact: true })
    .click();
  await expect(
    page.getByRole("spinbutton", { name: "Vector X", exact: true }),
  ).toHaveValue("1");
  await page
    .getByRole("spinbutton", { name: "Vector X", exact: true })
    .fill("5");
  await page.getByRole("button", { name: "Apply vector", exact: true }).click();
  await page.getByLabel("Show raw structure").check();
  await page.getByRole("button", { name: "f32 5", exact: true }).click();
  await expect(page.locator(".tree-row.selected .tree-label")).toContainText(
    "f32",
  );
  await expect(page.locator(".tree-row.selected")).toBeInViewport();
  await expect(
    page.getByRole("textbox", { name: "Record value", exact: true }),
  ).toHaveValue("5");
  await page.getByRole("button", { name: "Save Editor", exact: true }).click();
  await expect(
    page.getByRole("spinbutton", { name: "Money · base units", exact: true }),
  ).toHaveValue("12345");
});
test("drop files, copy guidance, malformed sidecar, narrow navigation and settings", async ({
  page,
  context,
}) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/");
  const loadPage = await page
    .locator(".page-content:not([hidden])")
    .boundingBox();
  const viewport = page.viewportSize();
  expect(loadPage).not.toBeNull();
  expect(viewport).not.toBeNull();
  expect(loadPage!.x + loadPage!.width).toBeCloseTo(viewport!.width, 0);
  await page.getByRole("button", { name: "Copy save location" }).click();
  await expect(
    page.getByRole("button", { name: "Copy save location" }),
  ).toHaveText("Copied");
  const bytes = Array.from(fixture());
  await page.locator(".drop-zone").evaluate((el, bytes) => {
    const transfer = new DataTransfer();
    transfer.items.add(new File([new Uint8Array(bytes)], "drop.dat"));
    transfer.items.add(new File(["broken"], "drop.info"));
    el.dispatchEvent(
      new DragEvent("drop", { bubbles: true, dataTransfer: transfer }),
    );
  }, bytes);
  await expect(page.getByRole("heading", { name: /drop.dat/ })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Toggle navigation" }).click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Interface scale").selectOption("1.15");
  await expect(page.getByRole("status")).toHaveText("Settings saved");
  await page.waitForTimeout(250);
  await page.screenshot({
    path: "test-results/workspace-narrow.png",
    fullPage: true,
  });
  await page.reload();
  await page.getByRole("button", { name: "Toggle navigation" }).click();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByLabel("Interface scale")).toHaveValue("1.15");
});
test("local real save round trip through browser", async ({ page }) => {
  test.skip(
    !process.env.GK2_SAVE_FIXTURE,
    "Set GK2_SAVE_FIXTURE for private save validation",
  );
  const bytes = await readFile(process.env.GK2_SAVE_FIXTURE!);
  await page.goto("/");
  await page
    .getByLabel("Select save files")
    .setInputFiles(process.env.GK2_SAVE_FIXTURE!);
  await expect(
    page.getByRole("spinbutton", { name: "Current health", exact: true }),
  ).toBeEnabled({ timeout: 30000 });
  expect(await exported(page)).toEqual(bytes);
  await page.screenshot({
    path: "test-results/workspace-wide.png",
    fullPage: true,
  });
});
