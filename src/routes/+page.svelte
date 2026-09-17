<script lang="ts">
  import { onDestroy } from "svelte";
  import { createBackend, type SaveBackend, type Summary, type NodeView } from "$lib/save-api";

  let backend: SaveBackend | undefined;
  let summary = $state<Summary | null>(null);
  let filename = $state("");
  let busy = $state(false);
  let error = $state("");
  let message = $state("");
  let nodes = $state<NodeView[]>([]);
  let total = $state(0);
  let offset = $state(0);
  let path = $state<{ id: number | null; label: string }[]>([{ id: null, label: "Save" }]);
  let selected = $state<NodeView | null>(null);
  let value = $state("");
  const pageSize = 100;
  onDestroy(() => backend?.dispose());

  async function run(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    error = "";
    try { await action(); } catch (e) { error = String(e); } finally { busy = false; }
  }
  async function refresh() {
    if (!backend) return;
    const response = await backend.request({ op: "children", parent: path.at(-1)!.id, offset, limit: pageSize });
    if (response.op === "children") { nodes = response.data.nodes; total = response.data.total; }
  }
  async function open(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    await run(async () => {
      if (file.size > 128 * 1024 * 1024) throw new Error("Save exceeds the 128 MiB limit");
      backend ??= await createBackend();
      const opened = await backend.open(new Uint8Array(await file.arrayBuffer()));
      summary = opened;
      filename = file.name;
      path = [{ id: null, label: "Save" }]; offset = 0; selected = null;
      message = "Loaded. Export without edits preserves the original bytes.";
      await refresh();
    });
  }
  async function enter(node: NodeView) {
    await run(async () => {
      path = [...path, { id: node.referenceTarget ?? node.id, label: node.name ?? node.typeName ?? `#${node.id}` }];
      offset = 0; selected = null; await refresh();
    });
  }
  async function navigate(index: number) {
    await run(async () => { path = path.slice(0, index + 1); offset = 0; selected = null; await refresh(); });
  }
  async function page(delta: number) {
    await run(async () => { offset += delta * pageSize; selected = null; await refresh(); });
  }
  async function apply(event: SubmitEvent) {
    event.preventDefault();
    await run(async () => {
      if (!backend || !selected || !summary) return;
      const response = await backend.request({ op: "set_value", edit: {
        node: selected.id, expectedTag: selected.tag, revision: summary.revision, value,
      } });
      if (response.op === "set_value") summary = response.data;
      message = `Updated ${selected.name ?? selected.kind} in memory. Export to save a copy.`;
      selected = null; await refresh();
    });
  }
  async function download() {
    await run(async () => {
      const bytes = await backend!.export();
      const url = URL.createObjectURL(new Blob([bytes.slice().buffer], { type: "application/octet-stream" }));
      const link = document.createElement("a");
      link.href = url; link.download = filename.replace(/\.dat$/i, "") + ".edited.dat";
      document.body.append(link); link.click(); link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 60_000);
      message = "Exported a copy. The original save has not been overwritten.";
    });
  }
</script>

<svelte:head><title>Graveyard Keeper 2 · Save inspector</title></svelte:head>
<main>
  <header>
    <p class="eyebrow">GRAVEYARD KEEPER 2</p>
    <h1>Save inspector</h1>
    <p>Open a .dat save to inspect its fields and edit existing scalar values.</p>
    <div class="toolbar">
      <label class="file">Open save <input aria-label="Open save" type="file" accept=".dat" onchange={open} disabled={busy} /></label>
      <button onclick={download} disabled={!summary || busy}>Export copy</button>
      {#if busy}<span role="status">Working…</span>{/if}
    </div>
  </header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if message}<p role="status">{message}</p>{/if}
  {#if summary}
    <section class="summary">
      <strong>{filename}</strong>
      <span>{summary.encodedBytes.toLocaleString()} bytes</span>
      <span>{summary.records.toLocaleString()} records</span>
      <span>{summary.types} types</span>
    </section>
    <p class="hint">Raw field edits do not apply game rules or update the companion .info metadata. Container and reference changes are read-only.</p>
    <nav aria-label="Save path">
      {#each path as part, index}<button disabled={busy} onclick={() => navigate(index)}>{part.label}</button>{/each}
    </nav>
    <div class="table-wrap">
      <table>
        <thead><tr><th>Field</th><th>Type</th><th>Value</th><th>Action</th></tr></thead>
        <tbody>
          {#each nodes as node (node.id)}
            <tr>
              <td title={`Node ${node.id}; original byte offset ${node.originalOffset}`}>{node.name ?? `[${node.id}]`}</td>
              <td title={node.typeName ?? node.kind}>{node.typeName?.split(",")[0] ?? node.kind}</td>
              <td class="value" title={node.value ?? ""}>{node.value ?? "—"}</td>
              <td>
                {#if node.childCount > 0 || node.referenceTarget !== null}
                  <button disabled={busy} onclick={() => enter(node)}>{node.referenceTarget !== null ? "Follow reference" : `Open (${node.childCount})`}</button>
                {:else if node.editable}
                  <button disabled={busy} onclick={() => { selected = node; value = node.value ?? ""; }}>Edit</button>
                {/if}
              </td>
            </tr>
          {:else}<tr><td colspan="4">No child fields.</td></tr>{/each}
        </tbody>
      </table>
    </div>
    <div class="toolbar">
      <button disabled={busy || offset === 0} onclick={() => page(-1)}>Previous</button>
      <span>{total ? offset + 1 : 0}–{Math.min(offset + pageSize, total)} of {total}</span>
      <button disabled={busy || offset + pageSize >= total} onclick={() => page(1)}>Next</button>
    </div>
    {#if selected}
      <form onsubmit={apply}>
        <label for="edit-value">{selected.name ?? selected.kind} ({selected.kind})</label>
        <textarea id="edit-value" bind:value disabled={busy} rows="3"></textarea>
        <div class="toolbar"><button disabled={busy} type="submit">Apply edit</button><button disabled={busy} type="button" onclick={() => selected = null}>Cancel</button></div>
      </form>
    {/if}
  {/if}
</main>

<style>
  :global(body) { margin: 0; background: #171c1b; color: #e5e9e2; font-family: system-ui, sans-serif; }
  main { max-width: 1150px; padding: 2.5rem 1.5rem; margin: auto; }
  h1 { margin: .4rem 0; font-size: 2.2rem; }
  .eyebrow { color: #c9b984; letter-spacing: .15em; font-size: .75rem; }
  p { color: #b8c2b8; }
  .toolbar, nav, .summary { display: flex; flex-wrap: wrap; gap: .75rem; align-items: center; margin: 1rem 0; }
  .summary { padding: 1rem; background: #242d28; border-radius: .5rem; }
  button, .file { border: 1px solid #697765; border-radius: .35rem; background: #303c31; color: #f1f3eb; padding: .5rem .8rem; font: inherit; cursor: pointer; }
  button:disabled { opacity: .45; cursor: default; }
  button:hover:not(:disabled) { background: #465442; }
  input { max-width: 15rem; margin-left: .5rem; }
  .hint { font-size: .85rem; }
  .error { color: #ffb5a8; white-space: pre-wrap; }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; text-align: left; font-size: .9rem; }
  th, td { padding: .7rem; border-bottom: 1px solid #344037; }
  th { color: #c9b984; }
  .value { max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  form { padding: 1rem; background: #242d28; border-radius: .5rem; }
  textarea { box-sizing: border-box; width: 100%; margin-top: .5rem; background: #171c1b; color: #e5e9e2; border: 1px solid #697765; padding: .75rem; }
</style>
