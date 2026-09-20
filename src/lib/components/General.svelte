<script lang="ts">
  import { onMount } from "svelte";
  import { loadGeneralIcons } from "../assets/general-icons";
  import type { SaveDocument, GeneralField } from "../document.svelte";
  let { doc }: { doc: SaveDocument } = $props();
  let fields = $state<GeneralField[]>([]);
  let drafts = $state<Record<string, string>>({});
  let moneyDraft = $state<{ gold: string; silver: string; bronze: string }>();
  let message = $state("");
  let icons = $state<Record<string, string>>({});
  onMount(() => {
    let mounted = true;
    loadGeneralIcons()
      .then((value) => {
        if (mounted) icons = value;
      })
      .catch((error) =>
        console.warn("General icons could not be loaded:", error),
      );
    return () => {
      mounted = false;
    };
  });
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
    let alive = true;
    doc
      .query<GeneralField[]>({ op: "general" })
      .then((v) => {
        if (alive) doc.general = v;
      })
      .catch((e) => (message = String(e)));
    return () => {
      alive = false;
    };
  });
  $effect(() => {
    if (doc.general) fields = doc.general;
  });
  let pending: Record<string, string> = {};
  const invalid = new Set<string>();
  function edit(key: string, value: string, valid: boolean) {
    drafts[key] = value;
    if (valid) invalid.delete(key);
    else invalid.add(key);
    doc.invalidGeneralDraft = invalid.size > 0;
    message = invalid.size ? "Enter valid values before saving." : "";
    if (!valid) {
      delete pending[key];
      return;
    }
    pending[key] = value;
    if (!doc.pendingGeneralEdits) void flush();
  }
  async function flush() {
    doc.pendingGeneralEdits = true;
    try {
      while (Object.keys(pending).length) {
        const values = pending;
        pending = {};
        try {
          await doc.transact([{ op: "general", values }]);
          fields = doc.general ?? fields;
          for (const [key, value] of Object.entries(values)) {
            if (drafts[key] === value && !invalid.has(key)) {
              delete drafts[key];
              if (key === "money") moneyDraft = undefined;
            }
          }
        } catch (error) {
          for (const key of Object.keys(values))
            if (!(key in pending)) invalid.add(key);
          doc.invalidGeneralDraft = invalid.size > 0;
          message = String(error);
        }
      }
    } finally {
      doc.pendingGeneralEdits = false;
    }
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
    const valid =
      Object.values(moneyDraft).every((part) => part.trim() !== "") &&
      [gold, silver, bronze].every(Number.isFinite) &&
      Number.isInteger(gold) &&
      Number.isInteger(silver) &&
      gold >= 0 &&
      silver >= 0 &&
      silver < 100 &&
      bronze >= 0 &&
      bronze <= 99;
    edit(
      "money",
      valid ? String(gold * 10000 + silver * 100 + bronze) : "",
      valid,
    );
  }
</script>

<div class="section-intro">
  <div>
    <h2>General</h2>
    <p>Edit player status and resources.</p>
  </div>
</div>
<div class="general-sections">
  {#each groups as group}
    <form
      class="general-section panel"
      onsubmit={(e) => {
        e.preventDefault();
      }}
    >
      <h3 class="strip">{group.name}</h3>
      <div class="general-section-body">
        <div class="general-fields">
          {#each group.keys as key}
            {@const field = fields.find((f) => f.key === key)}
            <div class="general-field">
              {#if key === "money"}
                {@const parts = moneyDraft ?? splitMoney(field?.value ?? "0")}
                <div class="money-fields">
                  {#each ["gold", "silver", "bronze"] as part}
                    <label
                      ><span class="general-field-label"
                        >{#if icons[part]}<img
                            src={icons[part]}
                            alt=""
                            class="general-resource-icon"
                          />{/if}{part[0].toUpperCase() + part.slice(1)}</span
                      ><input
                        aria-label={`${part[0].toUpperCase() + part.slice(1)} coins`}
                        type="number"
                        required
                        min="0"
                        max={part === "gold" ? undefined : "99"}
                        step={part === "bronze" ? "any" : "1"}
                        disabled={!field || !!field.error}
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
                  ><span class="general-field-label"
                    >{#if icons[key]}<img
                        src={icons[key]}
                        alt=""
                        class="general-resource-icon"
                      />{/if}{labels[key]}</span
                  ><input
                    aria-label={labels[key]}
                    type="number"
                    step={key === "hp" || key === "max_hp" ? "1" : "any"}
                    min={key === "max_hp" ? "1" : "0"}
                    required
                    disabled={!field || !!field.error}
                    value={drafts[key] ?? field?.value ?? ""}
                    oninput={(e) =>
                      edit(
                        key,
                        e.currentTarget.value,
                        e.currentTarget.validity.valid,
                      )}
                  /></label
                >{/if}
              {#if field?.error}<p class="hint warning">{field.error}</p>{/if}
            </div>
          {/each}
        </div>
        {#if group.name === "Vitals"}<div class="form-actions">
            <button
              type="button"
              disabled={doc.busy ||
                doc.pendingGeneralEdits ||
                !fields.find((f) => f.key === "hp" && !f.error) ||
                !fields.find((f) => f.key === "max_hp" && !f.error && f.value)}
              onclick={() => {
                const max = fields.find((f) => f.key === "max_hp");
                if (max?.value) edit("hp", max.value, true);
              }}>Restore Health</button
            >
          </div>{/if}
      </div>
    </form>
  {/each}
</div>
<p class="hint" role="status">
  {message ||
    "Changes remain in memory until the save file is written. Resource values use the game’s float32 precision."}
</p>

<style>
  .general-sections {
    width: 100%;
    display: grid;
    gap: 20px;
  }
  .general-section {
    min-width: 0;
  }
  .general-section-body {
    min-width: 0;
    padding: 20px;
  }
  .general-fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 160px), 1fr));
    gap: 16px;
  }
  .general-field {
    min-width: 0;
  }
  .field {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    margin: 0;
  }
  .money-fields {
    gap: 16px;
  }
  .money-fields .general-field-label {
    margin-bottom: 8px;
    color: inherit;
    font-size: inherit;
  }
  .form-actions {
    margin-top: 16px;
  }
  @media (min-width: 1200px) {
    .general-section-body {
      display: flex;
      align-items: start;
      gap: 24px;
    }
    .general-fields {
      flex: 1;
      grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    }
    .form-actions {
      flex: 0 0 128px;
      flex-direction: column;
      align-items: stretch;
      margin-top: 28px;
    }
  }
  .general-field-label {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .general-resource-icon {
    width: 20px;
    height: 20px;
    flex: 0 0 20px;
    object-fit: contain;
    image-rendering: pixelated;
  }
</style>
