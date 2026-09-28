<script lang="ts">
  import Plus from "~icons/ph/plus";
  import X from "~icons/ph/x";
  import type { ItemCatalog } from "../inventory/catalog";
  import ItemImage from "../inventory/ItemImage.svelte";
  import type { ZombieBodyItem } from "./catalog";
  import SkullValue from "./SkullValue.svelte";

  let {
    catalog,
    item,
    label,
    background,
    disabled = false,
    onclick,
    onremove,
  }: {
    catalog: ItemCatalog;
    item?: ZombieBodyItem;
    label: string;
    background?: string;
    disabled?: boolean;
    onclick: () => void;
    onremove?: () => void;
  } = $props();
  const definition = $derived(item ? catalog.items[item.id] : undefined);
  const name = $derived(item ? catalog.displayName(item.id) : label);
</script>

<div class="slot-wrap">
  <div
    class="slot"
    style:background-image={background ? `url("${background}")` : undefined}
  >
    <button
      type="button"
      class="slot-content"
      aria-label={item ? `Change ${label}: ${name}` : `Add ${label}`}
      title={item ? `${label}: ${name}` : label}
      {disabled}
      {onclick}
    >
      {#if item}<ItemImage
          {catalog}
          id={item.id}
          count={item.count}
        />{:else}<Plus />{/if}
    </button>
    {#if item && onremove}<button
        type="button"
        class="remove"
        aria-label={`Clear ${label}`}
        title={`Clear ${label}`}
        {disabled}
        onclick={onremove}><X /></button
      >{/if}
  </div>
  <strong>{label}</strong>
  {#if item}<span class="item-name">{name}</span>{/if}
  {#if definition && (definition.fields.redSkulls || definition.fields.whiteSkulls)}<SkullValue
      compact
      red={definition.fields.redSkulls}
      white={definition.fields.whiteSkulls}
    />{/if}
</div>

<style>
  .slot-wrap {
    display: grid;
    width: 92px;
    justify-items: center;
    gap: 3px;
    text-align: center;
  }
  .slot {
    position: relative;
    width: 80px;
    height: 80px;
    background: #242733;
    border: 1px solid #515765;
    background-size: 100% 100%;
    image-rendering: pixelated;
  }
  .slot-content {
    display: grid;
    width: 100%;
    height: 100%;
    padding: 0;
    place-items: center;
    color: #b5a070;
    background: transparent;
    border: 0;
  }
  .slot-content:hover {
    background: #ffffff0b;
  }
  .slot-content :global(svg) {
    width: 25px;
    height: 25px;
  }
  .remove {
    position: absolute;
    top: 0;
    left: 0;
    min-height: 0;
    padding: 1px;
    color: #c0b5a9;
    line-height: 0;
    background: #242733d9;
    border: 0;
  }
  .remove:hover {
    color: white;
    background: #84312d;
  }
  strong,
  .item-name {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  strong {
    color: var(--cream);
    font-size: 12px;
  }
  .item-name {
    color: var(--muted);
    font-size: 11px;
  }
</style>
