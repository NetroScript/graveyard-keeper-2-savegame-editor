<script lang="ts">
  import type { WidgetProps } from "./index";
  let { node, children, transact }: WidgetProps = $props();
  let drafts = $state<Record<number, string>>({});
  let error = $state("");
  async function apply() {
    try {
      await transact(
        children
          .filter((c) => drafts[c.id] !== undefined)
          .map((c) => ({
            op: "set",
            node: c.id,
            tag: c.tag,
            value: drafts[c.id],
          })),
      );
      drafts = {};
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
</script>

<form
  onsubmit={(e) => {
    e.preventDefault();
    void apply();
  }}
  class="vector-editor"
>
  <h3>{node.typeName?.split(",")[0].split(".").pop()}</h3>
  <div class="vector-fields">
    {#each children as child, i}<label
        ><span>{["X", "Y", "Z", "W"][i]}</span><input
          aria-label={`Vector ${["X", "Y", "Z", "W"][i]}`}
          type="number"
          step="any"
          value={drafts[child.id] ?? child.value ?? ""}
          oninput={(e) => (drafts[child.id] = e.currentTarget.value)}
        /></label
      >{/each}
  </div>
  <button class="primary" disabled={!Object.keys(drafts).length}
    >Apply vector</button
  >{#if error}<p role="alert">{error}</p>{/if}
</form>
