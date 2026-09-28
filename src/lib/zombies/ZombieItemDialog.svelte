<script lang="ts">
  import { onMount, untrack } from "svelte";
  import ItemImage from "../inventory/ItemImage.svelte";
  import SkullValue from "./SkullValue.svelte";
  import {
    itemSearch,
    plainText,
    type ItemCatalog,
    type ItemDefinition,
    type Family,
  } from "../inventory/catalog";

  type PickerMode = "organ" | "tool" | "armor" | "body" | "collar" | "cargo";

  let {
    catalog,
    title,
    current,
    filter,
    onconfirm,
    onclear,
    onclose,
    mode = "body",
    preferredTalent,
    talentNames = {},
    currentCount = "1",
    outOfBounds = false,
  }: {
    catalog: ItemCatalog;
    title: string;
    current?: string;
    filter: (item: ItemDefinition) => boolean;
    onconfirm: (id: string, count?: string) => Promise<void>;
    onclear?: () => Promise<void>;
    onclose: () => void;
    mode?: PickerMode;
    preferredTalent?: string;
    talentNames?: Record<string, string>;
    currentCount?: string;
    outOfBounds?: boolean;
  } = $props();

  let dialog: HTMLDialogElement;
  let query = $state("");
  let selected = $state(untrack(() => current ?? ""));
  let count = $state(untrack(() => currentCount));
  let limit = $state(60);
  let saving = $state(false);
  let error = $state("");
  const filteredFamilies = $derived(
    catalog.families
      .map((family) => ({
        ...family,
        variants: family.variants.filter(filter).sort(compareItems),
      }))
      .filter((family) => family.variants.length),
  );
  const search = $derived(itemSearch(filteredFamilies));
  const matches = $derived(search(query));
  const groups = $derived(
    groupFamilies(matches)
      .map((group) => ({
        ...group,
        families: group.families.slice(0, Math.max(0, limit - group.offset)),
      }))
      .filter((group) => group.families.length),
  );
  const chosen = $derived(catalog.items[selected]);
  const maximum = $derived(
    mode === "cargo"
      ? outOfBounds
        ? 2147483647
        : (chosen?.fields.stackCount ?? 1)
      : 1,
  );
  const validCount = $derived(
    mode !== "cargo" ||
      (count.trim() !== "" &&
        Number.isInteger(Number(count)) &&
        Number(count) > 0 &&
        Number(count) <= maximum),
  );

  $effect(() => {
    if (mode === "organ" && !selected && filteredFamilies[0])
      select(filteredFamilies[0].variants[0].id);
  });

  function select(id: string) {
    selected = id;
    if (mode === "cargo" && !outOfBounds && Number(count) > maximum)
      count = String(catalog.items[id]?.fields.stackCount ?? 1);
  }

  function itemLevel(item: ItemDefinition) {
    if (mode === "tool") return item.fields.talentBonus ?? 0;
    if (mode === "armor") return item.quality.value;
    return Math.max(
      item.quality.value,
      item.fields.redSkulls,
      item.fields.whiteSkulls,
    );
  }
  function compareItems(a: ItemDefinition, b: ItemDefinition) {
    return (
      itemLevel(b) - itemLevel(a) ||
      (b.fields.talentBonus ?? 0) - (a.fields.talentBonus ?? 0) ||
      b.fields.redSkulls - a.fields.redSkulls ||
      b.fields.whiteSkulls - a.fields.whiteSkulls ||
      catalog.displayName(a.id).localeCompare(catalog.displayName(b.id))
    );
  }
  function compareFamilies(a: Family, b: Family) {
    return (
      compareItems(a.variants[0], b.variants[0]) || a.name.localeCompare(b.name)
    );
  }
  function compareSkullFamilies(a: Family, b: Family) {
    const left = a.variants[0].fields;
    const right = b.variants[0].fields;
    const leftGain =
      Math.max(0, left.redSkulls) + Math.max(0, left.whiteSkulls);
    const rightGain =
      Math.max(0, right.redSkulls) + Math.max(0, right.whiteSkulls);
    return rightGain - leftGain || compareFamilies(a, b);
  }
  function typeName(type: string) {
    return (
      (
        {
          FishingRod: "Fishing rods",
          Reagents: "Alchemy kits",
          Axe: "Axes",
          Hammer: "Hammers",
          Pickaxe: "Pickaxes",
          Shovel: "Shovels",
          SmallTools: "Crafting tools",
          SurgicalKit: "Surgical kits",
          Book: "Books",
          Talisman: "Talismans",
          Hand: "Hands",
        } as Record<string, string>
      )[type] ?? type.replace(/([a-z])([A-Z])/g, "$1 $2")
    );
  }
  function groupFamilies(source: Family[]) {
    const result: {
      id: string;
      title: string;
      note?: string;
      families: Family[];
      offset: number;
    }[] = [];
    let offset = 0;
    const add = (
      id: string,
      title: string,
      families: Family[],
      note?: string,
      compare: (a: Family, b: Family) => number = compareFamilies,
    ) => {
      const sorted = [...families].sort(compare);
      if (!sorted.length) return;
      result.push({ id, title, note, families: sorted, offset });
      offset += sorted.length;
    };
    if (mode === "tool") {
      let remaining = source;
      if (preferredTalent) {
        const relevant = source.filter((family) =>
          family.variants[0].fields.talentIds?.includes(preferredTalent),
        );
        add(
          "recommended",
          `Best for ${talentNames[preferredTalent] ?? "current specialty"}`,
          relevant,
          "Higher ability levels improve work that uses this specialty.",
        );
        const relevantIds = new Set(relevant.map((family) => family.id));
        remaining = source.filter((family) => !relevantIds.has(family.id));
      }
      const types = [
        ...new Set(remaining.map((family) => family.variants[0].fields.type)),
      ];
      for (const type of types.sort((a, b) =>
        typeName(a).localeCompare(typeName(b)),
      ))
        add(
          type,
          typeName(type),
          remaining.filter((family) => family.variants[0].fields.type === type),
        );
    } else if (mode === "armor") {
      add(
        "normal",
        "Normal armor",
        source.filter((family) =>
          family.variants.some((item) => /^armor_\d+$/.test(item.id)),
        ),
        "Higher armor levels provide stronger combat protection.",
      );
      add(
        "debug",
        "Debug and NPC armor",
        source.filter(
          (family) =>
            !family.variants.some((item) => /^armor_\d+$/.test(item.id)),
        ),
        "Not normally obtainable. These entries may not behave correctly on a worker.",
      );
    } else if (mode === "body") {
      add(
        "embalming",
        "Body treatments",
        source.filter((family) => family.variants[0].fields.type === "Embalm"),
        "Treatments can add or remove skulls and occupy body-inventory slots.",
      );
      add(
        "skulls",
        "Skull-adding items",
        source.filter(
          (family) =>
            family.variants[0].fields.type !== "Embalm" &&
            (family.variants[0].fields.redSkulls > 0 ||
              family.variants[0].fields.whiteSkulls > 0),
        ),
        "These items increase the zombie's red or white skull total.",
        compareSkullFamilies,
      );
      add(
        "items",
        "Other items",
        source.filter(
          (family) =>
            family.variants[0].fields.type !== "Embalm" &&
            family.variants[0].fields.redSkulls <= 0 &&
            family.variants[0].fields.whiteSkulls <= 0,
        ),
      );
    } else if (mode === "cargo") {
      add(
        "cargo",
        "Cargo items",
        source,
        "Choose an item and the amount this porter should carry.",
      );
    } else add("choices", mode === "organ" ? "Variants" : "Choices", source);
    return result;
  }
  function abilityText(item: ItemDefinition) {
    const bonus = item.fields.talentBonus ?? 0;
    const talents = (item.fields.talentIds ?? [])
      .map((id) => talentNames[id] ?? id)
      .join(", ");
    return bonus > 0
      ? `Ability +${bonus}${talents ? ` · ${talents}` : ""}`
      : talents || "No ability bonus";
  }
  function itemMeta(item: ItemDefinition) {
    if (mode === "tool") return abilityText(item);
    if (mode === "armor") {
      if (!/^armor_\d+$/.test(item.id))
        return `Level ${item.quality.value} · not normally obtainable`;
      const protection =
        ["Minimal", "Basic", "Good", "Strong", "Strongest"][
          item.quality.value
        ] ?? "Unknown";
      return `Armor level ${item.quality.value} · ${protection} normal protection`;
    }
    return item.fields.type && item.fields.type !== "None"
      ? typeName(item.fields.type)
      : "Item";
  }

  onMount(() => {
    dialog.showModal();
    return () => dialog.close();
  });

  async function apply() {
    if (!chosen || saving) return;
    saving = true;
    error = "";
    try {
      await onconfirm(selected, mode === "cargo" ? count : undefined);
      onclose();
    } catch (reason) {
      error = String(reason);
    } finally {
      saving = false;
    }
  }
  async function clear() {
    if (!onclear || saving) return;
    saving = true;
    error = "";
    try {
      await onclear();
      onclose();
    } catch (reason) {
      error = String(reason);
    } finally {
      saving = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  oncancel={(event) => {
    event.preventDefault();
    if (!saving) onclose();
  }}
>
  <h2 class="dialog-title">{title}</h2>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void apply();
    }}
  >
    {#if mode !== "organ"}<label class="search"
        >Search<input
          value={query}
          autocomplete="off"
          oninput={(event) => {
            query = event.currentTarget.value;
            limit = 60;
          }}
        /></label
      >
      <div
        class="results"
        onscroll={(event) => {
          const list = event.currentTarget;
          if (list.scrollHeight - list.scrollTop - list.clientHeight < 192)
            limit = Math.min(matches.length, limit + 60);
        }}
      >
        {#each groups as group}
          <header class="group-heading">
            <strong>{group.title}</strong>
            {#if group.note}<small>{group.note}</small>{/if}
          </header>
          {#each group.families as family}
            {@const representative = family.variants[0]}
            <button
              type="button"
              class:selected={family.variants.some(
                (item) => item.id === selected,
              )}
              onclick={() => select(representative.id)}
            >
              <span class="preview"
                ><ItemImage {catalog} id={representative.id} /></span
              >
              <span class="choice-copy">
                <strong>{plainText(family.name)}</strong>
                <small>{itemMeta(representative)}</small>
                {#if representative.fields.redSkulls || representative.fields.whiteSkulls}<SkullValue
                    compact
                    red={representative.fields.redSkulls}
                    white={representative.fields.whiteSkulls}
                  />{/if}
              </span>
            </button>
          {/each}
        {/each}
      </div>{/if}
    {#if chosen}
      {@const family = filteredFamilies.find((entry) =>
        entry.variants.some((item) => item.id === selected),
      )}
      {#if family && family.variants.length > 1}
        <fieldset>
          <legend>Variant</legend>
          <div class="variants">
            {#each family.variants as item}<button
                type="button"
                class:selected={selected === item.id}
                onclick={() => select(item.id)}
                ><span class="variant-icon"
                  ><ItemImage {catalog} id={item.id} /></span
                ><span class="choice-copy">
                  <strong>{catalog.displayName(item.id)}</strong>
                  {#if item.fields.redSkulls || item.fields.whiteSkulls}<SkullValue
                      compact
                      red={item.fields.redSkulls}
                      white={item.fields.whiteSkulls}
                    />{/if}
                  <small
                    >{mode === "organ"
                      ? `Quality ${item.quality.value}`
                      : itemMeta(item)}</small
                  >
                </span></button
              >{/each}
          </div>
        </fieldset>
      {/if}
      <p class="selection">
        <span
          ><strong>{catalog.displayName(chosen.id)}</strong><small
            >{itemMeta(chosen)}</small
          ></span
        >
        {#if chosen.fields.redSkulls || chosen.fields.whiteSkulls}<SkullValue
            red={chosen.fields.redSkulls}
            white={chosen.fields.whiteSkulls}
          />{/if}
      </p>
      {#if mode === "cargo"}<label class="amount"
          >Amount<input
            aria-label="Cargo amount"
            type="number"
            min="1"
            max={maximum}
            step="1"
            required
            disabled={!chosen || (!outOfBounds && maximum === 1)}
            value={count}
            oninput={(event) => (count = event.currentTarget.value)}
          /><small>Stack size: {chosen.fields.stackCount}</small></label
        >{/if}
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <footer>
      {#if onclear}<button
          class="clear"
          type="button"
          onclick={() => void clear()}
          disabled={saving}>Clear slot</button
        >{/if}<span></span><button
        type="button"
        onclick={onclose}
        disabled={saving}>Cancel</button
      ><button class="primary" disabled={!chosen || !validCount || saving}
        >Apply</button
      >
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 32px));
    max-height: calc(100dvh - 32px);
    padding: 0;
    overflow: hidden;
    background: var(--surface);
    color: var(--cream);
    border: 1px solid var(--metal);
  }
  form {
    display: grid;
    gap: 12px;
    padding: 16px;
    max-height: calc(100dvh - 96px);
    overflow: auto;
  }
  .dialog-title {
    margin: 0;
    padding: 10px 16px;
    color: var(--cream);
    font-size: 16px;
    background: #694927;
    border-bottom: 1px solid #926b3f;
  }
  .search {
    display: grid;
    gap: 4px;
    color: #b9bec7;
    font-size: 12px;
  }
  .results {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px;
    min-width: 0;
    max-height: 330px;
    overflow: auto;
  }
  .results button {
    display: flex;
    width: 100%;
    min-width: 0;
    align-items: center;
    gap: 10px;
    min-height: 82px;
    padding: 4px 8px 4px 4px;
    text-align: left;
    background: #252831;
  }
  .results button.selected,
  .variants button.selected {
    color: #17181d;
    background: var(--gold);
    border-color: #f0d68b;
  }
  .preview {
    width: 72px;
    height: 72px;
    flex: 0 0 72px;
  }
  .results strong,
  .results small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .results button > span:last-child {
    min-width: 0;
  }
  .group-heading {
    grid-column: 1/-1;
    display: grid;
    gap: 2px;
    padding: 7px 9px;
    color: var(--cream);
    background: #20232b;
    border-left: 3px solid #9b7040;
  }
  .group-heading small {
    color: #aeb3bd;
    font-weight: 400;
  }
  .results small {
    color: inherit;
    opacity: 0.75;
    font-size: 11px;
  }
  fieldset {
    min-width: 0;
    max-width: 100%;
    margin: 0;
    padding: 8px;
    border: 1px solid var(--border);
  }
  legend {
    color: #b9bec7;
    font-size: 12px;
  }
  .variants {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
    max-height: 230px;
    overflow: auto;
  }
  .variants button {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    width: 100%;
    min-width: 0;
    min-height: 76px;
    align-items: center;
    gap: 4px;
    text-align: left;
  }
  .variant-icon {
    width: 64px;
    height: 64px;
  }
  .choice-copy {
    display: grid;
    align-content: center;
    gap: 3px;
    min-width: 0;
  }
  .choice-copy strong,
  .choice-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .choice-copy small {
    color: inherit;
    opacity: 0.78;
    font-size: 11px;
  }
  .choice-copy :global(.skulls) {
    justify-self: start;
  }
  .selection {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: 0;
    padding: 8px;
    background: var(--deep);
    border: 1px solid var(--border);
  }
  .selection > span:first-child {
    display: grid;
    gap: 2px;
  }
  .selection small {
    color: #aeb3bd;
  }
  .amount {
    display: grid;
    grid-template-columns: auto minmax(90px, 140px) 1fr;
    align-items: center;
    gap: 10px;
    color: #b9bec7;
  }
  .amount small {
    color: #aeb3bd;
  }
  .error {
    color: #f0a19c;
  }
  footer {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    gap: 8px;
  }
  .clear {
    color: #f0b1ac;
    border-color: #914947;
  }
  @media (max-width: 620px) {
    .results,
    .variants {
      grid-template-columns: 1fr;
    }
  }
</style>
