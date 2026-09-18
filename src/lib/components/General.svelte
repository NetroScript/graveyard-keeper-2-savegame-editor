<script lang="ts">
  import type { SaveDocument, GeneralField } from "../document.svelte";
  let { doc }: { doc: SaveDocument } = $props();
  let fields = $state<GeneralField[]>([]);
  let drafts = $state<Record<string, string>>({});
  let message = $state("");
  const labels: Record<string, string> = {
    hp: "Current health",
    max_hp: "Maximum health",
    energy: "Energy",
    stamina: "Stamina",
    money: "Money · base units",
    tech_red: "Red points",
    tech_green: "Green points",
    tech_blue: "Blue points",
    insanity: "Insanity",
    happiness: "Happiness",
  };
  const groups = [
    { name: "Vitals", keys: ["hp", "max_hp", "energy", "stamina"] },
    { name: "Money", keys: ["money"] },
    {
      name: "Technology Points",
      keys: ["tech_red", "tech_green", "tech_blue"],
    },
    { name: "Mental State", keys: ["insanity", "happiness"] },
  ];
  $effect(() => {
    const revision = doc.summary!.revision;
    let alive = true;
    doc
      .query<GeneralField[]>({ op: "general" })
      .then((v) => {
        if (alive) fields = v;
      })
      .catch((e) => (message = String(e)));
    return () => {
      alive = false;
    };
  });
  async function apply(keys: string[]) {
    const values = Object.fromEntries(
      keys.filter((k) => drafts[k] !== undefined).map((k) => [k, drafts[k]]),
    );
    if (!Object.keys(values).length) return;
    try {
      await doc.transact([{ op: "general", values }]);
      for (const key of keys) delete drafts[key];
      message = "Changes applied";
    } catch {}
  }
  function money(value: string) {
    const amount = Number(value);
    return Number.isFinite(amount)
      ? `${Math.floor(amount / 10000)} gold · ${Math.floor((amount % 10000) / 100)} silver · ${(amount % 100).toFixed(2).replace(/\.00$/, "")} bronze`
      : "";
  }
</script>

<div class="section-intro">
  <div>
    <h2>General</h2>
    <p>Edit player status and resources.</p>
  </div>
</div>
<div class="general-grid">
  {#each groups as group}
    <form
      class="panel"
      onsubmit={(e) => {
        e.preventDefault();
        void apply(group.keys);
      }}
    >
      <h3 class="strip">{group.name}</h3>
      <div class="panel-body">
        {#each group.keys as key}
          {@const field = fields.find((f) => f.key === key)}
          <label class="field"
            ><span>{labels[key]}</span><input
              aria-label={labels[key]}
              type="number"
              step={key === "hp" || key === "max_hp" ? "1" : "any"}
              min={key === "max_hp" ? "1" : "0"}
              disabled={!field || !!field.error || doc.busy}
              value={drafts[key] ?? field?.value ?? ""}
              oninput={(e) => (drafts[key] = e.currentTarget.value)}
            /></label
          >
          {#if field?.error}<p class="hint warning">{field.error}</p>{/if}
          {#if key === "money"}<p class="coin-preview">
              {money(drafts[key] ?? field?.value ?? "0")}
            </p>{/if}
        {/each}
        <div class="form-actions">
          {#if group.name === "Vitals"}<button
              type="button"
              onclick={() => {
                const max = fields.find((f) => f.key === "max_hp");
                if (max?.value) drafts.hp = max.value;
              }}>Restore Health</button
            >{/if}<button
            class="primary"
            disabled={doc.busy ||
              !group.keys.some((k) => drafts[k] !== undefined)}>Apply</button
          >
        </div>
      </div>
    </form>
  {/each}
</div>
<p class="hint" role="status">
  {message ||
    "Changes remain in memory until the save file is written. Resource values use the game’s float32 precision."}
</p>
