<script lang="ts">
  import type { SaveDocument, Settings } from "../document.svelte";
  import Inventory from "../inventory/Inventory.svelte";
  import General from "./General.svelte";
  import Inspector from "../inspector/Inspector.svelte";
  import Technologies from "../progression/Technologies.svelte";
  import Inspirations from "../progression/Inspirations.svelte";
  import FloppyDisk from "~icons/ph/floppy-disk";
  import ArrowCounterClockwise from "~icons/ph/arrow-counter-clockwise";
  import ArrowClockwise from "~icons/ph/arrow-clockwise";
  import DesktopTower from "~icons/ph/desktop-tower";
  import Tag from "~icons/ph/tag";
  import GameIcon from "./GameIcon.svelte";
  import { gameDayIcon } from "../assets/game-icons";
  let {
    doc,
    onsave,
    settings,
    assetVersion,
  }: {
    doc: SaveDocument;
    settings: Settings;
    assetVersion: string;
    onsave: (doc: SaveDocument, as: boolean) => void;
  } = $props();
  let view = $state("editor");
  let category = $state("General");
  let inspectorVisited = $state(false);
  let inventoryVisited = $state(false);
  let technologiesVisited = $state(false);
  let inspirationsVisited = $state(false);
  function versionParts(value: unknown) {
    const match = String(value ?? "").match(/^\d+(?:\.\d+)+$/);
    return match ? match[0].split(".").map(Number) : null;
  }
  const newerSaveVersion = $derived.by(() => {
    const save = versionParts(doc.metadata?.gameSaveVersion);
    const assets = versionParts(assetVersion);
    if (!save || !assets) return false;
    for (let index = 0; index < Math.max(save.length, assets.length); index++) {
      const difference = (save[index] ?? 0) - (assets[index] ?? 0);
      if (difference) return difference > 0;
    }
    return false;
  });
</script>

<header class="document-header">
  <div class="document-title">
    <h1>
      {doc.name}<span class="dirty-label"
        >{doc.summary?.dirty ? "Unsaved changes" : "Saved"}</span
      >
    </h1>
  </div>
  {#if doc.metadata}{@const m = doc.metadata}
    <details class="save-metadata">
      <summary title="Show complete save information">
        <GameIcon name={gameDayIcon(m.day)} />
        <strong>Day {String(m.day ?? "—")}</strong>
        <span class="metadata-date">{String(m.saveDateTime ?? "Unknown date")}</span>
        <span class="metadata-summary-fields">
          <span>Version {String(m.gameSaveVersion ?? "—")}</span>
          <span>{String(m.platform ?? "—")}</span>
          <span>{m.isDemoSave ? "Demo" : "Release"}</span>
        </span>
      </summary>
      <div class="metadata-expanded">
        <dl>
          <dt><Tag />Version</dt>
          <dd>{String(m.gameSaveVersion ?? "—")}</dd>
          <dt><DesktopTower />Platform</dt>
          <dd>{String(m.platform ?? "—")}</dd>
          <dt><GameIcon name="wskull" />Graveyard quality</dt>
          <dd>{String(m.graveyardQuality ?? "—")}</dd>
          <dt><GameIcon name="cross" variant="church" />Church quality</dt>
          <dd>{String(m.churchQuality ?? "—")}</dd>
          <dt><GameIcon name="village_REP" />Village reputation</dt>
          <dd>{String(m.villageRep ?? "—")}</dd>
        </dl>
        <p class="hint">
          Metadata records the original game save and is preserved unchanged.
        </p>
      </div>
    </details>
  {/if}
  <div class="toolbar">
    <button
      title="Undo"
      aria-label="Undo"
      disabled={!doc.summary?.canUndo || doc.busy}
      onclick={() => doc.mutate("undo").catch(() => {})}
      ><ArrowCounterClockwise /></button
    ><button
      title="Redo"
      aria-label="Redo"
      disabled={!doc.summary?.canRedo || doc.busy}
      onclick={() => doc.mutate("redo").catch(() => {})}
      ><ArrowClockwise /></button
    ><button
      disabled={doc.busy || doc.invalidGeneralDraft}
      onclick={() => onsave(doc, true)}>Save As</button
    ><button
      class="primary"
      disabled={!doc.summary?.dirty || doc.busy || doc.invalidGeneralDraft}
      onclick={() => onsave(doc, false)}><FloppyDisk />Save</button
    >
  </div>
</header>
<nav class="main-tabs" aria-label="Save views">
  <div class="tab-group">
    <button class:active={view === "editor"} onclick={() => (view = "editor")}
      >Save Editor</button
    ><button
      class:active={view === "inspector"}
      onclick={() => {
        view = "inspector";
        inspectorVisited = true;
      }}>Save Inspector</button
    >
  </div>
</nav>
{#if doc.error}<p class="error-banner" role="alert">{doc.error}</p>{/if}
{#if newerSaveVersion}<p class="compatibility-note warning" role="status">
    This save is from game version {String(doc.metadata?.gameSaveVersion)}, but
    the loaded assets describe {assetVersion}. New items or progression data may
    be shown by ID or unavailable for structured editing.
  </p>{/if}
{#if doc.unknownItemIds.length}<p
    class="compatibility-note warning"
    role="status"
  >
    {doc.unknownItemIds.length} item {doc.unknownItemIds.length === 1
      ? "definition is"
      : "definitions are"} missing from the loaded assets. Existing data is preserved,
    but those items cannot be safely added or replaced.
  </p>{/if}
<div hidden={view !== "editor"} class="editor-view">
  <nav class="category-tabs" aria-label="Editor categories">
    <div class="tab-group">
      {#each ["General", "Inventory", "Technologies", "Inspirations"] as tab}<button
          class:active={category === tab}
          onclick={() => {
            category = tab;
            if (tab === "Inventory") inventoryVisited = true;
            if (tab === "Technologies") technologiesVisited = true;
            if (tab === "Inspirations") inspirationsVisited = true;
          }}>{tab}</button
        >{/each}
    </div>
  </nav>
  <div class="view-content general-content">
    <div hidden={category !== "General"}><General {doc} /></div>
    <div hidden={category !== "Inventory"}>
      {#if inventoryVisited}<Inventory
          {doc}
          {settings}
          active={view === "editor" && category === "Inventory"}
        />{/if}
    </div>
    <div hidden={category !== "Technologies"}>
      {#if technologiesVisited}<Technologies
          {doc}
          active={view === "editor" && category === "Technologies"}
        />{/if}
    </div>
    <div hidden={category !== "Inspirations"}>
      {#if inspirationsVisited}<Inspirations
          {doc}
          active={view === "editor" && category === "Inspirations"}
        />{/if}
    </div>
  </div>
</div>

<div hidden={view !== "inspector"} class="inspector-view">
  {#if inspectorVisited}<Inspector {doc} active={view === "inspector"} />{/if}
</div>

<style>
  .document-header {
    position: relative;
    display: grid;
    grid-template-columns: minmax(180px, auto) minmax(180px, 1fr) auto;
  }
  .document-title {
    min-width: 0;
  }
  .save-metadata {
    position: relative;
    min-width: 0;
    padding: 0;
    color: #b8b2a8;
    background: transparent;
    border: 0;
  }
  .save-metadata summary {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    overflow: hidden;
    white-space: nowrap;
    background: #20232b;
    border: 1px solid #4b4d56;
  }
  .save-metadata summary::-webkit-details-marker {
    display: none;
  }
  .save-metadata summary::after {
    content: "▾";
    flex: 0 0 auto;
    color: #c49a4d;
  }
  .save-metadata[open] summary::after {
    content: "▴";
  }
  .save-metadata summary :global(.game-icon) {
    width: 28px;
    height: 28px;
    flex-basis: 28px;
  }
  .metadata-date {
    overflow: hidden;
    color: #a7aab2;
    text-overflow: ellipsis;
  }
  .metadata-summary-fields {
    display: flex;
    min-width: 0;
    gap: 10px;
    margin-left: auto;
    overflow: hidden;
    color: #8f949f;
    font-size: 11px;
  }
  .metadata-summary-fields span {
    flex: 0 0 auto;
  }
  .metadata-expanded {
    position: absolute;
    z-index: 40;
    top: calc(100% + 7px);
    left: 0;
    width: min(440px, calc(100vw - 48px));
    padding: 14px 16px;
    color: #b8b2a8;
    background: #292a33;
    border: 1px solid #5b5d66;
  }
  .metadata-expanded dl {
    max-width: none;
    margin: 0;
  }
  .metadata-expanded .hint {
    margin: 12px 0 0;
  }
  .compatibility-note {
    margin: 10px 20px 0;
    padding: 9px 12px;
  }
  @media (max-width: 1100px) {
    .metadata-summary-fields {
      display: none;
    }
  }
  @media (max-width: 760px) {
    .document-header {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .save-metadata {
      grid-column: 1 / -1;
      grid-row: 2;
    }
    .metadata-expanded {
      width: min(440px, calc(100vw - 40px));
    }
  }
</style>
