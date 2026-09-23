<script lang="ts">
  import { tick } from "svelte";
  import type { SaveDocument } from "../document.svelte";
  import { loadProgressionCatalog, dependencyClosure, localized, readableId, type ProgressionCatalog, type ProgressionState, type TechnologyNode } from "./catalog";
  import ProgressionIcon from "./ProgressionIcon.svelte";
  import InfoPopover from "./InfoPopover.svelte";
  import LocalizedText from "./LocalizedText.svelte";
  let { doc, active = false }: { doc: SaveDocument; active?: boolean } = $props();
  let catalog = $state<ProgressionCatalog>();
  let snapshot = $state<ProgressionState>();
  let tab = $state("");
  let selected = $state<TechnologyNode>();
  let error = $state("");
  let saving = $state(false);
  let treePanel = $state<HTMLElement>();

  $effect(() => {
    let alive = true;
    loadProgressionCatalog().then((value) => {
      if (!alive) return;
      catalog = value;
      if (!tab) tab = value.technology.tabs[0]?.id ?? "";
    }).catch((e) => error = String(e));
    return () => { alive = false; };
  });
  $effect(() => {
    if (!active) return;
    doc.progressionEpoch;
    let alive = true;
    doc.query<ProgressionState>({ op: "progression" }).then((value) => { if (alive) snapshot = value; }).catch((e) => error = String(e));
    return () => { alive = false; };
  });
  const nodes = $derived(catalog?.technology.nodes.filter((node) => node.tab === tab) ?? []);
  const unlocked = $derived(new Set<string>(snapshot?.unlockedTechnologies ?? []));
  // The game lays progression depth out from left to right across five lanes.
  const maximumLane = $derived(Math.max(...nodes.map((n) => n.y), 4));
  const canvasWidth = $derived(Math.max(1240, (Math.max(...nodes.map((n) => n.x), 4) + 1) * 220));
  const canvasHeight = $derived(Math.max(660, (maximumLane + 1) * 122 + 50));
  const px = (depth: number) => 105 + depth * 220;
  const py = (lane: number) => 60 + (maximumLane - lane) * 122;
  const available = (node: TechnologyNode) => !unlocked.has(node.id) && (node.parents.length === 0 || (node.lockType === "Any" ? node.parents.some((id) => unlocked.has(id)) : node.parents.every((id) => unlocked.has(id))));
  const rewardName = (reward: TechnologyNode["rewards"][number]) => localized(reward.name, reward.id) ?? readableId(reward.id);
  const rewardFacts = (reward: TechnologyNode["rewards"][number]) => reward.craftedAt?.length ? [{label:"Crafted at", value:reward.craftedAt.join(", ")}] : [];
  const nodeKind = (node: TechnologyNode) => node.type === "CharRep" ? "Character reputation" : node.type === "DisRep" ? "District reputation" : node.hiddenAtStart ? "Revealed by game progress" : "Technology";
  const requirementText = (node: TechnologyNode) => Object.entries(node.requirements ?? {}).filter(([, value]) => value > 0).map(([id, value]) => `${readableId(id)} ${value}`).join(", ");
  const additionalEffectCount = (node: TechnologyNode) => Object.keys(node.additionalEffects?.addResources ?? {}).length + Object.keys(node.additionalEffects?.setResources ?? {}).length + (node.additionalEffects?.expressions.length ?? 0);
  const popoverFacts = (node: TechnologyNode) => [
    {label:"State", value:unlocked.has(node.id)?"Unlocked":"Locked"},
    {label:"Type", value:nodeKind(node)},
    {label:"Prerequisites", value:String(node.parents.length)},
    ...(requirementText(node) ? [{label:"Requirement", value:requirementText(node)}] : []),
    ...(additionalEffectCount(node) ? [{label:"Other game effects", value:String(additionalEffectCount(node))}] : []),
  ];

  async function unlock(node: TechnologyNode) {
    selected = node;
    if (unlocked.has(node.id) || !catalog || saving) return;
    const allNodes = catalog.technology.nodes;
    const additions = dependencyClosure(node, allNodes, unlocked);
    const rewards = additions.flatMap((item) => item.rewards);
    const scrollLeft = treePanel?.scrollLeft ?? 0;
    const scrollTop = treePanel?.scrollTop ?? 0;
    saving = true; error = "";
    try {
      await doc.transact([{ op: "progression", action: { kind: "unlock_technologies", ids: additions.map((item) => item.id), rewards: {
        crafts: rewards.filter((r) => r.type === "craft").map((r) => r.id),
        alchemyFormulas: rewards.filter((r) => r.type === "alchemy").map((r) => r.id),
        buildings: rewards.filter((r) => r.type === "building").map((r) => r.id),
        townBuildings: rewards.filter((r) => r.type === "townBuilding").map((r) => r.id),
        perks: rewards.filter((r) => r.type === "perk").map((r) => ({ id: r.id, duration: String(r.duration ?? 0) })),
      }}}]);
      await tick();
      if (treePanel) { treePanel.scrollLeft = scrollLeft; treePanel.scrollTop = scrollTop; }
    } catch (e) { error = String(e); } finally { saving = false; }
  }
</script>

<div class="section-intro"><div><h2>Technologies</h2><p>Hover for details. Selecting a locked technology unlocks it and all of its prerequisites.</p></div></div>
{#if error}<p class="error-banner" role="alert">{error}</p>{/if}
{#if catalog && snapshot}
  <nav class="branch-tabs" aria-label="Technology trees">
    {#each catalog.technology.tabs as item}
      <button class:active={tab === item.id} onclick={() => { tab = item.id; selected = undefined; }}>
        <ProgressionIcon name={item.sprite} /><span>{item.name}</span>
      </button>
    {/each}
  </nav>
  <div class="progression-layout">
    <section bind:this={treePanel} class="tree-panel panel" aria-label={`${tab} technology tree`}>
      <div class="tree-canvas" style={`width:${canvasWidth}px;height:${canvasHeight}px`}>
        <svg class="connections" viewBox={`0 0 ${canvasWidth} ${canvasHeight}`} style={`width:${canvasWidth}px;height:${canvasHeight}px`} aria-hidden="true">
          {#each nodes as node}{#each node.parents as parentId}
            {@const parent = nodes.find((candidate) => candidate.id === parentId)}
            {#if parent}<path class:unlocked={unlocked.has(parent.id) && unlocked.has(node.id)} d={`M ${px(parent.x)} ${py(parent.y)} C ${(px(parent.x)+px(node.x))/2} ${py(parent.y)}, ${(px(parent.x)+px(node.x))/2} ${py(node.y)}, ${px(node.x)} ${py(node.y)}`} />{/if}
          {/each}{/each}
        </svg>
        {#each nodes as node}
          <button type="button" class="tech-node" class:gate={!!node.gate} class:unlocked={unlocked.has(node.id)} class:available={available(node)} class:selected={selected?.id === node.id} style={`left:${px(node.x)-(node.gate?50:96)}px;top:${py(node.y)-(node.gate?58:52)}px`} disabled={saving} onclick={() => node.gate ? selected=node : void unlock(node)}>
            {#if node.gate}
              <InfoPopover fill title={node.gate.name} facts={popoverFacts(node)}><span class="gate-value">{node.gate.value}</span><ProgressionIcon name={node.gate.sprite} label={node.gate.name} /></InfoPopover>
            {:else}
              <InfoPopover title={node.name} description={localized(node.description, `${node.id}_d`)} facts={popoverFacts(node)}><strong>{node.name}</strong></InfoPopover>
              <span class="reward-icons" style={`--reward-size:${node.rewards.length >= 4 ? 38 : node.rewards.length === 3 ? 48 : 54}px`}>{#each node.rewards.slice(0, 4) as reward}<InfoPopover title={rewardName(reward)} description={localized(reward.description,`${reward.id}_d`)} facts={rewardFacts(reward)} entries={reward.ingredients ?? []}><ProgressionIcon name={reward.sprite} label={reward.name} /></InfoPopover>{/each}</span>
              {#if unlocked.has(node.id)}<span class="status">✓ Unlocked</span>{:else}<span class="prices">{#each Object.entries(node.price).filter(([,v]) => v > 0) as [kind, value]}<span><ProgressionIcon name={kind} kind="font" />{value}</span>{/each}</span>{/if}
            {/if}
          </button>
        {/each}
      </div>
    </section>
    <aside class="details panel">
      {#if selected}
        <h3 class="strip">{selected.name}</h3><div class="details-body">
          {#if localized(selected.description, `${selected.id}_d`)}<p><LocalizedText text={selected.description} /></p>{/if}
          <p class="state"><strong>{unlocked.has(selected.id) ? "Unlocked" : "Locked"}</strong></p>
          {#if selected.type && selected.type !== "Common"}<p class="muted">This is a {nodeKind(selected).toLowerCase()} gate. Unlocking it bypasses that requirement without changing quests or reputation.</p>{:else if selected.hiddenAtStart}<p class="muted">The game normally reveals this technology through game or quest progress. Unlocking it does not mark that quest as completed.</p>{/if}
          {#if additionalEffectCount(selected)}<p class="warning">The game normally applies {additionalEffectCount(selected)} additional side effect{additionalEffectCount(selected) === 1 ? "" : "s"} when this technology unlocks. The editor preserves story state and does not execute game scripts.</p>{/if}
          {#if selected.parents.length}<h4>Prerequisites</h4><ul>{#each selected.parents as id}<li>{catalog.technology.nodes.find((n) => n.id === id)?.name ?? id}</li>{/each}</ul>{/if}
          {#if selected.rewards.length}<h4>Unlocks</h4><div class="reward-list">{#each selected.rewards as reward}<div><InfoPopover title={rewardName(reward)} description={localized(reward.description,`${reward.id}_d`)} facts={rewardFacts(reward)} entries={reward.ingredients ?? []}><ProgressionIcon name={reward.sprite} label={reward.name} /></InfoPopover><span><strong>{rewardName(reward)}</strong>{#if reward.craftedAt?.length}<small>Crafted at: {reward.craftedAt.join(", ")}</small>{/if}{#if localized(reward.description, `${reward.id}_d`)}<small><LocalizedText text={reward.description} /></small>{/if}</span></div>{/each}</div>{/if}
          {#if !unlocked.has(selected.id)}<button class="primary wide" disabled={saving} onclick={() => void unlock(selected!)}>Unlock with prerequisites</button>{/if}
        </div>
      {:else}<div class="details-body muted"><p>Select a technology to see its description, prerequisites and rewards.</p></div>{/if}
    </aside>
  </div>
{:else}<p class="hint">Loading technology definitions and save state…</p>{/if}

<style>
  .branch-tabs { display:flex; width:max-content; max-width:100%; margin:0 auto; gap:0; padding:3px; overflow-x:auto; background:#191b21; border:1px solid #41434b; }
  .branch-tabs button { position:relative; min-width:108px; padding:7px 12px; flex:0 0 auto; flex-direction:column; gap:2px; color:#aaa8a4; background:transparent; border:1px solid transparent; }
  .branch-tabs button + button::before { content:""; position:absolute; left:-1px; top:16%; bottom:16%; width:1px; background:#4b4d56; }
  .branch-tabs button.active { z-index:1; color:var(--gold); border-color:#51525a; background:#34353a; }
  .branch-tabs :global(.progression-icon) { --icon-size:58px; }
  .progression-layout { display:grid; gap:12px; margin-top:12px; }
  .tree-panel { position:relative; overflow:auto; width:100%; min-height:660px; background:#191a20; }
  .tree-canvas { position:relative; }
  .connections { position:absolute; inset:0; pointer-events:none; overflow:visible; }
  path { fill:none; stroke:#393d47; stroke-width:.7; vector-effect:non-scaling-stroke; } path.unlocked { stroke:#d29b25; stroke-width:2; }
  .tech-node { position:absolute; z-index:1; width:192px; min-height:104px; display:flex; flex-direction:column; gap:5px; padding:7px; background:#30333b; border-color:#555b68; color:#b8bdc7; }
  .tech-node.unlocked { background:#5b4523; border-color:#c79a3b; color:#f2dfaa; } .tech-node.available { background:#415630; border-color:#91ad50; color:#f1efda; } .tech-node.selected { outline:2px solid var(--gold); outline-offset:2px; }
  .tech-node.gate { width:100px; min-height:116px; padding:2px; background:transparent; border:0; }
  .tech-node.gate.unlocked { opacity:.55; }
  .tech-node.gate :global(.progression-icon) { --icon-size:92px; }
  .gate-value { position:relative; z-index:1; color:#e34b40; font-size:18px; font-weight:700; line-height:18px; text-shadow:1px 1px #16171c; }
  .tech-node strong { font-size:13px; line-height:1.15; }
  .reward-icons { display:flex; justify-content:center; align-items:center; gap:6px; width:100%; min-height:54px; }
  .reward-icons :global(.progression-icon) { --icon-size:var(--reward-size,54px); }
  .status { color:#c6df75; font-size:11px; } .prices { display:flex; justify-content:center; gap:8px; font-size:11px; }
  .prices > span { display:flex; align-items:center; gap:1px; } .prices :global(.progression-icon) { --icon-size:18px; }
  .details { width:min(100%,900px); } .details-body { padding:14px; } .details-body p { margin-top:0; }
  .details h4 { margin:18px 0 7px; color:var(--gold); } .details ul { padding-left:20px; }
  .reward-list { display:grid; gap:7px; } .reward-list > div { display:flex; align-items:center; gap:8px; padding:6px; background:#252831; border:1px solid #3b3f49; }
  .reward-list :global(.progression-icon) { --icon-size:54px; } .reward-list span { min-width:0; } .reward-list small { display:block; color:#aeb3be; margin-top:2px; }
  .wide { width:100%; margin-top:16px; } .muted { color:#aeb3be; }
  .warning { color:#e2b477; }
</style>
