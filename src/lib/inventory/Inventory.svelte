<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Plus from "~icons/ph/plus";
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
      // Updating a Map entry preserves its position. Newly discovered bags append;
      // editing an item must never reorder the inventories already on screen.
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

<div class="section-intro">
  <div>
    <h2>Inventory</h2>
    <p>Edit items in player, bag and world inventories.</p>
  </div>
</div>
{#if error}<p role="alert" class="error-banner">{error}</p>{/if}
{#if loading}<p role="status">Loading inventories…</p>{:else if catalog}
  <div class="inventories">
    {#each inventories as inventory (inventory.node)}
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
        {#if inventory.location && Object.values(inventory.location).some(Boolean)}<p
            class="hint location"
          >
            {[
              ...new Set(
                [inventory.location.scene, inventory.location.world].filter(
                  Boolean,
                ),
              ),
            ].join(" · ")}{#if inventory.location.position?.length}
              · {inventory.location.position
                .map((v) =>
                  Number(v).toLocaleString(undefined, {
                    maximumFractionDigits: 2,
                  }),
                )
                .join(", ")}{/if}
          </p>{/if}
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
    {:else}<p>No supported inventories were found.</p>{/each}
  </div>
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
  .inventories {
    display: grid;
    gap: 20px;
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
</style>
