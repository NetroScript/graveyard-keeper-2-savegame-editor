<script lang="ts">
  import type { SaveDocument } from "../document.svelte";
  import General from "./General.svelte";
  import Inspector from "../inspector/Inspector.svelte";
  import FloppyDisk from "~icons/ph/floppy-disk";
  import ArrowCounterClockwise from "~icons/ph/arrow-counter-clockwise";
  import ArrowClockwise from "~icons/ph/arrow-clockwise";
  let {
    doc,
    onsave,
  }: { doc: SaveDocument; onsave: (doc: SaveDocument, as: boolean) => void } =
    $props();
  let view = $state("editor");
  let category = $state("General");
  let inspectorVisited = $state(false);
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
      disabled={!doc.summary?.canUndo || doc.busy}
      onclick={() => doc.mutate("undo").catch(() => {})}
      ><ArrowCounterClockwise /></button
    ><button
      title="Redo"
      aria-label="Redo"
      disabled={!doc.summary?.canRedo || doc.busy}
      onclick={() => doc.mutate("redo").catch(() => {})}
      ><ArrowClockwise /></button
    ><button disabled={doc.busy} onclick={() => onsave(doc, true)}
      >Save As</button
    ><button
      class="primary"
      disabled={doc.busy}
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
  <button class:active={view === "editor"} onclick={() => (view = "editor")}
    >Save Editor</button
  ><button
    class:active={view === "inspector"}
    onclick={() => {
      view = "inspector";
      inspectorVisited = true;
    }}>Save Inspector</button
  >
</nav>
{#if doc.error}<p class="error-banner" role="alert">{doc.error}</p>{/if}
<div hidden={view !== "editor"} class="editor-view">
  <nav class="category-tabs" aria-label="Editor categories">
    {#each ["General", "Inventory", "Relationships", "Technologies"] as tab}<button
        class:active={category === tab}
        onclick={() => (category = tab)}
        >{tab}{#if tab !== "General"}<small>Not available</small>{/if}</button
      >{/each}
  </nav>
  <div class="view-content">
    <div hidden={category !== "General"}><General {doc} /></div>
    {#if category !== "General"}<section class="empty-state panel">
        <h2>{category}</h2>
        <p>
          This editor category is not available yet. Its data remains accessible
          in Save Inspector.
        </p>
      </section>{/if}
  </div>
</div>
<div hidden={view !== "inspector"} class="view-content">
  {#if inspectorVisited}<Inspector {doc} />{/if}
</div>
