<script lang="ts">
  import type { ItemCatalog } from "./catalog";
  let {
    catalog,
    id,
    count = "1",
    durability = null,
  }: {
    catalog: ItemCatalog;
    id: string;
    count?: string;
    durability?: string | null;
  } = $props();
  let image = $state<string>();
  let star = $state<string>();
  const definition = $derived(catalog.items[id]);
  $effect(() => {
    const item = definition;
    let active = true;
    image = undefined;
    star = undefined;
    if (item)
      Promise.allSettled([
        catalog.sprite(item.sprite, "#17181d"),
        catalog.sprite(item.quality.overlaySprite),
      ])
        .then(([i, s]) => {
          if (active) {
            image = i.status === "fulfilled" ? i.value : undefined;
            star = s.status === "fulfilled" ? s.value : undefined;
          }
        })
        .catch(() => {});
    return () => {
      active = false;
    };
  });
</script>

<span class="item-image">
  {#if image}<img class="sprite" src={image} alt="" />{:else}<span
      class="missing"
      title={id}>?</span
    >{/if}
  {#if star}<img class="quality" src={star} alt="Quality" />{/if}
  {#if Number(count) !== 1}<span class="count">{count}</span>{/if}
  {#if durability !== null}<meter
      class="durability"
      min="0"
      max="1"
      value={Number(durability)}
      title={`Durability: ${Math.round(Number(durability) * 100)}%`}
    ></meter><span class="durability-value"
      >{Math.round(Number(durability) * 100)}%</span
    >{/if}
</span>

<style>
  .item-image {
    position: relative;
    display: block;
    width: 100%;
    height: 100%;
    min-width: 32px;
    min-height: 32px;
  }
  .sprite {
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
    padding: 6px;
  }
  .quality {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 6px;
    pointer-events: none;
    object-fit: contain;
    image-rendering: pixelated;
  }
  .count {
    position: absolute;
    right: 4px;
    bottom: 3px;
    color: #ffd13d;
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    text-shadow: 1px 1px #15151b;
  }
  .durability {
    position: absolute;
    left: 4px;
    bottom: 0;
    width: calc(100% - 8px);
    height: 5px;
  }
  .durability-value {
    position: absolute;
    left: 4px;
    bottom: 5px;
    padding: 0 2px;
    color: #d9e3c5;
    background: #15171dcc;
    font-size: 9px;
    line-height: 12px;
    text-shadow: 1px 1px #15151b;
  }
  .missing {
    display: grid;
    place-items: center;
    height: 100%;
    color: #a5a2a0;
  }
</style>
