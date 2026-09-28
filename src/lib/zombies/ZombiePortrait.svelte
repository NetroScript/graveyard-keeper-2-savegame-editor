<script lang="ts">
  import { loadZombiePortrait } from "../assets/zombie-portraits";
  import type { ZombieState } from "./catalog";

  let {
    appearance,
    alt,
    includeStone = true,
    crop = true,
  }: {
    appearance: ZombieState["appearance"];
    alt: string;
    includeStone?: boolean;
    crop?: boolean;
  } = $props();
  let url = $state<string>();

  $effect(() => {
    const value = appearance;
    let current = true;
    url = undefined;
    loadZombiePortrait({
      setId: value.set,
      bodyId: value.body,
      headId: value.head,
      bodyLut: value.bodyLut || undefined,
      headLut: value.headLut || undefined,
      includeStone,
      crop,
    })
      .then((result) => {
        if (current) url = result;
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  });
</script>

<span class="zombie-portrait">
  {#if url}<img src={url} {alt} />{:else}<span aria-hidden="true">?</span>{/if}
</span>

<style>
  .zombie-portrait {
    display: grid;
    width: 100%;
    height: 100%;
    place-items: center;
    overflow: hidden;
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    image-rendering: pixelated;
  }
</style>
