<script lang="ts">
  import type { Snippet } from "svelte";
  import ProgressionIcon from "./ProgressionIcon.svelte";
  import LocalizedText from "./LocalizedText.svelte";
  let { title, description, facts = [], items = [], entries = [], fill = false, children }: { title: string; description?: string | null; facts?: { label: string; value: string }[]; items?: string[]; entries?: { name: string; count: number; sprite: string | null }[]; fill?: boolean; children: Snippet } = $props();
  let anchor: HTMLSpanElement;
  let popover: HTMLElement;
  let left = $state(0);
  let top = $state(0);
  function show() {
    const rect = anchor.getBoundingClientRect();
    left = Math.max(12, Math.min(window.innerWidth - 332, rect.left + rect.width / 2 - 150));
    top = rect.bottom + 8;
    if (!popover.matches(":popover-open")) popover.showPopover();
    requestAnimationFrame(() => {
      const box = popover.getBoundingClientRect();
      left = Math.max(12, Math.min(window.innerWidth - box.width - 12, rect.left + rect.width / 2 - box.width / 2));
      top = rect.bottom + 8;
      if (top + box.height > window.innerHeight - 12) top = Math.max(12, rect.top - box.height - 8);
    });
  }
  function hide() { if (popover?.matches(":popover-open")) popover.hidePopover(); }
</script>

<span class="anchor" class:fill role="presentation" bind:this={anchor} onmouseenter={show} onmouseleave={hide} onfocusin={show} onfocusout={hide}>
  {@render children()}
</span>
<span bind:this={popover} popover="manual" class="popover" role="tooltip" style={`left:${left}px;top:${top}px`}>
  <strong>{title}</strong>
  {#if description}<span class="description"><LocalizedText text={description} /></span>{/if}
  {#if facts.length}<dl>{#each facts as fact}<div><dt>{fact.label}</dt><dd>{fact.value}</dd></div>{/each}</dl>{/if}
  {#if entries.length}<span class="list-title">Materials</span><span class="entries">{#each entries as entry}<span class="entry"><ProgressionIcon name={entry.sprite} label={entry.name} /><span>{entry.name}</span><b>{entry.count}</b></span>{/each}</span>{/if}
  {#if items.length}<span class="list-title">Unlocks</span><ul>{#each items as item}<li>{item}</li>{/each}</ul>{/if}
</span>

<style>
  .anchor { display:inline-flex; min-width:0; }
  .anchor.fill { width:100%; flex-direction:column; align-items:center; gap:4px; }
  .popover { position:fixed; inset:auto; width:300px; margin:0; padding:10px 12px; text-align:left; color:#e2e3e8; background:#24262d; border:1px solid #9b814c; pointer-events:none; line-height:1.35; }
  .popover strong { display:block; color:#e3c36c; font-size:13px; }
  .description { display:block; margin-top:5px; color:#c1c5ce; font-size:12px; white-space:normal; }
  dl { margin:8px 0 0; padding-top:6px; border-top:1px solid #454953; font-size:11px; }
  dl div { display:flex; justify-content:space-between; gap:12px; } dt { color:#969daa; } dd { margin:0; text-align:right; }
  .list-title { display:block; margin-top:8px; padding-top:6px; color:#969daa; border-top:1px solid #454953; font-size:11px; }
  .entries { display:flex; justify-content:center; gap:8px; margin-top:6px; }
  .entry { position:relative; display:grid; justify-items:center; min-width:58px; color:#c8cbd2; font-size:10px; }
  .entry :global(.progression-icon) { --icon-size:48px; }
  .entry b { position:absolute; top:34px; right:3px; color:#f1c72e; font-size:13px; text-shadow:1px 1px #17181d; }
  ul { margin:4px 0 0; padding-left:17px; color:#d7d9df; font-size:11px; }
  li + li { margin-top:2px; }
</style>
