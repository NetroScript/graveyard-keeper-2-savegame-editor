<script lang="ts">
  import type { WidgetProps } from "./index";

  const guidPattern =
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  let { children, transact }: WidgetProps = $props();
  let field = $derived(children.find((child) => child.name === "id")!);
  let fieldId = $state<number>();
  let draft = $state("");
  let error = $state("");
  let valid = $derived(guidPattern.test(draft.trim()));

  $effect(() => {
    if (fieldId !== field.id) {
      fieldId = field.id;
      draft = field.value ?? "";
      error = "";
    }
  });

  async function apply() {
    const value = draft.trim().toLowerCase();
    if (!valid) return;
    try {
      await transact([{ op: "set", node: field.id, tag: field.tag, value }]);
      draft = value;
      error = "";
    } catch (cause) {
      error = String(cause);
    }
  }
</script>

<form
  class="guid-editor"
  onsubmit={(event) => {
    event.preventDefault();
    void apply();
  }}
>
  <h3>SGuid</h3>
  <label class="field stacked"
    >GUID<input
      aria-label="GUID"
      autocomplete="off"
      spellcheck="false"
      bind:value={draft}
    /></label
  >
  {#if draft && !valid}<p class="warning" role="status">
      Enter a GUID in the form 00000000-0000-0000-0000-000000000000.
    </p>{/if}
  <button
    type="button"
    class="primary"
    disabled={!valid ||
      draft.trim().toLowerCase() === field.value?.toLowerCase()}
    onclick={() => void apply()}>Apply GUID</button
  >
  {#if error}<p class="warning" role="alert">{error}</p>{/if}
</form>
