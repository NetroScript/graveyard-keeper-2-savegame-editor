<script lang="ts">
  import { tick } from "svelte";
  import type { SaveDocument } from "../document.svelte";
  import type {
    NodeLocation,
    NodeView,
    SearchResult,
    SearchStatus,
  } from "../save-api";
  import MagnifyingGlass from "~icons/ph/magnifying-glass";
  import X from "~icons/ph/x";
  import Question from "~icons/ph/question";
  import SearchHelpDialog from "./SearchHelpDialog.svelte";

  let {
    doc,
    active,
    showResults,
    selected,
    onselect,
    onreveal,
    onactive,
  }: {
    doc: SaveDocument;
    active: boolean;
    showResults: boolean;
    selected: number | null;
    onselect: (node: NodeView) => void | Promise<void>;
    onreveal: (location: NodeLocation, node: NodeView) => void | Promise<void>;
    onactive: (active: boolean) => void;
  } = $props();

  const rowHeight = 72;
  const pageSize = 100;
  let mode = $state<"simple" | "advanced">("simple");
  let simple = $state("");
  let advanced = $state("");
  let path = $state("");
  let name = $state("");
  let type = $state("");
  let ownClass = $state("");
  let owner = $state("");
  let ancestor = $state("");
  let value = $state("");
  let minimum = $state("");
  let maximum = $state("");
  let caseSensitive = $state(false);
  let status = $state<SearchStatus>();
  let error = $state("");
  let generation = 0;
  let pages = $state<Record<number, SearchResult[]>>({});
  let pageOrder: number[] = [];
  let loading = new Set<number>();
  let pageEpoch = 0;
  let scrollTop = $state(0);
  let viewportHeight = $state(420);
  let list = $state<HTMLDivElement>();
  let cursor = $state(-1);
  let lastRevision = -1;
  let helpOpen = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let visibleFirst = $derived(
    Math.max(0, Math.floor(scrollTop / rowHeight) - 10),
  );
  let visibleLast = $derived(
    Math.min(
      status?.discovered ?? 0,
      Math.ceil((scrollTop + viewportHeight) / rowHeight) + 10,
    ),
  );

  function literal(text: string) {
    return `"${text.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
  }
  function pattern(text: string) {
    return /^[^\s|!():<>="]+$/.test(text) ? text : literal(text);
  }
  function simpleQuery() {
    const terms: string[] = [];
    if (simple.trim()) terms.push(literal(simple.trim()));
    for (const [field, input] of [
      ["path", path],
      ["name", name],
      ["class", ownClass],
      ["owner", owner],
      ["ancestor", ancestor],
      ["value", value],
    ])
      if (input.trim()) terms.push(`${field}:${pattern(input.trim())}`);
    const types = type
      .split(",")
      .map((entry) => entry.trim())
      .filter(Boolean);
    if (types.length === 1) terms.push(`type:${pattern(types[0])}`);
    else if (types.length > 1)
      terms.push(`type:(${types.map(pattern).join(" | ")})`);
    if (minimum.trim()) terms.push(`value>=${minimum.trim()}`);
    if (maximum.trim()) terms.push(`value<=${maximum.trim()}`);
    return terms.join(" ");
  }
  function query() {
    return mode === "advanced" ? advanced.trim() : simpleQuery();
  }
  function switchMode(next: "simple" | "advanced") {
    if (next === "advanced" && mode === "simple") advanced = simpleQuery();
    mode = next;
  }
  function clear() {
    clearTimeout(debounceTimer);
    generation++;
    const current = status;
    if (current)
      void doc
        .query({
          op: "search_cancel",
          revision: current.revision,
          searchId: current.searchId,
        })
        .catch(() => {});
    simple = advanced = path = name = type = ownClass = owner = ancestor = value =
      minimum = maximum = "";
    status = undefined;
    pages = {};
    cursor = -1;
    onactive(false);
  }
  async function loadPage(
    page: number,
    currentGeneration = generation,
    currentPageEpoch = pageEpoch,
  ) {
    if (!status || pages[page] || loading.has(page)) return;
    if (page * pageSize >= status.discovered) return;
    loading.add(page);
    try {
      const response = await doc.query<{
        results: SearchResult[];
        status: SearchStatus;
      }>({
        op: "search_page",
        revision: status.revision,
        searchId: status.searchId,
        offset: page * pageSize,
        limit: pageSize,
      });
      if (
        currentGeneration !== generation ||
        currentPageEpoch !== pageEpoch
      )
        return;
      pages[page] = response.results;
      pageOrder = [...pageOrder.filter((entry) => entry !== page), page];
      while (pageOrder.length > 10) {
        const remove = pageOrder.shift()!;
        delete pages[remove];
      }
    } catch (reason) {
      if (
        currentGeneration === generation &&
        currentPageEpoch === pageEpoch
      )
        error = String(reason);
    } finally {
      loading.delete(page);
    }
  }
  function loadVisible() {
    if (!status?.discovered) return;
    const first = Math.max(0, Math.floor(scrollTop / rowHeight) - 10);
    const last = Math.min(
      status.discovered - 1,
      Math.ceil((scrollTop + viewportHeight) / rowHeight) + 10,
    );
    for (
      let page = Math.floor(first / pageSize);
      page <= Math.floor(last / pageSize);
      page++
    )
      void loadPage(page);
  }
  async function run() {
    const text = query();
    const currentGeneration = ++generation;
    error = "";
    pages = {};
    pageEpoch++;
    pageOrder = [];
    loading.clear();
    cursor = -1;
    if (!text) {
      status = undefined;
      onactive(false);
      return;
    }
    onactive(true);
    try {
      const started = await doc.query<SearchStatus>({
        op: "search_start",
        revision: doc.summary!.revision,
        query: text,
        caseSensitive,
      });
      if (currentGeneration !== generation) {
        void doc
          .query({
            op: "search_cancel",
            revision: started.revision,
            searchId: started.searchId,
          })
          .catch(() => {});
        return;
      }
      status = started;
      let current = started;
      while (
        currentGeneration === generation &&
        active &&
        current.status === "searching"
      ) {
        const previousCount = current.discovered;
        const nextStatus: SearchStatus = await doc.query<SearchStatus>({
          op: "search_step",
          revision: current.revision,
          searchId: current.searchId,
        });
        if (currentGeneration !== generation) return;
        current = nextStatus;
        status = current;
        if (current.status !== "searching") {
          pages = {};
          pageOrder = [];
          loading.clear();
          pageEpoch++;
        } else if (
          current.discovered !== previousCount &&
          current.discovered > 0
        ) {
          const lastPage = Math.floor((current.discovered - 1) / pageSize);
          if (
            pages[lastPage] &&
            pages[lastPage].length <
              Math.min(pageSize, current.discovered - lastPage * pageSize)
          )
            delete pages[lastPage];
        }
        loadVisible();
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
      }
      loadVisible();
    } catch (reason) {
      if (currentGeneration === generation) {
        error = String(reason).replace(/^Error:\s*/, "");
        status = undefined;
      }
    }
  }
  function resultAt(index: number) {
    return pages[Math.floor(index / pageSize)]?.[index % pageSize];
  }
  async function choose(result: SearchResult) {
    const node = (await doc.nodes([result.node]))[0];
    await onselect(node);
  }
  async function reveal(result: SearchResult) {
    if (!status) return;
    const [node, location] = await Promise.all([
      doc.nodes([result.node]).then((nodes) => nodes[0]),
      doc.query<NodeLocation>({
        op: "node_location",
        revision: doc.summary!.revision,
        node: result.node,
      }),
    ]);
    await onreveal(location, node);
  }
  function keyboard(event: KeyboardEvent) {
    if (!status?.discovered) return;
    if (!["ArrowDown", "ArrowUp", "Home", "End", "Enter"].includes(event.key))
      return;
    event.preventDefault();
    if (event.key === "ArrowDown")
      cursor = Math.min(status.discovered - 1, cursor + 1);
    else if (event.key === "ArrowUp") cursor = Math.max(0, cursor - 1);
    else if (event.key === "Home") cursor = 0;
    else if (event.key === "End") cursor = status.discovered - 1;
    else {
      const result = resultAt(cursor);
      if (result) void choose(result);
      return;
    }
    if (list) {
      if (cursor * rowHeight < list.scrollTop) list.scrollTop = cursor * rowHeight;
      else if ((cursor + 1) * rowHeight > list.scrollTop + list.clientHeight)
        list.scrollTop = (cursor + 1) * rowHeight - list.clientHeight;
      scrollTop = list.scrollTop;
      loadVisible();
    }
  }
  function submitted(event: SubmitEvent) {
    event.preventDefault();
    clearTimeout(debounceTimer);
    void run();
  }

  $effect(() => {
    const fingerprint = [
      mode,
      simple,
      advanced,
      path,
      name,
      type,
      ownClass,
      owner,
      ancestor,
      value,
      minimum,
      maximum,
      caseSensitive,
    ].join("\u0000");
    void fingerprint;
    const revision = doc.summary!.revision;
    generation++;
    if (!active) return;
    if (lastRevision !== -1 && lastRevision !== revision && query()) pages = {};
    lastRevision = revision;
    debounceTimer = setTimeout(() => void run(), 250);
    return () => clearTimeout(debounceTimer);
  });
  $effect(() => {
    if (list) {
      const element = list;
      const observer = new ResizeObserver(() => {
        viewportHeight = element.clientHeight;
        loadVisible();
      });
      observer.observe(element);
      return () => observer.disconnect();
    }
  });
  $effect(() => {
    if (status?.discovered) {
      void tick().then(loadVisible);
    }
  });
</script>

<form class="inspector-search" onsubmit={submitted}>
  <div class="search-mode" role="group" aria-label="Search mode">
    <button
      type="button"
      class:active={mode === "simple"}
      onclick={() => switchMode("simple")}>Simple</button
    ><button
      type="button"
      class:active={mode === "advanced"}
      onclick={() => switchMode("advanced")}>Advanced</button
    >
  </div>
  <label class="search-input">
    <MagnifyingGlass />
    {#if mode === "simple"}<input
        type="search"
        aria-label="Search save structure"
        placeholder="Search names, values, classes and paths"
        bind:value={simple}
      />{:else}<input
        type="search"
        aria-label="Search save structure"
        placeholder="Enter an advanced query"
        bind:value={advanced}
      />{/if}
    {#if query()}<button type="button" aria-label="Clear search" onclick={clear}
        ><X /></button
      >{/if}
  </label>
  {#if mode === "simple"}
    <details class="search-filters">
      <summary>Filters</summary>
      <div>
        <label>Path<input bind:value={path} placeholder="*playerInventory*" /></label>
        <label>Name<input bind:value={name} placeholder="id" /></label>
        <label>Data types<input bind:value={type} placeholder="string, int" /></label>
        <label>Own class<input bind:value={ownClass} placeholder="Item" /></label>
        <label>Nearest class<input bind:value={owner} placeholder="Item" /></label>
        <label>Any ancestor<input bind:value={ancestor} placeholder="Item" /></label>
        <label>Value<input bind:value={value} placeholder="*iron*" /></label>
        <label>Minimum<input bind:value={minimum} inputmode="decimal" /></label>
        <label>Maximum<input bind:value={maximum} inputmode="decimal" /></label>
      </div>
    </details>
  {:else}
    <details class="search-help">
      <summary>Query syntax</summary>
      <p>Spaces mean AND, <code>|</code> means OR, <code>!</code> means NOT, and parentheses group expressions.</p>
      <p>Qualifiers: <code>name:</code>, <code>path:</code>, <code>type:</code>, <code>class:</code>, <code>owner:</code>, <code>ancestor:</code>, <code>value:</code>.</p>
      <p>Use <code>*</code> and <code>?</code> as wildcards, or <code>==</code> for an exact text match. Numeric comparisons use <code>value&gt;=10</code> and similar operators.</p>
      <p>Use <code>child:(…)</code>, <code>descendant:(…)</code> and <code>parent:(…)</code> to match related stored records.</p>
    </details>
  {/if}
  <div class="search-options">
    <label><input type="checkbox" bind:checked={caseSensitive} />Case sensitive</label>
    <span>
      <button type="button" class="search-help-button" onclick={() => (helpOpen = true)}><Question />Syntax help</button>
      <button type="submit">Search</button>
    </span>
  </div>
</form>

{#if helpOpen}
  <SearchHelpDialog
    onclose={() => (helpOpen = false)}
    onuse={(example) => {
      mode = "advanced";
      advanced = example;
      helpOpen = false;
    }}
  />
{/if}

{#if query() && status}
  <div class="search-view-toggle" role="group" aria-label="Inspector search view">
    <button
      type="button"
      class:active={showResults}
      onclick={() => {
        onactive(true);
        void tick().then(loadVisible);
      }}>Results ({status.discovered.toLocaleString()})</button
    ><button
      type="button"
      class:active={!showResults}
      onclick={() => onactive(false)}>Structure</button
    >
  </div>
{/if}

{#if query()}
  <div class="search-results-view" hidden={!showResults}>
    <div class="search-summary" aria-live="polite">
    {#if status}
      <span>{status.discovered.toLocaleString()} matches</span>
      <span>{status.status === "searching" ? "Searching…" : status.status === "incomplete" ? "Incomplete" : "Complete"}</span>
    {:else if !error}<span>Preparing search…</span>{/if}
    </div>
    {#if status?.completionReason}<p class="warning search-warning">{status.completionReason}</p>{/if}
    {#if error}<p class="warning search-warning" role="alert">{error}</p>{/if}
    <div
      class="search-result-list"
      role="listbox"
      aria-label="Search results"
      tabindex="0"
      bind:this={list}
      onkeydown={keyboard}
      onscroll={(event) => {
        scrollTop = event.currentTarget.scrollTop;
        loadVisible();
      }}
    >
      <div class="search-result-space" style:height={`${(status?.discovered ?? 0) * rowHeight}px`}>
        {#each Array.from({ length: Math.max(0, visibleLast - visibleFirst) }, (_, offset) => visibleFirst + offset) as index (index)}
          {@const result = resultAt(index)}
          <div class="search-result-slot" style:transform={`translateY(${index * rowHeight}px)`}>
            {#if result}
              <button
                type="button"
                role="option"
                aria-selected={selected === result.node || cursor === index}
                class:selected={selected === result.node || cursor === index}
                onclick={() => {
                  cursor = index;
                  void choose(result);
                }}
              >
                <span><b>{result.name}</b><em>{result.typeName?.split(",")[0] ?? result.kind}</em><i>{result.matchField} match</i></span>
                <small>{result.value ?? result.path}</small>
                <small class="result-path">{result.path}</small>
              </button>
              <button type="button" class="reveal-result" onclick={() => void reveal(result)}>Reveal</button>
            {:else}<span class="result-loading">Loading…</span>{/if}
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}
