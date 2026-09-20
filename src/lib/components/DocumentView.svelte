<script lang="ts">
  import type { SaveDocument, Settings } from "../document.svelte";
  import Inventory from "../inventory/Inventory.svelte";
  import General from "./General.svelte";
  import Inspector from "../inspector/Inspector.svelte";
  import FloppyDisk from "~icons/ph/floppy-disk";
  import ArrowCounterClockwise from "~icons/ph/arrow-counter-clockwise";
  import ArrowClockwise from "~icons/ph/arrow-clockwise";
  let {
    doc,
    onsave,
    settings,
  }: {
    doc: SaveDocument;
    settings: Settings;
    onsave: (doc: SaveDocument, as: boolean) => void;
  } = $props();
  let view = $state("editor");
  let category = $state("General");
  let inspectorVisited = $state(false);
  let inventoryVisited = $state(false);
</script>

<header class="document-header">
  <div>
    <h1>
      {doc.name}<span class="dirty-label"
        >{doc.summary?.dirty ? "Unsaved changes" : "Saved"}</span
      >
    </h1>
  </div>
  <div class="toolbar">
    <button
      title="Undo"
      aria-label="Undo"
      disabled={!doc.summary?.canUndo || doc.busy || doc.pendingGeneralEdits}
      onclick={() => doc.mutate("undo").catch(() => {})}
      ><ArrowCounterClockwise /></button
    ><button
      title="Redo"
      aria-label="Redo"
      disabled={!doc.summary?.canRedo || doc.busy || doc.pendingGeneralEdits}
      onclick={() => doc.mutate("redo").catch(() => {})}
      ><ArrowClockwise /></button
    ><button
      disabled={doc.busy || doc.pendingGeneralEdits || doc.invalidGeneralDraft}
      onclick={() => onsave(doc, true)}>Save As</button
    ><button
      class="primary"
      disabled={!doc.summary?.dirty ||
        doc.busy ||
        doc.pendingGeneralEdits ||
        doc.invalidGeneralDraft}
      onclick={() => onsave(doc, false)}><FloppyDisk />Save</button
    >
  </div>
</header>
{#if doc.metadata}{@const m = doc.metadata}
  <details class="save-metadata">
    <summary
      >Day {String(m.day ?? "—")} · {String(m.saveDateTime ?? "Unknown date")} · {m.isDemoSave
        ? "Demo"
        : "Release"}</summary
    >
    <dl>
      <dt>Version</dt>
      <dd>{String(m.gameSaveVersion ?? "—")}</dd>
      <dt>Platform</dt>
      <dd>{String(m.platform ?? "—")}</dd>
      <dt>Church quality</dt>
      <dd>{String(m.churchQuality ?? "—")}</dd>
      <dt>Graveyard quality</dt>
      <dd>{String(m.graveyardQuality ?? "—")}</dd>
      <dt>Village reputation</dt>
      <dd>{String(m.villageRep ?? "—")}</dd>
    </dl>
    <p class="hint">
      Metadata records the original game save and is preserved unchanged.
    </p>
  </details>{/if}
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
<div hidden={view !== "editor"} class="editor-view">
  <nav class="category-tabs" aria-label="Editor categories">
    <div class="tab-group">
      {#each ["General", "Inventory", "Relationships", "Technologies"] as tab}<button
          class:active={category === tab}
          onclick={() => {
            category = tab;
            if (tab === "Inventory") inventoryVisited = true;
          }}
          >{tab}{#if tab !== "General" && tab !== "Inventory"}<small
              >Not available</small
            >{/if}</button
        >{/each}
    </div>
  </nav>
  <div class="view-content" class:general-content={category === "General" || category === "Inventory"}>
    <div hidden={category !== "General"}><General {doc} /></div>
    <div hidden={category !== "Inventory"}>
      {#if inventoryVisited}<Inventory {doc} {settings} active={view === "editor" && category === "Inventory"} />{/if}
    </div>
    {#if category !== "General" && category !== "Inventory"}<section
        class="empty-state panel"
      >
        <h2>{category}</h2>
        <p>
          This editor category is not available yet. Its data remains accessible
          in Save Inspector.
        </p>
      </section>{/if}
  </div>
</div>
<div hidden={view !== "inspector"} class="inspector-view">
  {#if inspectorVisited}<Inspector {doc} active={view === "inspector"} />{/if}
</div>
