<script lang="ts">
  import { onMount, untrack } from "svelte";
  import CaretLeft from "~icons/ph/caret-left";
  import CaretRight from "~icons/ph/caret-right";
  import MagicWand from "~icons/ph/magic-wand";
  import type { SaveDocument, Settings } from "../document.svelte";
  import { gameAssets } from "../assets/game-assets";
  import GameIcon from "../components/GameIcon.svelte";
  import {
    ItemCatalog,
    plainText,
    type ItemDefinition,
  } from "../inventory/catalog";
  import ProgressionIcon from "../progression/ProgressionIcon.svelte";
  import { notify, notifyError } from "../toasts.svelte";
  import ZombieItemDialog from "./ZombieItemDialog.svelte";
  import ZombiePortrait from "./ZombiePortrait.svelte";
  import ZombieSlot from "./ZombieSlot.svelte";
  import SkullValue from "./SkullValue.svelte";
  import {
    bestBodyItems,
    bestEquipment,
    loadZombieCatalog,
    zombieAppearanceSetId,
    type ZombieBodyItem,
    type ZombieCatalog,
    type ZombieSnapshot,
    type ZombieState,
  } from "./catalog";

  let {
    doc,
    settings,
    active,
    oninspect,
  }: {
    doc: SaveDocument;
    settings: Settings;
    active: boolean;
    oninspect?: (node: number) => void;
  } = $props();

  const organTypes = [
    ["brain", "Brain"],
    ["heart", "Heart"],
    ["guts", "Intestines"],
    ["skull", "Skull"],
    ["bones", "Bones"],
    ["skin", "Skin"],
  ] as const;
  const equipmentSlots = [
    ["collar", "Collar"],
    ["armor", "Armor"],
    ["hand", "Tool"],
  ] as const;

  let catalog = $state<ZombieCatalog>();
  let items = $state<ItemCatalog>();
  let snapshot = $state<ZombieSnapshot>();
  let selectedNode = $state<number>();
  let detailTab = $state("Overview");
  let branchId = $state("");
  let spendPoints = $state(false);
  let loadError = $state("");
  let loading = $state(true);
  let initialized = $state(false);
  let loadedZombieEpoch = $state(-1);
  let slotBackground = $state<string>();
  let nameDraft = $state("");
  let pointDraft = $state({ red: "0", green: "0", blue: "0" });
  let appearanceDraft = $state({
    set: "zombie_worker",
    body: 1001,
    head: 1050,
    bodyLut: "",
    headLut: "",
  });
  let itemDialog = $state<{
    title: string;
    current?: string;
    filter: (item: ItemDefinition) => boolean;
    confirm: (id: string, count?: string) => Promise<void>;
    clear?: () => Promise<void>;
    mode?: "organ" | "tool" | "armor" | "body" | "collar" | "cargo";
    preferredTalent?: string;
    currentCount?: string;
  }>();

  const selected = $derived(
    snapshot?.zombies.find((zombie) => zombie.node === selectedNode),
  );
  const loadedItems = $derived(items as ItemCatalog);
  const appearanceSet = $derived(
    catalog?.assets.customization.sets.find(
      (set) => set.id === appearanceDraft.set,
    ),
  );
  const hasUnsupportedAppearance = $derived(
    Boolean(
      selected &&
      appearanceSet &&
      (selected.appearance.set !== zombieAppearanceSetId(selected.id) ||
        !appearanceSet.bodyIds.includes(selected.appearance.body) ||
        !appearanceSet.headIds.includes(selected.appearance.head) ||
        (selected.appearance.bodyLut !== "" &&
          !appearanceSet.bodyLuts.some(
            (entry) => entry.name === selected.appearance.bodyLut,
          )) ||
        (selected.appearance.headLut !== "" &&
          !appearanceSet.headLuts.some(
            (entry) => entry.name === selected.appearance.headLut,
          ))),
    ),
  );
  const selectedBranch = $derived(
    catalog?.branches.find((branch) => branch.id === branchId),
  );
  const branchNodes = $derived(
    (catalog?.nodes ?? []).filter(
      (node) => node.talent === branchId && !node.hidden,
    ),
  );
  const treeWidth = $derived(
    Math.max(700, ...branchNodes.map((node) => node.x * 90 + 110)),
  );
  const treeHeight = $derived(
    Math.max(500, ...branchNodes.map((node) => node.y * 90 + 110)),
  );
  const studied = $derived(
    new Set(selected?.talents.flatMap((branch) => branch.studied) ?? []),
  );
  const talentNames = $derived(
    Object.fromEntries(
      (catalog?.branches ?? []).map((branch) => [branch.id, branch.name]),
    ),
  );
  const pocketItems = $derived(
    (selected?.items ?? []).filter((entry) => {
      const definition = items?.items[entry.id];
      return (
        !definition?.fields.isMainOrgan &&
        !definition?.fields.itemGroupIds.includes("burial_reward") &&
        !entry.equipped
      );
    }),
  );
  const porterCargo = $derived(selected?.cargo?.items ?? []);
  const freeCargoSlots = $derived(
    Math.max(
      0,
      Number(selected?.cargo?.capacity ?? 0) -
        (selected?.cargo?.items.length ?? 0),
    ),
  );

  function zombieName(zombie: ZombieState) {
    return plainText(catalog?.names[zombie.name] || zombie.name);
  }
  function availableTalentSlots(zombie: ZombieState) {
    return Math.max(0, Number(zombie.redSkulls) - zombie.usedTalentSlots);
  }
  function friendlyText(value: string | undefined, fallback = "") {
    const text = plainText(value || "").trim();
    return !text || /^[a-z0-9]+(?:_[a-z0-9]+)+$/i.test(text) ? fallback : text;
  }
  function syncDrafts(zombie: ZombieState | undefined) {
    if (!zombie) return;
    nameDraft = zombieName(zombie);
    pointDraft = { ...zombie.points };
    const setId = zombieAppearanceSetId(zombie.id);
    const set = catalog?.assets.customization.sets.find(
      (entry) => entry.id === setId,
    );
    appearanceDraft = set
      ? {
          set: set.id,
          body: set.bodyIds.includes(zombie.appearance.body)
            ? zombie.appearance.body
            : set.bodyIds[0],
          head: set.headIds.includes(zombie.appearance.head)
            ? zombie.appearance.head
            : set.headIds[0],
          bodyLut: set.bodyLuts.some(
            (entry) => entry.name === zombie.appearance.bodyLut,
          )
            ? zombie.appearance.bodyLut
            : (set.bodyLuts[0]?.name ?? ""),
          headLut: set.headLuts.some(
            (entry) => entry.name === zombie.appearance.headLut,
          )
            ? zombie.appearance.headLut
            : (set.headLuts[0]?.name ?? ""),
        }
      : { ...zombie.appearance };
    if (!catalog?.branches.some((branch) => branch.id === branchId))
      branchId = catalog?.branches[0]?.id ?? "";
  }
  async function refresh(preferred?: number) {
    snapshot = await doc.query<ZombieSnapshot>({ op: "zombies" });
    selectedNode = snapshot.zombies.some((entry) => entry.node === preferred)
      ? preferred
      : snapshot.zombies[0]?.node;
    syncDrafts(snapshot.zombies.find((entry) => entry.node === selectedNode));
  }
  onMount(() => {
    let alive = true;
    Promise.all([gameAssets(), loadZombieCatalog()])
      .then(async ([pack, zombieCatalog]) => {
        const itemCatalog = new ItemCatalog(pack);
        await doc.query({
          op: "inventory_catalog",
          catalog: itemCatalog.rules,
        });
        await doc.query({
          op: "zombie_catalog",
          catalog: zombieCatalog.backend,
        });
        const ui = pack.pack.metadata.catalogs["inventory-ui"] as
          { backgroundSprite?: string } | undefined;
        if (ui?.backgroundSprite)
          slotBackground = await itemCatalog.sprite(ui.backgroundSprite);
        if (!alive) return;
        items = itemCatalog;
        catalog = zombieCatalog;
        await refresh();
        loadedZombieEpoch = doc.zombieEpoch;
        initialized = true;
      })
      .catch((reason) => (loadError = String(reason).replace(/^Error:\s*/, "")))
      .finally(() => (loading = false));
    return () => {
      alive = false;
    };
  });
  $effect(() => {
    const epoch = doc.zombieEpoch;
    if (!initialized || !active || epoch === loadedZombieEpoch) return;
    loadedZombieEpoch = epoch;
    void refresh(untrack(() => selectedNode)).catch(notifyError);
  });
  $effect(() => {
    if (selected) syncDrafts(selected);
  });

  /** Rejections propagate: the item dialog shows them inline, other callers use `notifyError`. */
  async function mutate(
    action: Record<string, unknown> | Record<string, unknown>[],
  ) {
    if (!selected) return;
    const actions = Array.isArray(action) ? action : [action];
    await doc.transact(
      actions.map((entry) => ({
        op: "zombie",
        zombie: selected.node,
        action: entry,
      })),
    );
    loadedZombieEpoch = doc.zombieEpoch;
    await refresh(selected.node);
  }
  function applyPoints() {
    return mutate({
      kind: "set_points",
      red: String(pointDraft.red),
      green: String(pointDraft.green),
      blue: String(pointDraft.blue),
    });
  }
  const put = (node: number | null, item: string) => ({
    kind: "body_inventory",
    out_of_bounds: settings.outOfBoundsEdits,
    action: {
      kind: "put",
      node,
      item,
      count: "1",
      guid: crypto.randomUUID(),
      durability: items?.items[item]?.fields.hasDurability ? "1" : null,
    },
  });
  const remove = (node: number) => ({
    kind: "body_inventory",
    out_of_bounds: settings.outOfBoundsEdits,
    action: { kind: "remove", node },
  });
  function organ(type: string) {
    return selected?.items.find(
      (entry) =>
        items?.items[entry.id]?.fields.isMainOrgan &&
        entry.id.toLowerCase().startsWith(`${type}_`),
    );
  }
  function equipped(slot: string) {
    return selected?.items.find((entry) => entry.equipped === slot);
  }
  function editOrgan(type: string, label: string) {
    const current = organ(type);
    itemDialog = {
      title: `Choose ${label.toLowerCase()}`,
      current: current?.id,
      filter: (item) =>
        !!item.fields.isMainOrgan &&
        !item.fields.isOrganMistake &&
        item.id.toLowerCase().startsWith(`${type}_`),
      confirm: (id) => mutate(put(current?.node ?? null, id)),
      clear: current ? () => mutate(remove(current.node)) : undefined,
      mode: "organ",
    };
  }
  function editPocket(index: number) {
    const current = pocketItems[index];
    itemDialog = {
      title: "Choose body item",
      current: current?.id,
      filter: (item) =>
        !item.fields.isMainOrgan &&
        !item.fields.isBag &&
        item.fields.itemSize === "Small" &&
        !item.fields.itemGroupIds.includes("burial_reward") &&
        !["Collar", "BodyArmor"].includes(item.fields.type) &&
        !item.fields.talentIds?.length,
      confirm: (id) => mutate(put(current?.node ?? null, id)),
      clear: current ? () => mutate(remove(current.node)) : undefined,
      mode: "body",
    };
  }
  function editCargo(item?: ZombieBodyItem) {
    itemDialog = {
      title: "Choose porter cargo",
      current: item?.id,
      filter: (candidate) =>
        candidate.id !== "fake_porter_slot_filler" &&
        !candidate.fields.isBag &&
        candidate.fields.itemSize === "Small",
      confirm: (id, count = item?.count ?? "1") =>
        mutate({
          kind: "cargo_inventory",
          out_of_bounds: settings.outOfBoundsEdits,
          action: {
            kind: "put",
            node: item?.node ?? null,
            item: id,
            count,
            guid: crypto.randomUUID(),
            durability: items?.items[id]?.fields.hasDurability ? "1" : null,
          },
        }),
      clear: item
        ? () =>
            mutate({
              kind: "cargo_inventory",
              out_of_bounds: settings.outOfBoundsEdits,
              action: { kind: "remove", node: item.node },
            })
        : undefined,
      mode: "cargo",
      currentCount: item?.count ?? "1",
    };
  }
  function editEquipment(slot: "collar" | "armor" | "hand", label: string) {
    const current = equipped(slot);
    const matches = (item: ItemDefinition) =>
      slot === "collar"
        ? item.fields.type === "Collar"
        : slot === "armor"
          ? item.fields.type === "BodyArmor"
          : !!item.fields.talentIds?.length &&
            !["Collar", "BodyArmor"].includes(item.fields.type);
    itemDialog = {
      title: `Choose ${label.toLowerCase()}`,
      current: current?.id,
      filter: matches,
      confirm: async (id) =>
        mutate([
          put(current?.node ?? null, id),
          { kind: "equip", slot, node: null, item: id },
        ]),
      clear: current
        ? () => mutate({ kind: "equip", slot, node: null, item: null })
        : undefined,
      mode: slot === "hand" ? "tool" : slot === "armor" ? "armor" : "collar",
      preferredTalent: slot === "hand" ? preferredToolTalent() : undefined,
    };
  }

  function preferredToolTalent() {
    const currentTool = equipped("hand");
    const equippedTalent = currentTool
      ? items?.items[currentTool.id]?.fields.talentIds?.[0]
      : undefined;
    if (equippedTalent) return equippedTalent;
    const ranked = [...(selected?.talents ?? [])].sort(
      (a, b) =>
        b.studied.length - a.studied.length ||
        Number(b.value) - Number(a.value),
    );
    const best = ranked[0];
    return best && (best.studied.length || Number(best.value) > 0)
      ? best.id
      : undefined;
  }

  function perfectBodyOperations(
    zombie: ZombieState,
    itemCatalog: ItemCatalog,
  ) {
    const best = bestBodyItems(itemCatalog);
    const operations: Record<string, unknown>[] = [];
    for (const entry of zombie.items.filter((item) => {
      const definition = itemCatalog.items[item.id];
      return (
        !definition?.fields.isMainOrgan &&
        !definition?.fields.itemGroupIds.includes("burial_reward") &&
        !item.equipped
      );
    }))
      operations.push(remove(entry.node));
    for (const definition of best.organs) {
      const type = definition.id.split("_")[0].toLowerCase();
      operations.push(put(organ(type)?.node ?? null, definition.id));
    }
    if (best.embalming)
      for (let index = 0; index < 6; index++)
        operations.push(put(null, best.embalming.id));
    if (best.collar) {
      const current =
        equipped("collar") ??
        zombie.items.find(
          (entry) => itemCatalog.items[entry.id]?.fields.type === "Collar",
        );
      operations.push(put(current?.node ?? null, best.collar.id), {
        kind: "equip",
        slot: "collar",
        node: null,
        item: best.collar.id,
      });
    }
    return operations;
  }
  async function perfectBody() {
    if (!selected || !items) return;
    await mutate(perfectBodyOperations(selected, items));
    notify("The strongest available body parts and collar have been applied.");
  }
  function closure(ids: string[], current = new Set(studied)) {
    const byId = new Map((catalog?.nodes ?? []).map((node) => [node.id, node]));
    const visit = (id: string) => {
      if (current.has(id)) return;
      const node = byId.get(id);
      if (!node) return;
      for (const parent of node.lockType === "Any"
        ? node.parents.slice(0, 1)
        : node.parents)
        visit(parent);
      current.add(id);
    };
    for (const id of ids) visit(id);
    return current;
  }
  async function toggleTalent(id: string) {
    const next = new Set(studied);
    if (next.has(id)) {
      const pending = [id];
      while (pending.length) {
        const removed = pending.pop()!;
        next.delete(removed);
        for (const node of catalog?.nodes ?? [])
          if (node.parents.includes(removed) && next.has(node.id))
            pending.push(node.id);
      }
    } else closure([id], next);
    await mutate({
      kind: "set_talents",
      ids: [...next],
      grant_points: !spendPoints,
    });
  }
  async function perfectBranch() {
    if (!selectedBranch || !selected || !items) return;
    const next = closure(branchNodes.map((node) => node.id));
    const best = bestBodyItems(items);
    const maximumRed =
      best.organs.reduce((sum, item) => sum + item.fields.redSkulls, 0) +
      (best.embalming?.fields.redSkulls ?? 0) * 6;
    if (next.size > maximumRed) {
      notify(
        `${selectedBranch.name} needs ${next.size} talent slots, but the strongest available body provides ${maximumRed}.`,
        "error",
      );
      return;
    }
    const operations = perfectBodyOperations(selected, items);
    for (const definition of bestEquipment(items, selectedBranch.id)) {
      const slot = definition.fields.type === "BodyArmor" ? "armor" : "hand";
      if (
        operations.some(
          (entry) => entry.kind === "equip" && entry.slot === slot,
        )
      )
        continue;
      const current = equipped(slot);
      operations.push(put(current?.node ?? null, definition.id), {
        kind: "equip",
        slot,
        node: null,
        item: definition.id,
      });
    }
    operations.push({
      kind: "set_talents",
      ids: [...next],
      grant_points: true,
    });
    await mutate(operations);
    notify(
      `${zombieName(selected)} is now optimized for ${selectedBranch.name}.`,
    );
  }
  function cycle<T>(values: T[], current: T, direction: number) {
    const at = Math.max(0, values.indexOf(current));
    return values[(at + direction + values.length) % values.length];
  }
  function nodeStyle(x: number, y: number) {
    return `left:${Math.max(0, x) * 90 + 16}px;top:${Math.max(0, y) * 90 + 16}px`;
  }
</script>

<section class="zombie-editor">
  <header class="section-heading">
    <div>
      <h2>Zombies</h2>
      <p>Customize each worker’s body, equipment, talents and appearance.</p>
    </div>
    {#if selected}<button
        class="primary"
        onclick={() => void perfectBody().catch(notifyError)}
        ><MagicWand />Optimize body</button
      >{/if}
  </header>
  {#if loading}<p class="empty">
      Loading zombies…
    </p>{:else if loadError}<p class="error-banner" role="alert">
      {loadError}
    </p>{:else if !snapshot?.zombies.length}<p class="empty">
      No zombies were found in this save.
    </p>
  {:else if catalog && items}
    <div class="workspace">
      <aside class="zombie-list" aria-label="Zombies">
        {#each snapshot.zombies as zombie}<button
            class:active={zombie.node === selectedNode}
            onclick={() => {
              selectedNode = zombie.node;
              syncDrafts(zombie);
            }}
          >
            <span class="list-portrait"
              ><ZombiePortrait
                appearance={zombie.appearance}
                alt={`${zombieName(zombie)} portrait`}
              /></span
            ><span class="list-copy"
              ><strong>{zombieName(zombie)}</strong><span>{zombie.role}</span
              ><small
                ><SkullValue
                  compact
                  red={zombie.redSkulls}
                  white={zombie.whiteSkulls}
                /> · {availableTalentSlots(zombie)} talent {availableTalentSlots(
                  zombie,
                ) === 1
                  ? "slot"
                  : "slots"} available</small
              ></span
            >
          </button>{/each}
      </aside>
      {#if selected}<article class="detail">
          <div class="identity">
            <div class="portrait">
              <ZombiePortrait
                appearance={selected.appearance}
                alt={`${zombieName(selected)} portrait`}
              />
            </div>
            <div>
              <h3>{zombieName(selected)}</h3>
              <p>
                {selected.role} · {selected.world || "Not currently assigned"}
              </p>
              <SkullValue
                red={selected.redSkulls}
                white={selected.whiteSkulls}
              />
            </div>
          </div>
          <nav class="detail-tabs" aria-label="Zombie details">
            {#each ["Overview", "Body & equipment", "Talents", "Appearance"] as tab}<button
                class:active={detailTab === tab}
                onclick={() => (detailTab = tab)}>{tab}</button
              >{/each}
          </nav>

          <div hidden={detailTab !== "Overview"} class="panel-grid">
            <section class="panel">
              <h4>Name</h4>
              <label
                >Zombie name<input
                  bind:value={nameDraft}
                  maxlength="80"
                /></label
              ><button
                disabled={!nameDraft.trim() ||
                  nameDraft === zombieName(selected)}
                onclick={() =>
                  void mutate({ kind: "rename", name: nameDraft }).catch(
                    notifyError,
                  )}>Apply name</button
              >
            </section>
            <section class="panel">
              <h4>Technology points</h4>
              <div class="point-fields">
                {#each [["red", "tech_red"], ["green", "tech_green"], ["blue", "tech_blue"]] as point}<label
                    ><span><GameIcon name={point[1]} />{point[0]}</span><input
                      type="number"
                      min="0"
                      max="2147483647"
                      step="1"
                      required
                      value={pointDraft[point[0] as keyof typeof pointDraft]}
                      oninput={(event) => {
                        pointDraft[point[0] as keyof typeof pointDraft] =
                          event.currentTarget.value;
                      }}
                    /></label
                  >{/each}
              </div>
              <div class="actions">
                <button
                  onclick={() =>
                    (pointDraft = { red: "9999", green: "9999", blue: "9999" })}
                  >Set all to 9,999</button
                ><button
                  class="primary"
                  onclick={() => void applyPoints().catch(notifyError)}
                  >Apply points</button
                >
              </div>
            </section>
            <section class="panel">
              <h4>Assignment</h4>
              <dl>
                <dt>Role</dt>
                <dd>{selected.role}</dd>
                <dt>Location</dt>
                <dd>{selected.world || "Not assigned"}</dd>
                <dt>Workstation</dt>
                <dd>{selected.attachedGuid ? "Assigned" : "None"}</dd>
                <dt>Free talent slots</dt>
                <dd>
                  {availableTalentSlots(selected)}
                </dd>
              </dl>
            </section>
            <section class="panel">
              <h4>Quick improvement</h4>
              <button
                class="primary wide"
                onclick={() => void perfectBody().catch(notifyError)}
                ><MagicWand />Optimize body</button
              >
              <p class="hint">
                Applies the strongest available organs, body treatments and
                collar.
              </p>
            </section>
            <section class="panel advanced">
              <h4>Advanced save data</h4>
              <dl>
                <dt>Zombie record</dt>
                <dd>
                  {selected.node}
                  {#if oninspect}<button
                      onclick={() => oninspect(selected.node)}
                      >Open in Save Inspector</button
                    >{/if}
                </dd>
                <dt>Body record</dt>
                <dd>
                  {selected.body}
                  {#if oninspect}<button
                      onclick={() => oninspect(selected.body)}
                      >Open in Save Inspector</button
                    >{/if}
                </dd>
                <dt>Unique ID</dt>
                <dd>{selected.guid}</dd>
              </dl>
            </section>
          </div>

          <div hidden={detailTab !== "Body & equipment"} class="body-panel">
            <div class="body-summary">
              <SkullValue
                red={selected.redSkulls}
                white={selected.whiteSkulls}
              /><span>{pocketItems.length}/6 body pockets used</span>
            </div>
            <div class="body-layout">
              <section>
                <h4>Organs</h4>
                <div class="slot-grid">
                  {#each organTypes as [type, label]}<ZombieSlot
                      catalog={loadedItems}
                      item={organ(type)}
                      {label}
                      background={slotBackground}
                      onclick={() => editOrgan(type, label)}
                      onremove={organ(type)
                        ? () =>
                            void mutate(remove(organ(type)!.node)).catch(
                              notifyError,
                            )
                        : undefined}
                    />{/each}
                </div>
                <h4>Body pockets</h4>
                <div class="slot-grid">
                  {#each Array(6) as _, index}<ZombieSlot
                      catalog={loadedItems}
                      item={pocketItems[index]}
                      label={`Slot ${index + 1}`}
                      background={slotBackground}
                      onclick={() => editPocket(index)}
                      onremove={pocketItems[index]
                        ? () =>
                            void mutate(remove(pocketItems[index].node)).catch(
                              notifyError,
                            )
                        : undefined}
                    />{/each}
                </div>
              </section>
              <section class="equipment">
                <h4>Equipment</h4>
                {#each equipmentSlots as [slot, label]}<ZombieSlot
                    catalog={loadedItems}
                    item={equipped(slot)}
                    {label}
                    background={slotBackground}
                    onclick={() => editEquipment(slot, label)}
                    onremove={equipped(slot)
                      ? () =>
                          void mutate({
                            kind: "equip",
                            slot,
                            node: null,
                            item: null,
                          }).catch(notifyError)
                      : undefined}
                  />{/each}
              </section>
            </div>
            <section class="work-storage">
              <h4>Work storage</h4>
              {#if selected.role === "Porter" && selected.cargo}<div
                  class="storage-heading"
                >
                  <div>
                    <strong>Current cargo</strong>
                    <p>
                      Items currently being transported. Large cargo and its
                      reserved slot are shown but remain game-managed.
                    </p>
                  </div>
                  <span
                    >{selected.cargo.items.length}/{selected.cargo.capacity}
                    slots used</span
                  >
                </div>
                <div class="cargo-grid">
                  {#each porterCargo as item, index}{@const reserved =
                      item.id === "fake_porter_slot_filler"}{@const large =
                      items.items[item.id]?.fields.itemSize ===
                      "Big"}<ZombieSlot
                      catalog={loadedItems}
                      item={reserved ? undefined : item}
                      label={reserved
                        ? "Reserved for large cargo"
                        : `Cargo ${index + 1}`}
                      background={slotBackground}
                      disabled={large || reserved}
                      onclick={() => editCargo(item)}
                      onremove={large || reserved
                        ? undefined
                        : () =>
                            void mutate({
                              kind: "cargo_inventory",
                              out_of_bounds: settings.outOfBoundsEdits,
                              action: { kind: "remove", node: item.node },
                            }).catch(notifyError)}
                    />{/each}
                  {#each Array(freeCargoSlots) as _, index}<ZombieSlot
                      catalog={loadedItems}
                      label={`Empty cargo slot ${index + 1}`}
                      background={slotBackground}
                      onclick={() => editCargo()}
                    />{/each}
                </div>
              {:else if selected.carriedItem}<div class="carried-item">
                  <div>
                    <strong>{selected.carriedItem.label}</strong>
                    <p>
                      This is live work state. The game will deposit or replace
                      it as the current task progresses.
                    </p>
                  </div>
                  <ZombieSlot
                    catalog={loadedItems}
                    item={selected.carriedItem.item}
                    label="In transit"
                    background={slotBackground}
                    disabled
                    onclick={() => {}}
                  />
                </div>
              {:else if selected.role === "Gardener"}<p>
                  Gardeners use their body pockets while working. Edit them
                  above.
                </p>
              {:else if selected.attachedGuid}<p>
                  This worker uses the inventory of its assigned workstation.
                  Edit that container from the Inventories section.
                </p>
              {:else}<p>This worker has no separate work inventory.</p>{/if}
            </section>
          </div>

          <div hidden={detailTab !== "Talents"} class="talent-layout">
            <nav class="branch-tabs">
              {#each catalog.branches as branch}<button
                  class:active={branch.id === branchId}
                  onclick={() => (branchId = branch.id)}
                  ><ProgressionIcon
                    name={branch.fontIcon}
                    kind="font"
                    label={branch.name}
                  /><span>{branch.name}</span></button
                >{/each}
            </nav>
            {#if selectedBranch}<div class="talent-heading">
                <div>
                  <h4>{selectedBranch.name}</h4>
                  {#if friendlyText(selectedBranch.description)}<p>
                      {friendlyText(selectedBranch.description)}
                    </p>{/if}
                </div>
                <label class="spend-points"
                  ><input type="checkbox" bind:checked={spendPoints} /><span
                    ><strong>Use technology points</strong><small
                      >Deduct costs when unlocking individual talents.</small
                    ></span
                  ></label
                ><button
                  class="primary"
                  onclick={() => void perfectBranch().catch(notifyError)}
                  ><MagicWand />Optimize for {selectedBranch.name}</button
                >
              </div>{/if}
            <div class="talent-tree">
              <div
                class="talent-canvas"
                style:width={`${treeWidth}px`}
                style:height={`${treeHeight}px`}
              >
                <svg
                  viewBox={`0 0 ${treeWidth} ${treeHeight}`}
                  aria-hidden="true"
                  >{#each branchNodes as node}{#each node.parents as parentId}{@const parent =
                        branchNodes.find(
                          (entry) => entry.id === parentId,
                        )}{#if parent}<line
                          class:unlocked={studied.has(node.id) &&
                            studied.has(parent.id)}
                          x1={parent.x * 90 + 48}
                          y1={parent.y * 90 + 48}
                          x2={node.x * 90 + 48}
                          y2={node.y * 90 + 48}
                        />{/if}{/each}{/each}</svg
                >{#each branchNodes as node}<button
                    class="talent-node"
                    class:unlocked={studied.has(node.id)}
                    style={nodeStyle(node.x, node.y)}
                    title={friendlyText(node.description, node.name)}
                    disabled={node.availableAtStart}
                    onclick={() =>
                      void toggleTalent(node.id).catch(notifyError)}
                    ><ProgressionIcon
                      name={node.sprite}
                      label={node.name}
                      kind="sprite"
                      recolor={false}
                    /><span
                      >{node.talentValue > 0
                        ? `+${node.talentValue}`
                        : ""}</span
                    ></button
                  >{/each}
              </div>
            </div>
          </div>

          <div hidden={detailTab !== "Appearance"} class="appearance-panel">
            <div class="large-portrait">
              <ZombiePortrait
                appearance={appearanceDraft}
                alt="Zombie appearance preview"
                includeStone={false}
                crop={false}
              />
            </div>
            <div class="appearance-controls">
              {#if hasUnsupportedAppearance}<p class="appearance-warning">
                  This save contains a body style that this worker cannot load
                  safely. Applying this appearance will restore a valid body
                  while keeping any compatible head settings.
                </p>{/if}
              {#if (appearanceSet?.bodyIds.length ?? 0) > 1}<div
                  class="appearance-row"
                >
                  <strong>Body</strong><button
                    aria-label="Previous body"
                    onclick={() =>
                      (appearanceDraft.body = cycle(
                        appearanceSet!.bodyIds,
                        appearanceDraft.body,
                        -1,
                      ))}><CaretLeft /></button
                  ><span
                    >{appearanceSet!.bodyIds.indexOf(appearanceDraft.body) + 1}
                    / {appearanceSet!.bodyIds.length}</span
                  ><button
                    aria-label="Next body"
                    onclick={() =>
                      (appearanceDraft.body = cycle(
                        appearanceSet!.bodyIds,
                        appearanceDraft.body,
                        1,
                      ))}><CaretRight /></button
                  >
                </div>{/if}
              {#if (appearanceSet?.headIds.length ?? 0) > 1}<div
                  class="appearance-row"
                >
                  <strong>Head</strong><button
                    aria-label="Previous head"
                    onclick={() =>
                      (appearanceDraft.head = cycle(
                        appearanceSet!.headIds,
                        appearanceDraft.head,
                        -1,
                      ))}><CaretLeft /></button
                  ><span
                    >{appearanceSet!.headIds.indexOf(appearanceDraft.head) + 1}
                    / {appearanceSet!.headIds.length}</span
                  ><button
                    aria-label="Next head"
                    onclick={() =>
                      (appearanceDraft.head = cycle(
                        appearanceSet!.headIds,
                        appearanceDraft.head,
                        1,
                      ))}><CaretRight /></button
                  >
                </div>{/if}
              {#if (appearanceSet?.bodyLuts.length ?? 0) > 1}<div
                  class="appearance-row"
                >
                  <strong>Body palette</strong><button
                    aria-label="Previous body palette"
                    onclick={() =>
                      (appearanceDraft.bodyLut = cycle(
                        appearanceSet!.bodyLuts.map((entry) => entry.name),
                        appearanceDraft.bodyLut,
                        -1,
                      ))}><CaretLeft /></button
                  ><span
                    >{appearanceSet!.bodyLuts.findIndex(
                      (entry) => entry.name === appearanceDraft.bodyLut,
                    ) + 1} / {appearanceSet!.bodyLuts.length}</span
                  ><button
                    aria-label="Next body palette"
                    onclick={() =>
                      (appearanceDraft.bodyLut = cycle(
                        appearanceSet!.bodyLuts.map((entry) => entry.name),
                        appearanceDraft.bodyLut,
                        1,
                      ))}><CaretRight /></button
                  >
                </div>{/if}
              {#if (appearanceSet?.headLuts.length ?? 0) > 1}<div
                  class="appearance-row"
                >
                  <strong>Head palette</strong><button
                    aria-label="Previous head palette"
                    onclick={() =>
                      (appearanceDraft.headLut = cycle(
                        appearanceSet!.headLuts.map((entry) => entry.name),
                        appearanceDraft.headLut,
                        -1,
                      ))}><CaretLeft /></button
                  ><span
                    >{appearanceSet!.headLuts.findIndex(
                      (entry) => entry.name === appearanceDraft.headLut,
                    ) + 1} / {appearanceSet!.headLuts.length}</span
                  ><button
                    aria-label="Next head palette"
                    onclick={() =>
                      (appearanceDraft.headLut = cycle(
                        appearanceSet!.headLuts.map((entry) => entry.name),
                        appearanceDraft.headLut,
                        1,
                      ))}><CaretRight /></button
                  >
                </div>{/if}
              <button
                class="primary"
                onclick={() =>
                  void mutate({
                    kind: "set_appearance",
                    set: appearanceDraft.set,
                    body: Number(appearanceDraft.body),
                    head: Number(appearanceDraft.head),
                    body_lut: appearanceDraft.bodyLut,
                    head_lut: appearanceDraft.headLut,
                  }).catch(notifyError)}>Apply appearance</button
              >
            </div>
          </div>
        </article>{/if}
    </div>
  {/if}
</section>

{#if itemDialog && items}<ZombieItemDialog
    catalog={loadedItems}
    title={itemDialog.title}
    current={itemDialog.current}
    filter={itemDialog.filter}
    onconfirm={itemDialog.confirm}
    onclear={itemDialog.clear}
    mode={itemDialog.mode}
    preferredTalent={itemDialog.preferredTalent}
    currentCount={itemDialog.currentCount}
    outOfBounds={settings.outOfBoundsEdits}
    {talentNames}
    onclose={() => (itemDialog = undefined)}
  />{/if}

<style>
  .zombie-editor {
    display: grid;
    gap: 12px;
  }
  .section-heading {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 16px;
  }
  .section-heading h2,
  .detail h3,
  .panel h4,
  .body-panel h4,
  .talent-heading h4 {
    margin: 0;
  }
  .section-heading p,
  .identity p,
  .talent-heading p {
    margin: 4px 0;
    color: #aeb4be;
  }
  .section-heading button,
  .talent-heading button {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .workspace {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr);
    gap: 12px;
  }
  .zombie-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-height: 780px;
    overflow: auto;
    padding: 4px;
    background: var(--deep);
    border: 1px solid var(--border);
  }
  .zombie-list > button {
    display: grid;
    grid-template-columns: 58px minmax(0, 1fr);
    align-items: center;
    gap: 10px;
    padding: 7px;
    text-align: left;
    background: #292c35;
  }
  .zombie-list > button.active {
    color: var(--gold);
    background: #393a3d;
    border-color: #817863;
  }
  .list-portrait {
    width: 58px;
    height: 58px;
    overflow: hidden;
    background: #201f23;
    border: 1px solid #53545b;
  }
  .list-copy {
    display: grid;
    min-width: 0;
    gap: 3px;
  }
  .list-copy > span,
  .list-copy small {
    color: #abb1bb;
    font-size: 12px;
  }
  .list-copy strong,
  .list-copy > span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .list-copy small {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .detail {
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .identity {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 16px;
    background: #343039;
    border-bottom: 1px solid #6e604d;
  }
  .portrait {
    width: 112px;
    height: 112px;
    overflow: hidden;
    background: #211f23;
    border: 1px solid #5f5d63;
  }
  .detail-tabs,
  .branch-tabs {
    display: flex;
    gap: 2px;
    padding: 4px;
    overflow: auto;
    background: var(--deep);
  }
  .detail-tabs button.active,
  .branch-tabs button.active {
    color: var(--gold);
    background: #3a3b41;
    border-color: #716c61;
  }
  .panel-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
    padding: 12px;
  }
  .panel {
    display: grid;
    align-content: start;
    gap: 10px;
    padding: 12px;
    background: #2b2e36;
    border: 1px solid #484b54;
  }
  .panel label {
    display: grid;
    gap: 4px;
    color: #b5bac4;
    font-size: 12px;
  }
  .point-fields {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .point-fields label > span {
    display: flex;
    align-items: center;
    gap: 5px;
    text-transform: capitalize;
  }
  .point-fields :global(.game-icon) {
    width: 24px;
    height: 24px;
    flex-basis: 24px;
  }
  .actions,
  .body-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .wide {
    width: 100%;
    justify-content: center;
  }
  .hint {
    margin: 0;
    color: #9fa6b2;
    font-size: 12px;
  }
  .advanced {
    grid-column: 1/-1;
  }
  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 7px 12px;
    margin: 0;
  }
  dt {
    color: #aeb4be;
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .advanced dd button {
    margin-left: 8px;
    padding: 3px 6px;
  }
  .body-panel,
  .talent-layout,
  .appearance-panel {
    padding: 12px;
  }
  .body-summary {
    justify-content: space-between;
    margin-bottom: 14px;
  }
  .body-summary > span {
    padding: 6px 8px;
    background: var(--deep);
    border: 1px solid var(--border);
  }
  .body-layout {
    display: grid;
    grid-template-columns: minmax(340px, 1fr) 120px;
    justify-content: center;
    gap: 32px;
    max-width: 700px;
    margin: auto;
  }
  .body-layout section > h4 {
    margin: 8px 0;
    color: #b9bdc6;
    text-align: center;
  }
  .slot-grid {
    display: grid;
    grid-template-columns: repeat(3, 92px);
    justify-content: center;
    gap: 10px;
  }
  .equipment {
    display: grid;
    align-content: start;
    justify-items: center;
    gap: 10px;
    padding-left: 22px;
    border-left: 1px solid #484b54;
  }
  .work-storage {
    display: grid;
    gap: 12px;
    max-width: 700px;
    margin: 24px auto 0;
    padding-top: 16px;
    border-top: 1px solid #484b54;
  }
  .work-storage > h4,
  .work-storage p {
    margin: 0;
  }
  .work-storage p {
    color: #aeb4be;
  }
  .storage-heading,
  .carried-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .storage-heading > span {
    padding: 6px 8px;
    white-space: nowrap;
    background: var(--deep);
    border: 1px solid var(--border);
  }
  .cargo-grid {
    display: grid;
    grid-template-columns: repeat(4, 92px);
    gap: 10px;
  }
  .branch-tabs button {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 110px;
  }
  .branch-tabs :global(.progression-icon) {
    --icon-size: 32px;
  }
  .talent-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px;
  }
  .spend-points {
    display: flex;
    align-items: center;
    gap: 7px;
    color: #b5bac4;
    font-size: 12px;
  }
  .spend-points span,
  .spend-points small {
    display: block;
  }
  .spend-points small {
    color: #9299a5;
  }
  .talent-tree {
    width: 100%;
    height: 560px;
    overflow: auto;
    background: #24262e;
    border: 1px solid var(--border);
  }
  .talent-canvas {
    position: relative;
    margin-inline: auto;
  }
  .talent-canvas svg {
    position: absolute;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .talent-canvas line {
    stroke: #4a4e59;
    stroke-width: 2;
  }
  .talent-canvas line.unlocked {
    stroke: var(--gold);
    stroke-width: 3;
  }
  .talent-node {
    position: absolute;
    z-index: 1;
    width: 64px;
    height: 64px;
    padding: 5px;
    background: #343743;
    border-color: #5b6070;
  }
  .talent-node.unlocked {
    background: #5b4520;
    border-color: var(--gold);
  }
  .talent-node :global(.progression-icon) {
    --icon-size: 50px;
  }
  .talent-node span {
    position: absolute;
    right: 2px;
    bottom: 0;
    color: #ecd17b;
    font-size: 10px;
  }
  .appearance-panel {
    display: grid;
    grid-template-columns: minmax(260px, 400px) minmax(280px, 420px);
    align-items: center;
    justify-content: center;
    gap: 36px;
  }
  .large-portrait {
    height: 400px;
    overflow: hidden;
    background: #211f23;
    border: 1px solid var(--border);
  }
  .appearance-controls {
    display: grid;
    gap: 12px;
  }
  .appearance-warning {
    margin: 0;
    padding: 9px 12px;
    color: #e3d4b3;
    background: #352d23;
    border: 1px solid #735b38;
  }
  .appearance-row {
    display: grid;
    grid-template-columns: 120px 36px minmax(80px, 1fr) 36px;
    align-items: center;
    gap: 5px;
  }
  .appearance-row > span {
    text-align: center;
    text-transform: capitalize;
  }
  .appearance-row button {
    display: grid;
    padding: 7px;
    place-items: center;
  }
  .appearance-row :global(svg) {
    width: 18px;
  }
  .empty {
    padding: 32px;
    text-align: center;
    color: #aeb4be;
    border: 1px solid var(--border);
  }
  .error-banner {
    margin: 0;
  }
  @media (max-width: 1050px) {
    .workspace {
      grid-template-columns: 1fr;
    }
    .zombie-list {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      max-height: 250px;
    }
    .panel-grid {
      grid-template-columns: 1fr;
    }
    .advanced {
      grid-column: auto;
    }
  }
  @media (max-width: 700px) {
    .section-heading,
    .talent-heading {
      align-items: stretch;
      flex-direction: column;
    }
    .body-layout,
    .appearance-panel {
      grid-template-columns: 1fr;
    }
    .equipment {
      grid-template-columns: repeat(3, 92px);
      justify-content: center;
      padding: 12px 0 0;
      border-top: 1px solid #484b54;
      border-left: 0;
    }
    .equipment h4 {
      grid-column: 1/-1;
    }
    .appearance-row {
      grid-template-columns: 90px 34px minmax(70px, 1fr) 34px;
    }
    .point-fields {
      grid-template-columns: 1fr;
    }
  }
</style>
