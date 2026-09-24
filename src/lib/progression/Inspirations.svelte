<script lang="ts">
  import type { SaveDocument } from "../document.svelte";
  import { dependencyClosure, loadProgressionCatalog, localized, readableId, type InspirationDef, type ProgressionCatalog, type ProgressionState, type TalentLevelNode } from "./catalog";
  import ProgressionIcon from "./ProgressionIcon.svelte";
  import InfoPopover from "./InfoPopover.svelte";
  import LocalizedText from "./LocalizedText.svelte";
  import LoadingIndicator from "../components/LoadingIndicator.svelte";
  let { doc, active = false }: { doc: SaveDocument; active?: boolean } = $props();
  let catalog = $state<ProgressionCatalog>();
  let snapshot = $state<ProgressionState>();
  let branchId = $state("");
  let error = $state("");
  let saving = $state(false);
  let drafts = $state<Record<string, string>>({});
  let selectedPerk = $state<TalentLevelNode>();

  $effect(() => {
    let alive = true;
    loadProgressionCatalog().then((value) => { if (alive) { catalog = value; if (!branchId) branchId = value.talents.branches[0]?.id ?? ""; } }).catch((e) => error = String(e));
    return () => { alive = false; };
  });
  $effect(() => {
    if (!active) return;
    doc.progressionEpoch;
    let alive = true;
    doc.query<ProgressionState>({ op: "progression" }).then((value) => { if (alive) { snapshot = value; drafts = {}; } }).catch((e) => error = String(e));
    return () => { alive = false; };
  });
  const branch = $derived(snapshot?.talents.find((item) => item.id === branchId));
  const inspirations = $derived.by(() => {
    const defs = catalog?.talents.inspirations.filter((item) => item.talent === branchId) ?? [];
    const groups = new Map<string, InspirationDef[]>();
    for (const item of defs) { const group = groups.get(item.baseId) ?? []; group.push(item); groups.set(item.baseId, group); }
    return [...groups.values()].map((levels) => levels.sort((a,b) => a.level-b.level));
  });
  const perkNodes = $derived(catalog?.talents.levelUps.filter((item) => item.talent === branchId) ?? []);
  const studied = $derived(new Set<string>(branch?.studiedLevelUps ?? []));
  const bounds = $derived.by(() => { const xs=perkNodes.map(n=>n.x), ys=perkNodes.map(n=>n.y); return { minX:Math.min(...xs,0),maxX:Math.max(...xs,1),minY:Math.min(...ys,0),maxY:Math.max(...ys,1) }; });
  const px=(x:number)=>7+((x-bounds.minX)/Math.max(1,bounds.maxX-bounds.minX))*86;
  const py=(y:number)=>7+((y-bounds.minY)/Math.max(1,bounds.maxY-bounds.minY))*86;
  const branchName = (id: string, exported: string) => exported !== id ? exported : ({talent_orange:"Building",talent_red:"Metallurgy",talent_green:"Farming",talent_yellow:"Theology",talent_blue:"Anatomy"}[id] ?? id);
  const perkName = (node: TalentLevelNode) => localized(node.name,node.id) ?? localized(node.perk?.name,node.perk?.id ?? "") ?? readableId(node.id);
  const perkDescription = (node: TalentLevelNode) => localized(node.description,`${node.id}_d`) ?? localized(node.perk?.description,`${node.perk?.id}_d`);
  const inspirationDescription = (levels: InspirationDef[]) => levels.map((item) => localized(item.description, `${item.id}_d`)).find(Boolean) ?? null;
  function purchased(levels: InspirationDef[], progress?: { completionGoalValue: string }) {
    let remaining=Number(progress?.completionGoalValue ?? 0), result=0;
    for (const level of levels) { if (remaining < level.completionGoal) break; remaining-=level.completionGoal; result++; }
    return result;
  }
  async function setTalent(field: string, value: string) {
    if (!branch || saving || value.trim()==="") return;
    saving=true; error="";
    try { await doc.transact([{ op:"progression", action:{kind:"set_talent",talent:branch.id,field,value} }]); }
    catch(e){error=String(e);} finally{saving=false;}
  }
  async function setInspiration(levels: InspirationDef[], levelOverride?: number, currentOverride?: number) {
    if (!branch || saving) return;
    const base=levels[0].baseId, progress=branch.inspirations.find(p=>p.id===base);
    const count=Math.max(0,Math.min(levels.length,levelOverride ?? Number(drafts[`${base}:level`] ?? purchased(levels,progress))));
    const maximum=levels[Math.min(count,levels.length-1)]?.completionGoal ?? 0;
    const current=String(Math.max(0,Math.min(maximum,currentOverride ?? Number(drafts[`${base}:current`] ?? progress?.currentValue ?? "0"))));
    const goal=levels.slice(0,count).reduce((sum,item)=>sum+item.completionGoal,0);
    saving=true; error="";
    try { await doc.transact([{op:"progression",action:{kind:"set_inspiration",talent:branch.id,id:base,current_value:current,completion_goal_value:String(goal)}}]); }
    catch(e){error=String(e);} finally{saving=false;}
  }
  function changeProgress(levels: InspirationDef[], chosenLevel: number, value: number) {
    const base=levels[0].baseId;
    drafts[`${base}:current`]=String(value);
    const maximum=levels[Math.min(chosenLevel,levels.length-1)]?.completionGoal ?? 0;
    if (maximum > 0 && value >= maximum && chosenLevel < levels.length) {
      const next=chosenLevel+1;
      drafts[`${base}:level`]=String(next);
      void setInspiration(levels,next,value);
    }
  }
  async function unlockPerk(node: TalentLevelNode) {
    selectedPerk=node;
    if(studied.has(node.id)||saving) return;
    const additions=dependencyClosure(node,perkNodes,studied);
    saving=true; error="";
    try { await doc.transact([{op:"progression",action:{kind:"unlock_talent_levels",talent:branchId,levels:additions.map(item=>({id:item.id,talentValue:item.talentValue}))}}]); }
    catch(e){error=String(e);} finally{saving=false;}
  }
</script>

<div class="section-intro"><div><h2>Inspirations</h2><p>Edit inspiration progress and reveal the full perk tree for each talent.</p></div></div>
{#if error}<p class="error-banner" role="alert">{error}</p>{/if}
{#if catalog && snapshot}
  <nav class="talent-tabs" aria-label="Inspiration categories">
    {#each catalog.talents.branches as item}{@const saved=snapshot.talents.find(t=>t.id===item.id)}
      <button class:active={branchId===item.id} onclick={()=>{branchId=item.id;selectedPerk=undefined;drafts={};}} title={item.name}>
        <ProgressionIcon name={item.fontIcon} kind="font" /><span>{branchName(item.id, item.name)}</span><strong>{saved?.curTalentValue ?? 0}</strong>
      </button>
    {/each}
  </nav>
  {#if branch}
    <section class="branch-summary panel">
      <label>Experience<input type="number" min="0" value={drafts.curExp ?? branch.curExp} oninput={(e)=>drafts.curExp=e.currentTarget.value} onblur={(e)=>void setTalent("curExp",e.currentTarget.value)} /></label>
      <label>Inspiration level<input type="number" min="0" value={drafts.curTalentLevel ?? branch.curTalentLevel} oninput={(e)=>drafts.curTalentLevel=e.currentTarget.value} onblur={(e)=>void setTalent("curTalentLevel",e.currentTarget.value)} /></label>
      <label>Perk points<input type="number" min="0" value={drafts.talentExpPoints ?? branch.talentExpPoints} oninput={(e)=>drafts.talentExpPoints=e.currentTarget.value} onblur={(e)=>void setTalent("talentExpPoints",e.currentTarget.value)} /></label>
      <label>Mastery<input type="number" min="0" value={drafts.curTalentValue ?? branch.curTalentValue} oninput={(e)=>drafts.curTalentValue=e.currentTarget.value} onblur={(e)=>void setTalent("curTalentValue",e.currentTarget.value)} /></label>
    </section>
    <div class="inspiration-layout">
      <section class="inspiration-panel panel"><h3 class="strip">Inspirations</h3><div class="inspiration-list">
        {#each inspirations as levels}{@const base=levels[0].baseId}{@const progress=branch.inspirations.find(p=>p.id===base)}{@const level=purchased(levels,progress)}{@const shown=levels[Math.min(level,levels.length-1)]}{@const chosenLevel=Math.max(0,Math.min(levels.length,Number(drafts[`${base}:level`] ?? level)))}{@const progressMax=levels[Math.min(chosenLevel,levels.length-1)]?.completionGoal ?? 0}{@const progressValue=Math.max(0,Math.min(progressMax,Number(drafts[`${base}:current`] ?? progress?.currentValue ?? "0")))}
          <article class="inspiration-card">
            <InfoPopover title={shown.name} description={inspirationDescription(levels)} facts={[{label:"Unlocked levels",value:`${chosenLevel} / ${levels.length}`},{label:"Current goal",value:String(progressMax)},{label:"Experience",value:String(shown.completionExp)}]}><ProgressionIcon name={shown.sprite} label={shown.name} /></InfoPopover>
            <div class="inspiration-copy"><h4>{shown.name}</h4>{#if inspirationDescription(levels)}<p><LocalizedText text={inspirationDescription(levels)!} /></p>{/if}
              <div class="inspiration-fields"><label class="progress-control">Progress<span><input aria-label={`${shown.name} progress`} type="range" min="0" max={progressMax} value={progressValue} oninput={(e)=>drafts[`${base}:current`]=e.currentTarget.value} onchange={(e)=>changeProgress(levels,chosenLevel,Number(e.currentTarget.value))} /><output>{progressValue} / {progressMax}</output></span></label>
                <fieldset><legend>Unlocked levels</legend><div class="level-buttons">{#each Array(levels.length+1) as _,i}<button type="button" aria-label={`${shown.name}: ${i} unlocked levels`} aria-pressed={chosenLevel===i} onclick={()=>drafts[`${base}:level`]=String(i)}>{i}</button>{/each}</div></fieldset>
                <button disabled={saving} onclick={()=>void setInspiration(levels)}>Apply</button>
              </div>
            </div>
          </article>
        {/each}
      </div></section>
      <section class="perk-panel panel"><h3 class="strip">Perks</h3><div class="perk-tree">
        <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">{#each perkNodes as node}{#each node.parents as id}{@const parent=perkNodes.find(n=>n.id===id)}{#if parent}<path class:unlocked={studied.has(parent.id)&&studied.has(node.id)} d={`M ${px(parent.x)} ${py(parent.y)} L ${px(node.x)} ${py(node.y)}`} />{/if}{/each}{/each}</svg>
        {#each perkNodes as node}<button type="button" class="perk-node" class:unlocked={studied.has(node.id)} class:selected={selectedPerk?.id===node.id} style={`left:calc(${px(node.x)}% - 32px);top:calc(${py(node.y)}% - 32px)`} disabled={saving} onclick={()=>void unlockPerk(node)}><InfoPopover fill title={perkName(node)} description={perkDescription(node)} facts={[{label:"State",value:studied.has(node.id)?"Unlocked":"Locked"},{label:"Point cost",value:String(node.pointPrice)},{label:"Mastery",value:`+${node.talentValue}`}]}><ProgressionIcon name={node.sprite} label={perkName(node)} recolor={false} /><span>{node.pointPrice}</span></InfoPopover></button>{/each}
      </div>
      <div class="perk-details">{#if selectedPerk}<h4>{perkName(selectedPerk)}</h4>{#if perkDescription(selectedPerk)}<p><LocalizedText text={perkDescription(selectedPerk)!} /></p>{/if}<strong>{studied.has(selectedPerk.id)?"Unlocked":"Locked"}</strong>{:else}<p>Hover for perk details. Selecting a locked perk unlocks it and all prerequisites.</p>{/if}</div>
      </section>
    </div>
  {/if}
{:else}<LoadingIndicator label="Loading inspiration definitions and save state…" />{/if}

<style>
  .talent-tabs { display:flex; width:max-content; max-width:100%; margin:0 auto; gap:0; padding:3px; overflow-x:auto; border:1px solid #41434b; background:#191b21; }
  .talent-tabs button { position:relative; z-index:0; min-width:92px; flex:0 0 auto; flex-direction:column; gap:1px; padding:5px 10px; color:#aaa8a4; background:transparent; border:1px solid transparent; }
  .talent-tabs button + button::before { content:""; position:absolute; left:-1px; top:16%; bottom:16%; width:1px; background:#4b4d56; }
  .talent-tabs button.active { z-index:1; border-color:#51525a; color:var(--gold); background:#34353a; }
  .talent-tabs :global(.progression-icon){--icon-size:60px}.talent-tabs strong{position:absolute;top:5px;right:7px;min-width:20px;padding:1px 4px;color:#d3d7df;background:#20232bd9;border:1px solid #555967;font-size:11px}.talent-tabs span{text-transform:capitalize}
  .talent-tabs button:active { transform:none; }
  .branch-summary { display:flex; gap:12px; justify-content:center; flex-wrap:wrap; padding:10px; margin-top:10px; } .branch-summary label{display:flex;align-items:center;gap:7px;color:#b8bdc7}.branch-summary input{width:78px}
  .inspiration-layout{display:grid;grid-template-columns:minmax(430px,1fr) minmax(430px,1fr);gap:12px;margin-top:12px;align-items:start}.inspiration-panel,.perk-panel{min-width:0}
  .inspiration-list{display:grid;grid-template-columns:repeat(auto-fit,minmax(360px,1fr));gap:8px;padding:10px;max-height:660px;overflow:auto}.inspiration-card{display:flex;gap:12px;padding:10px;background:#2b2e36;border:1px solid #444955}.inspiration-card :global(.progression-icon){--icon-size:76px}.inspiration-copy{min-width:0;flex:1}.inspiration-copy h4{margin:0;color:#dcc87d}.inspiration-copy p{margin:4px 0 8px;color:#adb3be;font-size:12px;min-height:30px}.inspiration-fields{display:grid;grid-template-columns:1fr auto;gap:8px;align-items:end}.inspiration-fields label,.inspiration-fields legend{font-size:11px;color:#aeb3be}.progress-control{grid-column:1/-1}.progress-control>span{display:flex;align-items:center;gap:8px;margin-top:4px}.progress-control input{flex:1;min-width:120px}.progress-control output{min-width:52px;text-align:right;color:#e3c36c}.inspiration-fields fieldset{min-width:0;margin:0;padding:0;border:0}.level-buttons{display:flex;flex-wrap:wrap;gap:2px;margin-top:3px}.level-buttons button{min-width:29px;height:29px;padding:3px}.level-buttons button[aria-pressed="true"]{color:#17181d;background:var(--gold);border-color:#f0d68b}.inspiration-fields>button{height:31px}
  .perk-tree{height:560px;min-width:430px;position:relative;background:#24262e;overflow:auto}.perk-tree svg{position:absolute;inset:0;width:100%;height:100%;pointer-events:none}.perk-tree path{stroke:#4a4e59;stroke-width:.8;vector-effect:non-scaling-stroke}.perk-tree path.unlocked{stroke:#d29b25;stroke-width:2}.perk-node{position:absolute;width:64px;height:64px;padding:4px;background:#343743;border-color:#5b6070}.perk-node.unlocked{background:#5b4520;border-color:#d8a91f}.perk-node.selected{outline:2px solid #eee0a2}.perk-node :global(.progression-icon){--icon-size:50px}.perk-node span{position:absolute;right:2px;bottom:0;font-size:10px;color:#ddbadf}.perk-details{min-height:100px;padding:10px;border-top:1px solid var(--border)}.perk-details h4{margin:0;color:var(--gold)}.perk-details p{margin:5px 0;color:#b7bcc6}
  @media(max-width:1050px){.inspiration-layout{grid-template-columns:1fr}.perk-tree{min-width:0}}
</style>
