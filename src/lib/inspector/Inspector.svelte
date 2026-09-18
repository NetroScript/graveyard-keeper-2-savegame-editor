<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import type { SaveDocument } from "../document.svelte";
  import type { NodeView, Operation } from "../save-api";
  import TreeNode from "./TreeNode.svelte";
  import { matchWidget } from "./widgets";
  import DotsSixVertical from "~icons/ph/dots-six-vertical";
  let { doc }: { doc: SaveDocument } = $props();
  let roots = $state<NodeView[]>([]);
  let selected = $state<number | null>(null);
  let node = $state<NodeView>();
  let children = $state<NodeView[]>([]);
  let breadcrumbs = $state<NodeView[]>([]);
  let drafts = $state<Record<number, string>>({});
  let raw = $state(false);
  let error = $state("");
  let name = $state("");
  let value = $state("0");
  let kind = $state("i32");
  let template = $state("");
  let target = $state("");
  let position = $state("0");
  let templates = $state<{ id: string; typeName: string }[]>([]);
  let layout = $state<HTMLDivElement>();
  let treeWidth = $state<number | null>(null);
  const expanded = new SvelteSet<number>();
  let widget = $derived(node ? matchWidget(node, children) : undefined);
  async function refresh() {
    try {
      roots = (
        await doc.query<{ nodes: NodeView[] }>({
          op: "children",
          parent: null,
          offset: 0,
          limit: 200,
        })
      ).nodes;
      if (selected !== null) {
        node = (await doc.nodes([selected]))[0];
        children = (
          await doc.query<{ nodes: NodeView[] }>({
            op: "children",
            parent: selected,
            offset: 0,
            limit: 200,
          })
        ).nodes;
        const trail: NodeView[] = [];
        let cursor = node;
        while (cursor) {
          trail.unshift(cursor);
          if (cursor.parent === null) break;
          cursor = (await doc.nodes([cursor.parent]))[0];
        }
        breadcrumbs = trail;
      }
      templates = await doc.query({ op: "templates" });
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    const revision = doc.summary!.revision;
    const id = selected;
    void refresh();
  });
  async function select(n: NodeView) {
    selected = n.id;
    raw = false;
    error = "";
    try {
      let cursor = n;
      while (cursor.parent !== null) {
        expanded.add(cursor.parent);
        cursor = (await doc.nodes([cursor.parent]))[0];
      }
    } catch (e) {
      error = String(e);
    }
  }
  function boundedTreeWidth(width: number, total: number) {
    return Math.max(220, Math.min(width, total - 320));
  }
  function resizeStart(event: PointerEvent) {
    if (!layout) return;
    event.preventDefault();
    const bounds = layout.getBoundingClientRect();
    const move = (moveEvent: PointerEvent) => {
      treeWidth = boundedTreeWidth(
        moveEvent.clientX - bounds.left,
        bounds.width,
      );
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
    };
    move(event);
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
  }
  function resizeKey(event: KeyboardEvent) {
    if (!layout || !["ArrowLeft", "ArrowRight", "Home"].includes(event.key))
      return;
    event.preventDefault();
    const total = layout.getBoundingClientRect().width;
    if (event.key === "Home") treeWidth = null;
    else {
      const current = treeWidth ?? total * 0.38;
      treeWidth = boundedTreeWidth(
        current + (event.key === "ArrowLeft" ? -24 : 24),
        total,
      );
    }
  }
  async function transaction(operations: Operation[]) {
    try {
      await doc.transact(operations);
      error = "";
    } catch (e) {
      error = String(e);
      throw e;
    }
  }
  async function action(operations: Operation[]) {
    try {
      await transaction(operations);
    } catch {}
  }
  async function remove() {
    if (!node) return;
    const parent = node.parent;
    await action([{ op: "remove", node: node.id }]);
    if (!error) selected = parent;
  }
</script>

<div
  bind:this={layout}
  class="inspector-layout"
  style:--tree-width={treeWidth === null ? "38%" : `${treeWidth}px`}
>
  <div class="panel tree-pane">
    <h3 class="strip">Save structure</h3>
    <ul class="tree">
      {#each roots as root (root.id)}<TreeNode
          {doc}
          node={root}
          {selected}
          onselect={select}
          {expanded}
        />{/each}
    </ul>
  </div>
  <button
    class="pane-grabber"
    aria-label="Resize inspector panes"
    title="Drag to resize panes; press Home to reset"
    onpointerdown={resizeStart}
    onkeydown={resizeKey}
    ondblclick={() => (treeWidth = null)}><DotsSixVertical /></button
  >
  <section class="panel details-pane">
    <h3 class="strip">{node?.name ?? "Record details"}</h3>
    <div class="panel-body">
      {#if node}
        <nav class="breadcrumbs" aria-label="Record path">
          {#each breadcrumbs as crumb}<button onclick={() => void select(crumb)}
              >{crumb.name ?? crumb.kind}</button
            ><span>/</span>{/each}
        </nav>
        <p class="type-name">{node.typeName ?? node.kind}</p>
        {#if widget}<widget.component
            {node}
            {children}
            transact={transaction}
          /><label class="check"
            ><input type="checkbox" bind:checked={raw} />Show raw structure</label
          >{/if}
        {#if !widget || raw}
          <dl class="record-meta">
            <dt>Handle</dt>
            <dd>{node.id}</dd>
            <dt>Wire type</dt>
            <dd>{node.kind} · {node.tag}</dd>
          </dl>
          {#if node.editable}<form
              onsubmit={async (e) => {
                e.preventDefault();
                if (node) {
                  await action([
                    {
                      op: "set",
                      node: node.id,
                      tag: node.tag,
                      value: drafts[node.id] ?? node.value ?? "",
                    },
                  ]);
                  if (!error) delete drafts[node.id];
                }
              }}
            >
              <label class="field stacked"
                >Value<input
                  aria-label="Record value"
                  value={drafts[node.id] ?? node.value ?? ""}
                  oninput={(e) => {
                    if (node) drafts[node.id] = e.currentTarget.value;
                  }}
                /></label
              ><button class="primary" disabled={doc.busy}>Apply value</button>
            </form>{:else if node.value !== null}<pre>{node.value}</pre>{/if}
          {#if node.referenceTarget !== null}<button
              onclick={async () => {
                if (
                  node?.referenceTarget !== null &&
                  node?.referenceTarget !== undefined
                )
                  await select((await doc.nodes([node.referenceTarget]))[0]);
              }}>Go to referenced object</button
            >
            <form
              onsubmit={(e) => {
                e.preventDefault();
                if (node)
                  void action([
                    { op: "retarget", node: node.id, target: Number(target) },
                  ]);
              }}
            >
              <label class="field"
                >Target handle<input
                  type="number"
                  min="0"
                  bind:value={target}
                /></label
              ><button>Retarget reference</button>
            </form>{/if}
          {#if node.childCount}<div class="child-links">
              {#each children as child}<button
                  onclick={() => void select(child)}
                  >{child.name ?? child.kind}<small>{child.value}</small
                  ></button
                >{/each}
            </div>{/if}
          {#if [1, 2, 3, 4, 6].includes(node.tag)}
            <details>
              <summary>Add a field or entry</summary>
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  if (node)
                    void action([
                      {
                        op: "insert",
                        parent: node.id,
                        index: node.childCount,
                        name: node.tag === 6 ? null : name || null,
                        kind,
                        value,
                      },
                    ]);
                }}
              >
                {#if node.tag !== 6}<label class="field"
                    >Name<input bind:value={name} /></label
                  >{/if}
                <label class="field"
                  >Type<select bind:value={kind}
                    >{#each ["i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "f32", "f64", "string", "bool", "null", "array"] as k}<option
                        >{k}</option
                      >{/each}</select
                  ></label
                ><label class="field">Value<input bind:value /></label><button
                  >Add scalar / array</button
                >
              </form>
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  if (node)
                    void action([
                      {
                        op: "template",
                        parent: node.id,
                        index: node.childCount,
                        name: node.tag === 6 ? null : name || null,
                        template,
                        values: {},
                      },
                    ]);
                }}
              >
                <label class="field"
                  >Template<select bind:value={template}
                    ><option value="">Choose a supported type</option
                    >{#each templates as t}<option value={t.id}>{t.id}</option
                      >{/each}</select
                  ></label
                ><button disabled={!template}>Create from template</button>
              </form>
            </details>{/if}
          {#if node.parent !== null}<div class="form-actions">
              <button
                onclick={() =>
                  node && action([{ op: "duplicate", node: node.id }])}
                >Duplicate subtree</button
              ><label class="field"
                >Sibling index<input
                  aria-label="Sibling index"
                  type="number"
                  min="0"
                  bind:value={position}
                /></label
              ><button
                onclick={() =>
                  node &&
                  action([
                    { op: "move", node: node.id, index: Number(position) },
                  ])}>Move</button
              ><button class="danger" onclick={remove}>Remove</button>
            </div>
            <p class="hint">
              Duplication preserves game identifiers, including SGuid.
            </p>{/if}
        {/if}
      {:else}<p>Select a record in the tree to inspect or edit it.</p>{/if}
      {#if error}<p class="warning" role="alert">{error}</p>{/if}
    </div>
  </section>
</div>
