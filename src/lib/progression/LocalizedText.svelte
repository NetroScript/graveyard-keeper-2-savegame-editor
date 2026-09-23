<script lang="ts">
  import ProgressionIcon from "./ProgressionIcon.svelte";

  let { text }: { text: string } = $props();

  type Part = { text: string } | { icon: string };

  function split(value: string): Part[] {
    const parts: Part[] = [];
    const sprites = /<sprite\b[^>]*\bname\s*=\s*["']([^"']+)["'][^>]*>/gi;
    let start = 0;
    for (const match of value.matchAll(sprites)) {
      const index = match.index ?? 0;
      if (index > start) parts.push({ text: value.slice(start, index) });
      parts.push({ icon: match[1] });
      start = index + match[0].length;
    }
    if (start < value.length) parts.push({ text: value.slice(start) });
    return parts;
  }

  let parts = $derived(split(text));
</script>

<span class="localized-text">
  {#each parts as part}
    {#if "icon" in part}
      <span class="inline-icon">
        <ProgressionIcon name={part.icon} kind="font" label={part.icon} recolor={false} />
      </span>
    {:else}{part.text}{/if}
  {/each}
</span>

<style>
  .localized-text { white-space:pre-line; }
  .inline-icon { display:inline-flex; width:1.2em; height:1.2em; margin:0 .08em; vertical-align:-.23em; }
  .inline-icon :global(.progression-icon) { --icon-size:1.2em; }
</style>
