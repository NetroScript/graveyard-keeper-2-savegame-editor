<script lang="ts">
  import type { SaveDocument, GeneralField } from "../document.svelte";
  let { doc }: { doc: SaveDocument } = $props();
  let fields = $state<GeneralField[]>([]);
  let drafts = $state<Record<string, string>>({});
  let moneyDraft = $state<{ gold: string; silver: string; bronze: string }>();
  let message = $state("");
  const labels: Record<string, string> = {
    hp: "Current health",
    max_hp: "Maximum health",
    energy: "Energy",
    stamina: "Stamina",
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
      if (keys.includes("money")) moneyDraft = undefined;
      message = "Changes applied";
    } catch {}
  }
  function splitMoney(value: string) {
    const amount = Number(value);
    if (!Number.isFinite(amount) || amount < 0)
      return { gold: "0", silver: "0", bronze: "0" };
    return {
      gold: String(Math.floor(amount / 10000)),
      silver: String(Math.floor((amount % 10000) / 100)),
      bronze: String(amount % 100),
    };
  }
  function editMoney(part: "gold" | "silver" | "bronze", value: string) {
    const field = fields.find((entry) => entry.key === "money");
    moneyDraft = {
      ...(moneyDraft ?? splitMoney(field?.value ?? "0")),
      [part]: value,
    };
    const gold = Number(moneyDraft.gold);
    const silver = Number(moneyDraft.silver);
    const bronze = Number(moneyDraft.bronze);
    drafts.money =
      [gold, silver, bronze].every(Number.isFinite) &&
      gold >= 0 &&
      silver >= 0 &&
      silver < 100 &&
      bronze >= 0 &&
      bronze < 100
        ? String(gold * 10000 + silver * 100 + bronze)
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
          {#if key === "money"}
            {@const parts = moneyDraft ?? splitMoney(field?.value ?? "0")}
            <div class="money-fields">
              {#each ["gold", "silver", "bronze"] as part}
                <label
                  ><span>{part[0].toUpperCase() + part.slice(1)}</span><input
                    aria-label={`${part[0].toUpperCase() + part.slice(1)} coins`}
                    type="number"
                    required
                    min="0"
                    max={part === "gold" ? undefined : "99"}
                    step={part === "bronze" ? "any" : "1"}
                    disabled={!field || !!field.error || doc.busy}
                    value={parts[part as keyof typeof parts]}
                    oninput={(event) =>
                      editMoney(
                        part as "gold" | "silver" | "bronze",
                        event.currentTarget.value,
                      )}
                  /></label
                >
              {/each}
            </div>
          {:else}<label class="field"
              ><span>{labels[key]}</span><input
                aria-label={labels[key]}
                type="number"
                step={key === "hp" || key === "max_hp" ? "1" : "any"}
                min={key === "max_hp" ? "1" : "0"}
                disabled={!field || !!field.error || doc.busy}
                value={drafts[key] ?? field?.value ?? ""}
                oninput={(e) => (drafts[key] = e.currentTarget.value)}
              /></label
            >{/if}
          {#if field?.error}<p class="hint warning">{field.error}</p>{/if}
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
