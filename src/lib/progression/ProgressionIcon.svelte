<script lang="ts">
  import { loadGameIcon } from "../assets/game-icons";
  let { name, kind = "sprite", label = "", recolor = true }: { name?: string | null; kind?: "sprite" | "font"; label?: string; recolor?: boolean } = $props();
  let url = $state<string>();
  $effect(() => {
    let alive = true;
    url = undefined;
    if (name) loadGameIcon(name, kind, { crop: true, outline: kind === "sprite" && recolor ? "#17181d" : undefined }).then((value) => { if (alive) url = value; });
    return () => { alive = false; };
  });
</script>
<span class="progression-icon">{#if url}<img src={url} alt={label} />{:else}<span aria-hidden="true">?</span>{/if}</span>
<style>
  .progression-icon { position:relative; display:inline-grid; width:var(--icon-size,42px); height:var(--icon-size,42px); min-width:0; min-height:0; place-items:center; overflow:hidden; flex:0 0 var(--icon-size,42px); color:#6f7785; }
  img { position:absolute; inset:0; display:block; width:100%; height:100%; max-width:100%; max-height:100%; object-fit:contain; image-rendering:crisp-edges; image-rendering:pixelated; }
</style>
