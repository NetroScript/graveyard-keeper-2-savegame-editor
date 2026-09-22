<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { SaveDocument } from "../document.svelte";
  import type { WorldDrop } from "../drops";

  let {
    doc,
    drops,
    onclose,
    onremoved,
  }: {
    doc: SaveDocument;
    drops: WorldDrop[];
    onclose: () => void;
    onremoved: () => void;
  } = $props();

  let dialog: HTMLDialogElement;
  let selected = $state(untrack(() => drops.map((drop) => drop.node)));
  let saving = $state(false);
  let error = $state("");
  const revision = untrack(() => doc.summary!.revision);
  const allSelected = $derived(selected.length === drops.length);

  onMount(() => {
    dialog.showModal();
    return () => dialog.close();
  });

  function toggle(node: number, checked: boolean) {
    selected = checked
      ? [...selected, node]
      : selected.filter((candidate) => candidate !== node);
  }

  function coordinates(drop: WorldDrop) {
    return `${drop.location.x}, ${drop.location.y}, ${drop.location.z}`;
  }

  async function removeSelected() {
    if (!selected.length || saving) return;
    if (doc.summary!.revision !== revision) {
      error = "The document changed. Close and reopen this dialog.";
      return;
    }
    saving = true;
    error = "";
    try {
      await doc.transact([
        { op: "drops", action: { kind: "remove", nodes: selected } },
      ]);
      onremoved();
    } catch (cause) {
      error = String(cause);
    } finally {
      saving = false;
    }
  }
</script>

<dialog
  bind:this={dialog}
  oncancel={(event) => {
    event.preventDefault();
    if (!saving) onclose();
  }}
  aria-labelledby="drop-cleanup-title"
>
  <h2 id="drop-cleanup-title" class="strip">Delete dropped items</h2>
  <form
    onsubmit={(event) => {
      event.preventDefault();
      void removeSelected();
    }}
  >
    <p>
      Select the drops to remove from the save. You can undo this change until
      the document is closed.
    </p>
    <div class="selection-actions">
      <button
        type="button"
        onclick={() => (selected = allSelected ? [] : drops.map((drop) => drop.node))}
      >{allSelected ? "Select none" : "Select all"}</button>
      <span>{selected.length} of {drops.length} selected</span>
    </div>
    <div class="drop-list">
      {#each drops as drop (drop.node)}
        <label class="drop-row">
          <input
            type="checkbox"
            checked={selected.includes(drop.node)}
            onchange={(event) => toggle(drop.node, event.currentTarget.checked)}
          />
          <span class="drop-details">
            <span class="drop-heading">
              <strong>{drop.id}</strong>
              {#if drop.count !== "1"}<span>×{drop.count}</span>{/if}
            </span>
            <span>{drop.type} · {drop.source}</span>
            <span title={`Coordinates: ${coordinates(drop)}`}>
              {drop.location.world || "Unknown location"} · {coordinates(drop)}
            </span>
          </span>
        </label>
      {/each}
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="dialog-actions">
      <button type="button" disabled={saving} onclick={onclose}>Cancel</button>
      <button
        class="danger"
        type="submit"
        disabled={!selected.length || saving || doc.busy}
      >Delete selected</button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(760px, calc(100vw - 32px));
    padding: 0;
    color: inherit;
    background: #272832;
    border: 1px solid #595b65;
    max-height: calc(100dvh - 32px);
  }
  dialog::backdrop {
    background: #0009;
  }
  form {
    padding: 20px;
  }
  form > p {
    margin-top: 0;
  }
  .selection-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin: 16px 0 8px;
    color: #aaa8a3;
    font-size: 12px;
  }
  .drop-list {
    max-height: min(480px, 55dvh);
    overflow: auto;
    border: 1px solid #555966;
    background: #1e2028;
  }
  .drop-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    margin: 0;
    border-bottom: 1px solid #3c3e47;
    cursor: pointer;
  }
  .drop-row:last-child {
    border-bottom: 0;
  }
  .drop-row:hover {
    background: #292c35;
  }
  .drop-details,
  .drop-heading {
    min-width: 0;
    display: flex;
    gap: 8px;
  }
  .drop-details {
    flex-direction: column;
    font-size: 12px;
    color: #aaa8a3;
  }
  .drop-heading {
    justify-content: space-between;
    color: #e2ddd3;
    font-size: 14px;
  }
  .drop-heading strong {
    overflow-wrap: anywhere;
  }
  .error {
    color: #ef9c8f;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
</style>
