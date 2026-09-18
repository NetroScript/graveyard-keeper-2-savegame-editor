<script lang="ts">
  import { tick } from "svelte";
  import type { SaveDocument } from "../document.svelte";
  import type { NodeView } from "../save-api";
  import TreeNode from "./TreeNode.svelte";
  let {
    doc,
    node,
    selected,
    onselect,
    expanded,
  }: {
    doc: SaveDocument;
    node: NodeView;
    selected: number | null;
    onselect: (node: NodeView) => void | Promise<void>;
    expanded: Set<number>;
  } = $props();
  let row = $state<HTMLDivElement>();
  let isExpanded = $derived(expanded.has(node.id));
  let children = $state<NodeView[]>([]);
  let total = $state(0);
  let error = $state("");
  async function load(offset = 0) {
    try {
      const page = await doc.query<{ nodes: NodeView[]; total: number }>({
        op: "children",
        parent: node.id,
        offset,
        limit: 100,
      });
      children = offset ? [...children, ...page.nodes] : page.nodes;
      total = page.total;
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    const revision = doc.summary!.revision;
    if (isExpanded) void load();
  });
  $effect(() => {
    if (selected === node.id && row) {
      void tick().then(() =>
        row?.scrollIntoView({ block: "nearest", inline: "nearest" }),
      );
    }
  });
</script>

<li>
  <div bind:this={row} class:selected={selected === node.id} class="tree-row">
    <button
      class="expander"
      aria-label={`${isExpanded ? "Collapse" : "Expand"} ${node.name ?? node.kind}`}
      disabled={!node.childCount}
      onclick={() =>
        isExpanded ? expanded.delete(node.id) : expanded.add(node.id)}
      >{node.childCount ? (isExpanded ? "▾" : "▸") : "·"}</button
    >
    <button class="tree-label" onclick={() => void onselect(node)}
      ><span
        >{node.name ??
          node.typeName?.split(",")[0].split(".").pop() ??
          node.kind}</span
      ><small>{node.value ?? `${node.childCount} fields`}</small></button
    >
  </div>
  <ul hidden={!isExpanded}>
    {#each children as child (child.id)}<TreeNode
        {doc}
        node={child}
        {selected}
        {onselect}
        {expanded}
      />{/each}{#if children.length < total}<li>
        <button onclick={() => load(children.length)}
          >Load more ({total - children.length})</button
        >
      </li>{/if}
  </ul>
  {#if error}<small class="warning">{error}</small>{/if}
</li>
