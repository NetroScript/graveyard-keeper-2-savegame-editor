import { test, expect, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { encode } from "@msgpack/msgpack";
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
function textScalar(name: string, value: string) {
  return Buffer.concat([Buffer.from([39]), str(name), str(value)]);
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
function objectNode(
  name: string,
  typeId: number,
  typeName: string,
  objectId: number,
  children: Buffer[],
) {
  return Buffer.concat([
    Buffer.from([1]),
    str(name),
    Buffer.from([47]),
    i32(typeId),
    str(typeName),
    i32(objectId),
    ...children,
    Buffer.from([5]),
  ]);
}
function fixture() {
  return node("", [
    node("playerData", [
      node("inventory", [
        node("inventoryItem", [
          textScalar("id", "inventory"),
          scalar("count", 1),
          node("inventory", [Buffer.from([6, 0, 0, 0, 0, 0, 0, 0, 0, 7])]),
          scalar("inventorySize", 20),
          scalar("inventoryFillSize", 0),
          node("properties", [Buffer.from([6, 0, 0, 0, 0, 0, 0, 0, 0, 7])]),
        ]),
      ]),
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
    objectNode("wgo", 1, "WgoData, Assembly-CSharp", 7, [
      textScalar("id", "graveyard_gate"),
    ]),
    objectNode("scene", 2, "GameSceneData, Assembly-CSharp", 8, [
      textScalar("id", "Village"),
    ]),
    objectNode("item", 3, "Item, Assembly-CSharp", 9, [
      textScalar("id", "simple_iron_parts"),
      scalar("count", 12),
    ]),
    objectNode(
      "resource",
      4,
      "LazyBearTechnology.GameResAtom, LazyBearTechnology",
      10,
      [
        textScalar("type", "money"),
        Buffer.concat([
          Buffer.from([31]),
          str("value"),
          Buffer.from([0, 0, 72, 65]),
        ]),
      ],
    ),
    objectNode("uniqueId", 5, "SGuid, Assembly-CSharp", 11, [
      textScalar("id", "12345678-1234-1234-1234-123456789abc"),
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
  const save = page.getByRole("button", { name: "Save", exact: true });
  await (
    (await save.isEnabled())
      ? save
      : page.getByRole("button", { name: "Save As", exact: true })
  ).click();
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
  const maximum = page.getByRole("spinbutton", {
    name: "Maximum health",
    exact: true,
  });
  await maximum.fill("0");
  await expect(
    page.getByRole("button", { name: "Save", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Save As", exact: true }),
  ).toBeDisabled();
  await maximum.fill("100");
  await expect(
    page.getByRole("button", { name: "Save As", exact: true }),
  ).toBeEnabled();
  const sections = page.locator(".general-section");
  await expect(sections.locator("h3.strip")).toHaveText([
    "Vitals",
    "Money",
    "Technology Points",
    "Mental State",
    "Save utilities",
  ]);
  const content = await page.locator(".general-content").boundingBox();
  for (const section of await sections.all()) {
    const bounds = await section.boundingBox();
    expect(bounds!.width).toBeGreaterThan(content!.width - 65);
  }
  await expect(
    page.getByRole("button", { name: "Save", exact: true }),
  ).toBeDisabled();
  expect(await exported(page)).toEqual(fixture());
  await page
    .getByRole("spinbutton", { name: "Current health", exact: true })
    .fill("88");
  await page.getByRole("button", { name: "Load Saves", exact: true }).click();
  await open(page, "two.dat");
  await page
    .getByRole("button", { name: /^one\.dat(?: Unsaved changes)?$/ })
    .click();
  await expect(
    page.getByRole("spinbutton", { name: "Current health", exact: true }),
  ).toHaveValue("88");
  await expect(
    page.getByRole("button", { name: "Save", exact: true }),
  ).toBeEnabled();
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
  await page.getByRole("spinbutton", { name: "Gold coins" }).fill("1");
  await page.getByRole("spinbutton", { name: "Silver coins" }).fill("23");
  await page.getByRole("spinbutton", { name: "Bronze coins" }).fill("45");
  await expect(
    page.getByRole("button", { name: "Save", exact: true }),
  ).toBeEnabled();
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
  const vectorRow = page.locator(".tree-row").filter({
    has: page.locator(".tree-title > span", { hasText: "position" }),
  });
  await expect(vectorRow.locator(".tree-title em")).toHaveText("Vector3");
  await expect(vectorRow.locator(".tree-summary")).toContainText("x1");
  await expect(vectorRow.locator(".tree-summary")).toContainText("y2");
  await expect(vectorRow.locator(".tree-summary")).toContainText("z3");
  await expect(vectorRow.locator(".expander")).toBeDisabled();
  await expect(
    page
      .locator(".tree-row")
      .filter({ hasText: "wgoWgoDataobject #7idgraveyard_gate" }),
  ).toBeVisible();
  await expect(
    page
      .locator(".tree-row")
      .filter({ hasText: "sceneGameSceneDataidVillage" }),
  ).toBeVisible();
  const itemRow = page
    .locator(".tree-row")
    .filter({ hasText: "itemidsimple_iron_partscount12" });
  await expect(itemRow).toBeVisible();
  await expect(itemRow.locator(".tree-title em")).toHaveCount(0);
  const itemTitle = await itemRow.locator(".tree-title").boundingBox();
  const itemSummary = await itemRow.locator(".tree-summary").boundingBox();
  expect(itemTitle).not.toBeNull();
  expect(itemSummary).not.toBeNull();
  expect(itemSummary!.x - itemTitle!.x - itemTitle!.width).toBeLessThan(20);
  await expect(
    page
      .locator(".tree-row")
      .filter({ hasText: "resourceGameResAtomtypemoneyvalue12.5" }),
  ).toBeVisible();
  const guidRow = page.locator(".tree-row").filter({
    has: page.locator(".tree-title > span", { hasText: "uniqueId" }),
  });
  await expect(guidRow.locator(".tree-summary")).toHaveText(
    "12345678-1234-1234-1234-123456789abc",
  );
  await guidRow.locator(".tree-label").click();
  const guid = page.getByRole("textbox", { name: "GUID", exact: true });
  await expect(guid).toHaveValue("12345678-1234-1234-1234-123456789abc");
  await guid.fill("not-a-guid");
  await expect(page.getByText("Enter a GUID in the form")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Apply GUID", exact: true }),
  ).toBeDisabled();
  await guid.fill("abcdefab-cdef-abcd-efab-cdefabcdefab");
  await page.getByRole("button", { name: "Apply GUID", exact: true }).click();
  await expect(guidRow.locator(".tree-summary")).toHaveText(
    "abcdefab-cdef-abcd-efab-cdefabcdefab",
  );
  await vectorRow.locator(".tree-label").click();
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
    page.getByRole("spinbutton", { name: "Gold coins" }),
  ).toHaveValue("1");
  await expect(
    page.getByRole("spinbutton", { name: "Silver coins" }),
  ).toHaveValue("23");
  await expect(
    page.getByRole("spinbutton", { name: "Bronze coins" }),
  ).toHaveValue("45");
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
  await page.getByRole("button", { name: "Inventory", exact: true }).click();
  const player = page.getByRole("region", {
    name: "Player inventory",
    exact: true,
  });
  await expect(player).toBeVisible({ timeout: 30000 });
  await expect(player.locator(".sprite").first()).toBeVisible();
  const categoryNames = await page.locator(".category-heading h3").allTextContents();
  expect(categoryNames[0]).toBe("Player");
  expect(categoryNames.slice(1)).toEqual(
    [...categoryNames.slice(1)].sort((a, b) => a.localeCompare(b)),
  );
  await expect(
    page.getByRole("navigation", { name: "Inventory categories" }).getByRole("button"),
  ).toHaveCount(categoryNames.length);
  const inventoryOrder = await page
    .locator(".inventory-heading h3")
    .allTextContents();
  await page.screenshot({ path: "test-results/inventory-real.png" });
  await player.locator(".slot-content").first().click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.screenshot({ path: "test-results/inventory-dialog-real.png" });
  const amount = page.getByLabel("Item amount");
  if (await amount.isEnabled()) {
    const original = Number(await amount.inputValue());
    await amount.fill(String(original === 1 ? 2 : 1));
    await page.getByRole("button", { name: "Confirm", exact: true }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    expect(
      await page.locator(".inventory-heading h3").allTextContents(),
    ).toEqual(inventoryOrder);
    await expect(page.locator(".inventory").first()).toHaveAttribute(
      "aria-label",
      "Player inventory",
    );
  } else {
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
  }
  await player
    .getByRole("button", { name: "Add item to Player inventory", exact: true })
    .click();
  await page
    .getByRole("combobox", { name: "Item", exact: true })
    .fill("Burial Certificate");
  await page
    .getByRole("listbox", { name: "Valid items" })
    .getByRole("option")
    .first()
    .click();
  const variants = page.locator(".variants");
  await expect(variants.getByRole("button")).toHaveCount(3);
  await expect(variants.locator("button[aria-pressed=true]")).toHaveCount(1);
  await expect(variants.locator("img.quality").first()).toBeVisible();
  await expect(variants.locator("img.quality")).toHaveCount(3, {
    timeout: 20000,
  });
  await variants.getByRole("button").last().click();
  await expect(variants.getByRole("button").last()).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.screenshot({ path: "test-results/inventory-variants-real.png" });
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
});

test("real save renders progression trees and undo restores unlocks", async ({ page }) => {
  test.skip(!process.env.GK2_SAVE_FIXTURE, "Set GK2_SAVE_FIXTURE for private save validation");
  const bytes = await readFile(process.env.GK2_SAVE_FIXTURE!);
  await page.goto("/");
  await page.getByLabel("Select save files").setInputFiles(process.env.GK2_SAVE_FIXTURE!);
  await expect(page.getByRole("spinbutton", { name: "Current health", exact: true })).toBeEnabled({ timeout: 30000 });

  await page.getByRole("button", { name: "Technologies", exact: true }).click();
  await expect(page.getByRole("navigation", { name: "Technology trees" })).toBeVisible({ timeout: 30000 });
  await expect(page.locator(".tech-node").first()).toBeVisible();
  await expect(page.locator(".branch-tabs img").first()).toBeVisible();
  await expect(page.locator(".tech-node > .anchor > strong").filter({ hasText: /^Furniture Kit I$/ })).toHaveCount(1);
  await expect(page.getByText("tech_furniture_kit_1", { exact: true })).toHaveCount(0);
  await expect(page.getByText(/t_b_signboard_house|repair_sign_1102/)).toHaveCount(0);
  await expect.poll(async () => (await page.locator(".reward-icons img").first().boundingBox())?.width ?? 0).toBeGreaterThan(45);
  await page.locator(".tech-node strong").first().hover();
  const techPopover = page.locator('[popover]:popover-open');
  await expect(techPopover).toBeVisible();
  await page.locator(".reward-icons .progression-icon").first().hover();
  await expect(techPopover.locator("strong")).not.toBeEmpty();
  await page.screenshot({ path: "test-results/technologies-real.png", fullPage: true });
  const reputationGate = page.locator(".tech-node.gate").first();
  if (await reputationGate.count()) {
    await reputationGate.scrollIntoViewIfNeeded();
    await expect(reputationGate.locator("img")).toBeVisible();
    await expect(reputationGate.locator(".gate-value")).toBeVisible();
    await page.screenshot({ path: "test-results/technology-gate-real.png", fullPage: true });
  }
  const unlockedTechs = page.locator(".tech-node.unlocked");
  const unlockedTechCount = await unlockedTechs.count();
  const lockedTech = page.locator(".tech-node:not(.unlocked)").first();
  await lockedTech.click();
  await expect.poll(() => unlockedTechs.count()).toBeGreaterThan(unlockedTechCount);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(unlockedTechs).toHaveCount(unlockedTechCount);

  await page.getByRole("button", { name: "Inspirations", exact: true }).click();
  await expect(page.getByRole("navigation", { name: "Inspiration categories" })).toBeVisible({ timeout: 30000 });
  await expect(page.locator(".inspiration-card").first()).toBeVisible();
  await expect(page.locator(".perk-node").first()).toBeVisible();
  await expect(page.locator(".perk-node img").first()).toBeVisible();
  const inspirationCard = page.locator(".inspiration-card").first();
  const selectedLevel = inspirationCard.locator('[aria-pressed="true"]');
  const previousLevel = Number(await selectedLevel.textContent());
  const maximumLevel = await inspirationCard.locator(".level-buttons button").count() - 1;
  if (previousLevel < maximumLevel) {
    const progress = inspirationCard.getByRole("slider");
    await progress.fill(await progress.getAttribute("max") ?? "0");
    await expect(inspirationCard.locator('[aria-pressed="true"]')).toHaveText(String(previousLevel + 1));
    await page.getByRole("button", { name: "Undo", exact: true }).click();
    await expect(inspirationCard.locator('[aria-pressed="true"]')).toHaveText(String(previousLevel));
  }
  await page.screenshot({ path: "test-results/inspirations-real.png", fullPage: true });
  const unlockedPerks = page.locator(".perk-node.unlocked");
  const unlockedPerkCount = await unlockedPerks.count();
  const lockedPerk = page.locator(".perk-node:not(.unlocked)").first();
  await lockedPerk.click();
  await expect.poll(() => unlockedPerks.count()).toBeGreaterThan(unlockedPerkCount);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(unlockedPerks).toHaveCount(unlockedPerkCount);
  expect(await exported(page)).toEqual(bytes);
});

function inventoryPack(extraItems = 0) {
  const definition = (
    id: string,
    name: string,
    stack: number,
    quality = 0,
    bag = false,
  ) => ({
    id,
    name,
    description: "Food",
    sprite: null,
    quality: {
      type: quality ? "Star" : "None",
      value: quality,
      family: quality ? "apple" : null,
      overlaySprite: null,
    },
    fields: {
      stackCount: stack,
      itemSize: "Small",
      isBag: bag,
      bagSize: bag ? 4 : 0,
      inventorySize: 0,
      hasDurability: false,
      itemGroupIds: ["food"],
      type: "None",
      redSkulls: 0,
      whiteSkulls: 0,
    },
  });
  const metadata = encode({
    schemaVersion: 1,
    images: {},
    catalogs: {
      items: {
        ...Object.fromEntries(
          Array.from({ length: extraItems }, (_, i) => {
            const id = `test_${String(i).padStart(3, "0")}`;
            return [id, definition(id, `Test item ${i}`, 10)];
          }),
        ),
        "apple:1": definition("apple:1", "Apple", 10, 1),
        "apple:3": definition("apple:3", "Apple", 10, 3),
        tool: definition("tool", "Tool", 1),
        bag: definition("bag", "Food bag", 1, 0, true),
      },
      icons: { sprites: {}, fontIcons: {}, spriteAssets: [] },
      "inventory-rules": {
        bags: {
          bag: { complete: true, allowedItemIds: ["apple:1", "apple:3"] },
        },
      },
    },
  });
  const header = Buffer.alloc(16);
  header.set(Buffer.from("GK2PACK\0"));
  header.writeUInt32LE(1, 8);
  header.writeUInt32LE(metadata.length, 12);
  return Buffer.concat([header, metadata]);
}
test("item suggestions load more on scroll and retain compact rows", async ({
  page,
}) => {
  await page.route("**/assets/game.gk2pack", (route) =>
    route.fulfill({
      body: inventoryPack(120),
      contentType: "application/octet-stream",
    }),
  );
  await page.goto("/");
  await open(page);
  await page.getByRole("button", { name: "Inventory", exact: true }).click();
  await page
    .getByRole("button", { name: "Add item to Player inventory", exact: true })
    .click();
  const list = page.getByRole("listbox", { name: "Valid items" });
  await expect(list.getByRole("option")).toHaveCount(50);
  expect(
    (await list.getByRole("option").first().boundingBox())!.height,
  ).toBeLessThanOrEqual(66);
  await list.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(list.getByRole("option")).toHaveCount(100);
  await list.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect(list.getByRole("option")).toHaveCount(123);
  await page
    .getByRole("combobox", { name: "Item", exact: true })
    .fill("test_119");
  await expect(
    list.getByRole("option").filter({ hasText: "test_119" }),
  ).toBeVisible();
});

test("inventory dialog searches variants, enforces bounds, edits, deletes and undoes", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const send = Worker.prototype.postMessage;
    (window as any).saveRequests = [];
    Worker.prototype.postMessage = function (message: any, ...args: any[]) {
      if (message.op === "request")
        (window as any).saveRequests.push(message.payload.request.op);
      return Reflect.apply(send, this, [message, ...args]);
    };
  });
  await page.route("**/assets/game.gk2pack", (route) =>
    route.fulfill({
      body: inventoryPack(),
      contentType: "application/octet-stream",
    }),
  );
  await page.goto("/");
  await open(page);
  await page.getByRole("button", { name: "Inventory", exact: true }).click();
  const player = page.getByRole("region", {
    name: "Player inventory",
    exact: true,
  });
  await expect(player.locator(".inventory-slot")).toHaveCount(20);
  await expect(page.locator(".category-heading h3")).toHaveText(["Player"]);
  const categoryNavigation = page.getByRole("navigation", {
    name: "Inventory categories",
  });
  await expect(
    categoryNavigation.getByRole("button", {
      name: "Player: 1 container; shortcut 1",
    }),
  ).toBeVisible();
  const containerSearch = page.getByLabel("Search containers and items");
  await containerSearch.focus();
  await page.keyboard.press("1");
  await expect(containerSearch).toHaveValue("1");
  await containerSearch.fill("");
  await page.evaluate(() => (document.activeElement as HTMLElement)?.blur());
  await page.keyboard.press("1");
  await expect(page.locator(".inventory-category")).toBeFocused();
  const hideEmpty = page.getByLabel("Hide empty containers");
  await hideEmpty.check();
  await expect(player).toHaveCount(0);
  await expect(page.getByText("No containers match the current filters.")).toBeVisible();
  await hideEmpty.uncheck();
  await expect(player).toBeVisible();
  await player
    .getByRole("button", { name: "Add item to Player inventory", exact: true })
    .click();
  const dialog = page.getByRole("dialog");
  await dialog
    .getByRole("combobox", { name: "Item", exact: true })
    .fill("aple");
  await expect(dialog.getByRole("option")).toHaveCount(1);
  await dialog.getByRole("option").click();
  await expect(dialog.getByLabel("Variant")).toHaveValue("apple:3");
  await dialog.getByLabel("Item amount").fill("11");
  await expect(
    dialog.getByRole("button", { name: "Confirm", exact: true }),
  ).toBeDisabled();
  await dialog.getByLabel("Item amount").fill("4");
  await page.evaluate(() => {
    (window as any).saveRequests = [];
  });
  await dialog.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(dialog).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).saveRequests)).toEqual([
    "transact",
  ]);
  await player
    .getByRole("button", { name: "Edit Apple, amount 4", exact: true })
    .click();
  await expect(dialog.getByLabel("Item amount")).toHaveValue("4");
  await dialog.getByLabel("Item amount").fill("7");
  await dialog.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(
    player.getByRole("button", { name: "Edit Apple, amount 7", exact: true }),
  ).toBeVisible();
  await containerSearch.fill("apple");
  await expect(player).toBeVisible();
  await containerSearch.fill("tool");
  await expect(player).toHaveCount(0);
  await containerSearch.fill("player");
  await expect(player).toBeVisible();
  await containerSearch.fill("");
  await page.evaluate(() => {
    const saveAs = [...document.querySelectorAll("button")].find(
      (button) => button.textContent?.trim() === "Save As",
    )!;
    (window as any).disabledTransitions = [];
    new MutationObserver(() =>
      (window as any).disabledTransitions.push(saveAs.hasAttribute("disabled")),
    ).observe(saveAs, { attributes: true, attributeFilter: ["disabled"] });
  });
  await player
    .getByRole("button", { name: "Remove Apple", exact: true })
    .click();
  await expect(
    player.getByRole("button", { name: "Remove Apple", exact: true }),
  ).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).disabledTransitions)).not.toContain(
    true,
  );
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(
    player.getByRole("button", { name: "Edit Apple, amount 7", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Allow out of bounds edits").check();
  await page.getByRole("button", { name: /^one\.dat/ }).click();
  await player.getByLabel("Capacity: Player inventory").fill("24");
  await page.evaluate(() => {
    (window as any).saveRequests = [];
  });
  await player.getByLabel("Capacity: Player inventory").press("Tab");
  await expect(player.locator(".inventory-slot")).toHaveCount(24);
  expect(await page.evaluate(() => (window as any).saveRequests)).toEqual([
    "transact",
  ]);
  await player
    .getByRole("button", { name: "Edit Apple, amount 7", exact: true })
    .click();
  await dialog.getByLabel("Item amount").fill("50");
  await dialog.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(
    player.getByRole("button", { name: "Edit Apple, amount 50", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "test-results/inventory-wide.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator(".inventory-tools")).toBeVisible();
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
  ).toBe(true);
  await page.locator(".rail").evaluate((rail) =>
    Promise.all(rail.getAnimations().map((animation) => animation.finished)),
  );
  await page.screenshot({
    path: "test-results/inventory-narrow.png",
  });
  await page.getByRole("button", { name: "Toggle navigation" }).click();
  await expect(page.getByRole("navigation", { name: "Workspace" })).toBeVisible();
});
