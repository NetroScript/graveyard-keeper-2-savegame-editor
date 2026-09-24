<script lang="ts">
  import { onMount, untrack, tick } from "svelte";
  import type { SaveDocument } from "../document.svelte";
  import {
    type ItemCatalog,
    type InventoryData,
    type InventoryItem,
    type InventoryRule,
    type Family,
    itemSearch,
    variantLabel,
  } from "./catalog";
  import ItemImage from "./ItemImage.svelte";
  let {
    doc,
    catalog,
    inventory,
    rule,
    existing,
    outOfBounds,
    onclose,
  }: {
    doc: SaveDocument;
    catalog: ItemCatalog;
    inventory: InventoryData;
    rule: InventoryRule;
    existing?: InventoryItem;
    outOfBounds: boolean;
    onclose: () => void;
  } = $props();
  const allowed = $derived.by(() => {
    if (inventory.kind !== "Tool belt") return rule.allowed;
    const occupiedTypes = new Set(
      inventory.items
        .filter((item) => item.node !== existing?.node)
        .map((item) => catalog.items[item.id]?.fields.type)
        .filter((type): type is string => !!type),
    );
    return rule.allowed.filter(
      (id) => !occupiedTypes.has(catalog.items[id]?.fields.type),
    );
  });
  const families = $derived(catalog.forInventory(allowed));
  const search = $derived(itemSearch(families));
  const initial = untrack(() => ({
    id: existing?.id ?? "",
    name: existing ? catalog.items[existing.id]?.name || existing.id : "",
    count: existing?.count ?? "1",
    durability: existing?.durability
      ? String(Math.round(Number(existing.durability) * 100))
      : "100",
    revision: doc.summary!.revision,
  }));
  let itemId = $state(initial.id);
  let query = $state(initial.name);
  let count = $state(initial.count);
  let durability = $state(initial.durability);
  let expanded = $state(false);
  let resultLimit = $state(50);
  let active = $state(0);
  let error = $state("");
  let saving = $state(false);
  let dialog: HTMLDialogElement;
  const revision = initial.revision;
  const matches = $derived(search(query === initial.name ? "" : query));
  const results = $derived(matches.slice(0, resultLimit));
  const selected = $derived(catalog.items[itemId]);
  const family = $derived(
    families.find((f) => f.variants.some((i) => i.id === itemId)),
  );
  const maximum = $derived(
    inventory.kind === "Tool belt"
      ? 1
      : outOfBounds
        ? 2147483647
        : (selected?.fields.stackCount ?? 1),
  );
  const valid = $derived(
    !!selected &&
      allowed.includes(itemId) &&
      count.trim() !== "" &&
      Number.isInteger(Number(count)) &&
      Number(count) > 0 &&
      Number(count) <= maximum &&
      (!selected.fields.hasDurability ||
        (Number.isFinite(Number(durability)) &&
          Number(durability) >= 0 &&
          Number(durability) <= 100)),
  );
  onMount(() => {
    dialog.showModal();
    return () => dialog.close();
  });
  function choose(f: Family) {
    itemId = f.variants[0].id;
    query = f.name;
    expanded = false;
    count = "1";
    durability = "100";
  }
  function changeVariant(id: string) {
    itemId = id;
    if (!outOfBounds && Number(count) > catalog.items[id].fields.stackCount)
      count = String(catalog.items[id].fields.stackCount);
  }
  async function confirm() {
    if (!valid || saving) return;
    if (doc.summary!.revision !== revision) {
      error = "The document changed. Close and reopen this dialog.";
      return;
    }
    saving = true;
    try {
      await doc.transact([
        {
          op: "inventory",
          container: inventory.node,
          out_of_bounds: outOfBounds,
          action: {
            kind: "put",
            node: existing?.node ?? null,
            item: itemId,
            count,
            guid: crypto.randomUUID(),
            durability: selected.fields.hasDurability
              ? String(Number(durability) / 100)
              : null,
          },
        },
      ]);
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  oncancel={(e) => {
    e.preventDefault();
    if (!saving) onclose();
  }}
  aria-labelledby="item-dialog-title"
>
  <h2 id="item-dialog-title" class="strip">
    {existing ? "Edit item" : "Add item"}
  </h2>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void confirm();
    }}
  >
    <div class="item-choice">
      <div class="selected-slot">
        {#if itemId}<ItemImage
            {catalog}
            id={itemId}
            {count}
            durability={selected?.fields.hasDurability
              ? String(Number(durability) / 100)
              : null}
          />{/if}
      </div>
      <div class="search-box">
        <label for="item-search">Item</label>
        <input
          id="item-search"
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={expanded}
          aria-controls="item-options"
          aria-activedescendant={expanded && results[active]
            ? `item-option-${active}`
            : undefined}
          value={query}
          autocomplete="off"
          onfocus={() => {
            expanded = true;
            active = 0;
          }}
          onclick={() => (expanded = true)}
          oninput={(e) => {
            query = e.currentTarget.value;
            resultLimit = 50;
            itemId = "";
            expanded = true;
            active = 0;
          }}
          onkeydown={(e) => {
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              expanded = true;
              if (e.key === "ArrowDown" && active + 1 >= resultLimit)
                resultLimit = Math.min(matches.length, resultLimit + 50);
              active = Math.max(
                0,
                Math.min(
                  Math.min(matches.length, resultLimit) - 1,
                  active + (e.key === "ArrowDown" ? 1 : -1),
                ),
              );
              void tick().then(() =>
                document
                  .getElementById(`item-option-${active}`)
                  ?.scrollIntoView({ block: "nearest" }),
              );
            }
            if (e.key === "Enter" && expanded) {
              e.preventDefault();
              if (results[active]) choose(results[active]);
            }
            if (e.key === "Escape" && expanded) {
              e.preventDefault();
              e.stopPropagation();
              expanded = false;
            }
          }}
        />
      </div>
    </div>
    {#if expanded}<div
        class="search-results"
        onscroll={(event) => {
          const list = event.currentTarget;
          if (list.scrollHeight - list.scrollTop - list.clientHeight < 192)
            resultLimit = Math.min(matches.length, resultLimit + 50);
        }}
        id="item-options"
        role="listbox"
        aria-label="Valid items"
      >
        {#each results as result, i}<button
            type="button"
            role="option"
            id={`item-option-${i}`}
            aria-selected={i === active}
            onpointermove={() => (active = i)}
            onclick={() => choose(result)}
          >
            <span class="result-icon"
              ><ItemImage {catalog} id={result.variants[0].id} /></span
            ><span>{result.name}<small>{result.id}</small></span>
          </button>{:else}<p>No matching items</p>{/each}
      </div>
    {/if}
    {#if family && family.variants.length > 1}
      {#if family.variants.some((v) => v.quality.overlaySprite) && !selected?.fields.isMainOrgan}
        <fieldset class="variants">
          <legend>Variant</legend>
          {#each family.variants as variant}<button
              type="button"
              aria-pressed={itemId === variant.id}
              onclick={() => changeVariant(variant.id)}
              ><span class="variant-image"
                ><ItemImage {catalog} id={variant.id} /></span
              ><span>{variantLabel(variant)}</span></button
            >{/each}
        </fieldset>
      {:else}<label class="dialog-field"
          >Variant<select
            value={itemId}
            onchange={(e) => changeVariant(e.currentTarget.value)}
            >{#each family.variants as variant}<option value={variant.id}
                >{variantLabel(variant)}</option
              >{/each}</select
          ></label
        >{/if}{/if}
    <label class="dialog-field"
      >Amount<input
        aria-label="Item amount"
        type="number"
        min="1"
        max={maximum}
        step="1"
        required
        disabled={!selected || (!outOfBounds && maximum === 1)}
        value={count}
        oninput={(e) => (count = e.currentTarget.value)}
      /></label
    >
    {#if selected}<p class="hint">
        Stack size: {selected.fields.stackCount}
      </p>{/if}
    {#if selected?.fields.hasDurability}<div class="durability-field">
        <label for="item-durability">Durability</label>
        <input
          id="item-durability"
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={durability}
        />
        <output for="item-durability">{durability}%</output>
        <button type="button" onclick={() => (durability = "100")}
          >Repair</button
        >
      </div>{/if}
    {#if error}<p role="alert">{error}</p>{/if}
    <div class="dialog-actions">
      <button type="button" disabled={saving} onclick={onclose}>Cancel</button
      ><button
        class="primary"
        type="submit"
        disabled={!valid || saving || doc.busy}>Confirm</button
      >
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(580px, calc(100vw - 32px));
    padding: 0;
    color: inherit;
    background: #272832;
    border: 1px solid #595b65;
    max-height: calc(100dvh - 32px);
    overflow: visible;
  }
  dialog::backdrop {
    background: #0009;
  }
  form {
    padding: 20px;
    max-height: calc(100dvh - 100px);
    overflow: auto;
  }
  .item-choice {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .selected-slot {
    width: 80px;
    height: 80px;
    flex: 0 0 80px;
    background: #1e2028;
    border: 1px solid #555966;
  }
  .search-box {
    flex: 1;
    min-width: 0;
  }
  .search-box input {
    width: 100%;
    margin-top: 6px;
  }
  .search-results {
    max-height: min(320px, 40dvh);
    overflow: auto;
    background: #1c1e25;
    border: 1px solid #555966;
    margin-top: 4px;
  }
  .search-results button {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    gap: 6px;
    padding: 0 8px 0 2px;
    min-height: 64px;
    line-height: 1.2;
    align-items: center;
    width: 100%;
    text-align: center;
    border: 0;
    background: transparent;
  }
  .search-results button[aria-selected="true"] {
    background: #46413a;
  }
  .search-results small {
    display: block;
    color: #a9a8ad;
  }
  .result-icon {
    width: 64px;
    height: 64px;
  }
  .dialog-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 18px;
  }
  .durability-field {
    display: grid;
    grid-template-columns: auto minmax(120px, 1fr) 44px auto;
    align-items: center;
    gap: 10px;
    margin-top: 18px;
  }
  .durability-field output {
    color: var(--cream);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  .variants {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 18px 0 0;
    border: 0;
    padding: 0;
  }
  .variants button {
    padding: 4px;
    display: grid;
    justify-items: center;
    background: #1e2028;
  }
  .variants button[aria-pressed="true"] {
    background: #514735;
    border-color: #aa8952;
    color: #f2ca77;
  }
  .variant-image {
    display: block;
    width: 80px;
    height: 80px;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 24px;
  }
</style>
