<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Plus from "~icons/ph/plus";
  import MagnifyingGlass from "~icons/ph/magnifying-glass";
  import X from "~icons/ph/x";
  import { gameAssets } from "../assets/game-assets";
  import { type SaveDocument, type Settings } from "../document.svelte";
  import {
    ItemCatalog,
    type InventoryData,
    type InventoryItem,
    type InventoryRule,
    type InventorySnapshot,
    plainText,
  } from "./catalog";
  import ItemImage from "./ItemImage.svelte";
  import ItemDialog from "./ItemDialog.svelte";
  let {
    doc,
    settings,
    active = true,
  }: { doc: SaveDocument; settings: Settings; active?: boolean } = $props();
  let catalog = $state<ItemCatalog>();
  let inventories = $state<InventoryData[]>([]);
  let rules = $state<Record<string, InventoryRule>>({});
  let loadedEpoch = -1;
  let error = $state("");
  let loading = $state(true);
  let editing = $state<{
    inventory: InventoryData;
    existing?: InventoryItem;
  }>();
  let pages = $state<Record<number, number>>({});
  let background = $state<string>();
  let search = $state("");
  let hideEmpty = $state(false);

  function title(inventory: InventoryData) {
    return plainText(catalog?.items[inventory.title]?.name || inventory.title);
  }
  function category(inventory: InventoryData) {
    if (inventory.kind === "Player" || inventory.location?.player) return "Player";
    return inventory.location?.scene || inventory.location?.world || "Other";
  }
  function location(inventory: InventoryData) {
    if (!inventory.location) return "";
    const names = [inventory.location.scene, inventory.location.world].filter(
      (value, index, values): value is string =>
        !!value && values.indexOf(value) === index,
    );
    const position = inventory.location.position;
    if (position?.length)
      names.push(
        position
          .map((value) =>
            Number(value).toLocaleString(undefined, { maximumFractionDigits: 2 }),
          )
          .join(", "),
      );
    return names.join(" · ");
  }
  function matches(inventory: InventoryData, value: string) {
    const terms = value.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
    if (!terms.length) return true;
    const itemText = inventory.items.flatMap((item) => {
      const definition = catalog?.items[item.id];
      return [item.id, plainText(definition?.name || ""), definition?.fields.type];
    });
    const text = [
      inventory.kind,
      inventory.title,
      title(inventory),
      category(inventory),
      inventory.location?.scene,
      inventory.location?.world,
      ...itemText,
    ]
      .filter(Boolean)
      .join(" ")
      .toLocaleLowerCase();
    return terms.every((term) => text.includes(term));
  }
  const groups = $derived.by(() => {
    const grouped = new Map<string, InventoryData[]>();
    for (const inventory of inventories) {
      if ((hideEmpty && inventory.items.length === 0) || !matches(inventory, search))
        continue;
      const label = category(inventory);
      const entries = grouped.get(label) ?? [];
      entries.push(inventory);
      grouped.set(label, entries);
    }
    return [...grouped]
      .map(([label, entries]) => ({
        label,
        id: `inventory-category-${entries[0].node}`,
        inventories: entries.sort(
          (a, b) =>
            Number(b.kind === "Player") - Number(a.kind === "Player") ||
            a.kind.localeCompare(b.kind) ||
            title(a).localeCompare(title(b)) ||
            a.node - b.node,
        ),
      }))
      .sort(
        (a, b) =>
          Number(b.label === "Player") - Number(a.label === "Player") ||
          a.label.localeCompare(b.label),
      );
  });
  function jumpToCategory(id: string) {
    const target = document.getElementById(id);
    target?.scrollIntoView({ block: "start" });
    target?.focus({ preventScroll: true });
  }
  function categoryShortcut(index: number) {
    if (index < 9) return String(index + 1);
    if (index === 9) return "0";
    return "";
  }
  function categoryKeydown(event: KeyboardEvent) {
    if (
      !active ||
      editing ||
      event.ctrlKey ||
      event.altKey ||
      event.metaKey ||
      event.shiftKey ||
      event.repeat
    )
      return;
    const index = event.key === "0" ? 9 : Number(event.key) - 1;
    if (!Number.isInteger(index) || index < 0 || index >= Math.min(groups.length, 10))
      return;
    const focused = document.activeElement;
    if (
      focused instanceof HTMLInputElement ||
      focused instanceof HTMLTextAreaElement ||
      focused instanceof HTMLSelectElement ||
      (focused instanceof HTMLElement && focused.isContentEditable)
    )
      return;
    event.preventDefault();
    jumpToCategory(groups[index].id);
  }
  onMount(() => {
    let active = true;
    (async () => {
      try {
        const pack = await gameAssets();
        const loaded = new ItemCatalog(pack);
        await doc.query({ op: "inventory_catalog", catalog: loaded.rules });
        if (active) {
          catalog = loaded;
        }
        const ui = pack.pack.metadata.catalogs["inventory-ui"] as
          { backgroundSprite?: string } | undefined;
        if (ui?.backgroundSprite) {
          const url = await loaded.sprite(ui.backgroundSprite);
          if (active) background = url;
        }
      } catch (e) {
        if (active) {
          error = String(e);
          loading = false;
        }
      }
    })();
    return () => {
      active = false;
    };
  });
  $effect(() => {
    const epoch = doc.inventoryEpoch;
    if (!catalog || !active || loadedEpoch === epoch) return;
    let alive = true;
    doc
      .query<InventorySnapshot>({ op: "inventories" })
      .then((data) => {
        if (alive) {
          inventories = data.inventories;
          rules = data.rules;
          loadedEpoch = epoch;
          loading = false;
        }
      })
      .catch((e) => {
        if (alive) {
          error = String(e);
          loading = false;
        }
      });
    return () => {
      alive = false;
    };
  });
  $effect(() => {
    const delta = doc.inventoryDelta;
    if (!delta) return;
    untrack(() => {
      const updated = new Map(inventories.map((i) => [i.node, i]));
      for (const id of delta.removed) updated.delete(id);
      for (const inventory of delta.upsert)
        updated.set(inventory.node, inventory);
      // Preserve backend order here; the derived grouped view applies stable location sorting.
      const entries = [...updated.values()];
      inventories = [
        ...entries.filter((i) => i.kind === "Player"),
        ...entries.filter((i) => i.kind !== "Player"),
      ];
      rules = { ...rules, ...delta.rules };
    });
  });
  async function edit(
    inventory: InventoryData,
    action: Record<string, unknown>,
  ) {
    error = "";
    try {
      await doc.transact([
        {
          op: "inventory",
          container: inventory.node,
          out_of_bounds: settings.outOfBoundsEdits,
          action,
        },
      ]);
    } catch (e) {
      error = String(e);
    }
  }
  const pageSize = 200;
</script>

<svelte:window onkeydown={categoryKeydown} />

<div class="section-intro">
  <div>
    <h2>Inventory</h2>
    <p>Edit items in player, bag and world inventories.</p>
  </div>
</div>
{#if error}<p role="alert" class="error-banner">{error}</p>{/if}
{#if loading}<p role="status">Loading inventories…</p>{:else if catalog}
  <div class="inventory-tools panel">
    <label class="inventory-search">
      <span>Search containers and items</span>
      <span class="search-field"><MagnifyingGlass /><input
          type="search"
          placeholder="Place, container or item"
          bind:value={search}
        /></span>
    </label>
    <label class="empty-toggle"><input type="checkbox" bind:checked={hideEmpty} />Hide
      empty containers</label
    >
    <nav class="category-jump" aria-label="Inventory categories">
      <span>Jump to category</span>
      <div class="category-links">
        {#each groups as group, index (group.label)}
          {@const shortcut = categoryShortcut(index)}
          <button
            type="button"
            aria-label={`${group.label}: ${group.inventories.length} ${group.inventories.length === 1 ? "container" : "containers"}${shortcut ? `; shortcut ${shortcut}` : ""}`}
            onclick={() => jumpToCategory(group.id)}
          >{#if shortcut}<kbd>{shortcut}</kbd>{/if}<span>{group.label}</span><small
              >{group.inventories.length}</small
            ></button
          >
        {/each}
      </div>
    </nav>
  </div>
  {#if groups.length}<div class="category-groups">
    {#each groups as group (group.label)}
      <section
        class="inventory-category"
        id={group.id}
        aria-labelledby={`${group.id}-title`}
        tabindex="-1"
      >
        <header class="category-heading">
          <h3 id={`${group.id}-title`}>{group.label}</h3>
          <span>{group.inventories.length} {group.inventories.length === 1 ? "container" : "containers"}</span>
        </header>
        <div class="inventories">
    {#each group.inventories as inventory (inventory.node)}
      {@const rule = rules[inventory.ruleId]}
      {@const size = Math.max(
        Number(inventory.capacity),
        inventory.items.length,
        0,
      )}
      {@const page = Math.min(
        pages[inventory.node] ?? 0,
        Math.max(0, Math.ceil(size / pageSize) - 1),
      )}
      {@const locationText = location(inventory)}
      <section
        class="panel inventory"
        aria-label={inventory.kind === "Player"
          ? "Player inventory"
          : `${inventory.kind}: ${inventory.title}`}
      >
        <header class="strip inventory-heading">
          <h3>
            {inventory.kind}{#if inventory.kind !== "Player"}<small
                >{plainText(
                  catalog.items[inventory.title]?.name || inventory.title,
                )}</small
              >{/if}
          </h3>
          <label
            >Capacity {#if settings.outOfBoundsEdits}<input
                aria-label={`Capacity: ${inventory.title}`}
                type="number"
                min={inventory.items.length}
                max="2147483647"
                step="1"
                value={inventory.capacity}
                disabled={doc.busy}
                onchange={(e) => {
                  if (e.currentTarget.validity.valid)
                    void edit(inventory, {
                      kind: "capacity",
                      value: e.currentTarget.value,
                    });
                }}
              />{:else}<span>{inventory.capacity}</span>{/if}</label
          >
        </header>
        {#if locationText}<p class="hint location">{locationText}</p>{/if}
        {#if rule?.error}<p class="hint warning">{rule.error}</p>{/if}
        <div class="inventory-grid">
          {#each Array.from({ length: Math.min(pageSize, size - page * pageSize) }, (_, i) => i + page * pageSize) as index}
            {@const item = inventory.items[index]}
            <div
              class="inventory-slot"
              style:background-image={background
                ? `url("${background}")`
                : undefined}
            >
              {#if item}
                <button
                  class="slot-content"
                  aria-label={`Edit ${plainText(catalog.items[item.id]?.name || item.id)}, amount ${item.count}`}
                  title={plainText(catalog.items[item.id]?.name || item.id)}
                  disabled={doc.busy}
                  onclick={() => (editing = { inventory, existing: item })}
                  ><ItemImage
                    {catalog}
                    id={item.id}
                    count={item.count}
                    durability={item.durability}
                  /></button
                >
                <button
                  class="remove-item"
                  aria-label={`Remove ${plainText(catalog.items[item.id]?.name || item.id)}`}
                  disabled={doc.busy}
                  onclick={() =>
                    edit(inventory, { kind: "remove", node: item.node })}
                  ><X /></button
                >
              {:else if index === inventory.items.length}<button
                  class="slot-content add-item"
                  aria-label={`Add item to ${inventory.title}`}
                  disabled={doc.busy || !!rule?.error || !rule?.allowed.length}
                  onclick={() => (editing = { inventory })}><Plus /></button
                >{/if}
            </div>
          {/each}
        </div>
        {#if size > pageSize}<div class="slot-pages">
            <button
              disabled={page === 0}
              onclick={() => (pages[inventory.node] = page - 1)}
              >Previous slots</button
            ><span
              >{page * pageSize + 1}–{Math.min(size, (page + 1) * pageSize)} of {size}</span
            ><button
              disabled={(page + 1) * pageSize >= size}
              onclick={() => (pages[inventory.node] = page + 1)}
              >Next slots</button
            >
          </div>{/if}
      </section>
    {/each}
        </div>
      </section>
    {/each}
  </div>{:else}<p class="inventory-empty">
      {inventories.length
        ? "No containers match the current filters."
        : "No supported inventories were found."}
    </p>{/if}
  {#if editing}<ItemDialog
      {doc}
      {catalog}
      inventory={editing.inventory}
      rule={rules[editing.inventory.ruleId] ?? {
        allowed: [],
        error: "Inventory rules unavailable",
      }}
      existing={editing.existing}
      outOfBounds={settings.outOfBoundsEdits}
      onclose={() => (editing = undefined)}
    />{/if}
{/if}

<style>
  .inventory-tools {
    display: grid;
    grid-template-columns: minmax(240px, 1fr) auto;
    align-items: end;
    gap: 16px;
    padding: 16px;
    margin-bottom: 28px;
  }
  .inventory-tools label {
    font-size: 12px;
    color: var(--muted);
  }
  .inventory-search {
    display: grid;
    gap: 6px;
  }
  .search-field {
    position: relative;
    display: block;
  }
  .search-field :global(svg) {
    position: absolute;
    left: 10px;
    top: 50%;
    width: 18px;
    height: 18px;
    transform: translateY(-50%);
    color: #999ba2;
    pointer-events: none;
  }
  .search-field input {
    padding-left: 36px;
  }
  .empty-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    color: var(--cream) !important;
    white-space: nowrap;
  }
  .category-jump {
    grid-column: 1 / -1;
    display: grid;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
    color: var(--muted);
  }
  .category-links {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .category-links button {
    min-height: 32px;
    padding: 5px 9px;
    background: var(--deep);
    border-color: #51545e;
    border-radius: 2px;
  }
  .category-links kbd {
    min-width: 18px;
    padding: 1px 4px;
    color: #edc15b;
    background: #30343e;
    border: 1px solid #5c5e66;
    font: 500 11px/1.4 Roboto, Arial, sans-serif;
    text-align: center;
  }
  .category-links small {
    min-width: 18px;
    color: var(--muted);
    text-align: center;
  }
  .category-groups {
    display: grid;
    gap: 32px;
  }
  .inventory-category {
    scroll-margin-top: 16px;
  }
  .inventory-category:focus {
    outline: none;
  }
  .category-heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 12px;
    padding: 0 2px 8px;
    border-bottom: 1px solid #6b604f;
  }
  .category-heading h3 {
    color: #e3bd78;
    font-size: 20px;
  }
  .category-heading span {
    color: var(--muted);
    font-size: 12px;
  }
  .inventories {
    display: grid;
    gap: 20px;
  }
  .inventory-empty {
    padding: 18px;
    border: 1px solid var(--line);
    background: var(--deep);
  }
  .inventory-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  h3 small {
    font-size: 13px;
    font-weight: 400;
  }
  .inventory-heading label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .inventory-heading input {
    width: 90px;
    padding: 4px 8px;
  }
  .inventory-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, 80px);
    gap: 5px;
    padding: 16px;
  }
  .inventory-slot {
    position: relative;
    width: 80px;
    height: 80px;
    background: #242733;
    border: 1px solid #515765;
    background-size: 100% 100%;
    image-rendering: pixelated;
  }
  .slot-content {
    padding: 0;
    width: 100%;
    height: 100%;
    border: 0;
    background: transparent;
    display: block;
  }
  .slot-content:hover {
    background: #ffffff0b;
  }
  .add-item {
    display: grid;
    place-items: center;
    color: #b5a070;
  }
  .add-item :global(svg) {
    width: 26px;
    height: 26px;
  }
  .remove-item {
    position: absolute;
    left: 0;
    top: 0;
    padding: 1px;
    min-height: 0;
    line-height: 0;
    border: 0;
    background: #242733c9;
    color: #c0b5a9;
  }
  .remove-item:hover {
    background: #84312d;
    color: #fff;
  }
  .slot-pages {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 12px;
    padding: 12px;
  }
  .inventory > .hint {
    padding: 0 16px;
  }
  @media (max-width: 800px) {
    .inventory-tools {
      grid-template-columns: 1fr;
      align-items: stretch;
    }
  }
</style>
