<script lang="ts">
  import { loadGameIcon } from "../assets/game-icons";

  let {
    name,
    kind = "font",
    variant = "default",
  }: {
    name: string;
    kind?: "font" | "sprite";
    variant?: "default" | "church";
  } = $props();
  let url = $state<string>();

  $effect(() => {
    let current = true;
    loadGameIcon(name, kind, {
      crop: true,
      outline: kind === "sprite" ? "#17181d" : undefined,
    }).then((value) => {
      if (current) url = value;
    });
    return () => {
      current = false;
    };
  });
</script>

<span class="game-icon" class:church={variant === "church"} aria-hidden="true">
  {#if url}<img src={url} alt="" />{/if}
</span>

<style>
  .game-icon {
    position: relative;
    display: inline-grid;
    place-items: center;
    width: 24px;
    height: 24px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    flex: 0 0 24px;
  }
  img {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    image-rendering: crisp-edges;
    image-rendering: pixelated;
  }
</style>
