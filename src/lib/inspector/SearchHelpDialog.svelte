<script lang="ts">
  import { onMount } from "svelte";

  let {
    onclose,
    onuse,
  }: {
    onclose: () => void;
    onuse: (query: string) => void;
  } = $props();

  let dialog: HTMLDialogElement;
  const examples = [
    {
      name: "Inventory item string IDs",
      query:
        "path:*playerInventory* ancestor:Item type:string name:id value:*iron*",
      explanation:
        "Find string fields named id below an Item anywhere on a logical path containing playerInventory, then require iron in the value.",
    },
    {
      name: "Numeric range",
      query: "type:(int | float) value>=10 value<=100",
      explanation:
        "Find integer or floating-point records whose numeric value is between 10 and 100, including both endpoints.",
    },
    {
      name: "Alternative item values outside equipment",
      query: "(value:*iron* | value:*copper*) !path:*equipment*",
      explanation:
        "Match either material name, then exclude records reached through an equipment path.",
    },
    {
      name: "Exact field with alternative values",
      query: "name==worldId value==(Prison | RuinedTemple)",
      explanation:
        "Find fields named exactly worldId whose complete value is either Prison or RuinedTemple. The case-sensitive option also applies to exact matches.",
    },
    {
      name: "Object with a direct world field",
      query: "child:(name==worldId value==(Prison | RuinedTemple))",
      explanation:
        "Return each object or container that has a direct child field named worldId whose value is exactly Prison or RuinedTemple.",
    },
    {
      name: "Object containing a nested world field",
      query: "descendant:(name==worldId value==(Prison | RuinedTemple))",
      explanation:
        "Return containers with that worldId field anywhere below them. Add a target filter such as class:WgoData when only one kind of containing object should be returned.",
    },
    {
      name: "Elements below PalaceSewer",
      query: "parent:(child:(name==id value==PalaceSewer)) !name==id",
      explanation:
        "Return direct children of a parent that identifies itself through a direct id field equal to PalaceSewer, excluding the id field itself.",
    },
    {
      name: "Fields directly owned by Item",
      query: "owner:Item type:number value>0",
      explanation:
        "Find positive numeric fields whose nearest containing typed object is Item. Nested typed objects inside an Item do not match this owner filter.",
    },
    {
      name: "Anything nested below an Item",
      query: "ancestor:Item name:id",
      explanation:
        "Find id fields at any depth below an Item, including fields inside other typed objects nested within that Item.",
    },
    {
      name: "Literal operator characters",
      query: 'value:"price >= 10 | unavailable"',
      explanation:
        "Quotes make spaces and operator characters literal. Wildcards inside quotes are literal too.",
    },
  ];

  onMount(() => {
    dialog.showModal();
    return () => dialog.close();
  });
</script>

<dialog
  bind:this={dialog}
  class="panel modal search-help-dialog"
  aria-labelledby="search-help-title"
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
>
  <h2 id="search-help-title" class="strip">Inspector search syntax</h2>
  <div class="help-content">
    <section>
      <h3>Simple search</h3>
      <p>
        Simple search checks a record's value, name, class, wire type and
        logical path. Results rank direct value matches first, followed by
        names, classes, types and paths. Each result states which field matched.
      </p>
      <p>
        Substring matching is literal, so <code>iron</code> also occurs inside
        <code>environmentData</code>. Use <code>value:*iron*</code> in Advanced mode,
        or the Value filter in Simple mode, when only stored values should match.
      </p>
    </section>

    <section>
      <h3>Operators and text</h3>
      <table>
        <thead><tr><th>Syntax</th><th>Meaning</th></tr></thead>
        <tbody>
          <tr
            ><td><code>space</code></td><td
              >AND. Every adjacent expression must match.</td
            ></tr
          >
          <tr
            ><td><code>|</code></td><td>OR. Either expression may match.</td
            ></tr
          >
          <tr
            ><td><code>!</code></td><td>NOT. Excludes matching records.</td></tr
          >
          <tr
            ><td><code>( )</code></td><td
              >Groups expressions. Grouping is evaluated before NOT, AND and OR.</td
            ></tr
          >
          <tr
            ><td><code>"text"</code></td><td
              >Treats spaces, operators and wildcards as literal text. Use <code
                >\"</code
              >
              and <code>\\</code> inside quotes.</td
            ></tr
          >
          <tr
            ><td><code>*</code></td><td
              >Matches zero or more characters. A wildcard pattern must match
              the whole selected field.</td
            ></tr
          >
          <tr><td><code>?</code></td><td>Matches exactly one character.</td></tr
          >
          <tr
            ><td><code>==</code></td><td
              >Matches the entire selected text field. It can qualify a group of
              alternatives.</td
            ></tr
          >
        </tbody>
      </table>
    </section>

    <section>
      <h3>Qualifiers</h3>
      <dl>
        <dt><code>name:</code></dt>
        <dd>The record's field or entry name.</dd>
        <dt><code>value:</code></dt>
        <dd>
          The displayed scalar value. This is textual matching, even for <code
            >value:10</code
          >.
        </dd>
        <dt><code>type:</code></dt>
        <dd>
          The wire data type, such as <code>string</code>, <code>i32</code> or
          <code>f32</code>. Aliases are <code>int</code>, <code>float</code> and
          <code>number</code>.
        </dd>
        <dt><code>class:</code></dt>
        <dd>
          The record's own serialized class. A string field normally has no own
          class.
        </dd>
        <dt><code>owner:</code></dt>
        <dd>
          The nearest typed object containing the record, excluding the record
          itself.
        </dd>
        <dt><code>ancestor:</code></dt>
        <dd>
          Any typed object containing the record at any depth, excluding the
          record itself.
        </dd>
        <dt><code>path:</code></dt>
        <dd>
          The logical route to the record. It includes field names, array
          positions and followed internal references.
        </dd>
      </dl>
      <p>
        Class names may be short (<code>Item</code>), namespace-qualified (<code
          >Game.Items.Item</code
        >) or fully serialized with an assembly name. Search uses actual
        serialized identities and does not infer inheritance.
      </p>
    </section>

    <section>
      <h3>Numeric comparisons</h3>
      <p>
        Use <code>value=10</code>, <code>value&gt;10</code>,
        <code>value&gt;=10</code>, <code>value&lt;100</code> and
        <code>value&lt;=100</code>. Negative numbers, decimals and scientific
        notation are accepted. Multiple comparisons form a range. Integer
        comparisons remain exact for 64-bit values. A single <code>=</code> is
        numeric equality; use <code>==</code> for exact text.
      </p>
    </section>

    <section>
      <h3>Structure</h3>
      <dl>
        <dt><code>child:(…)</code></dt>
        <dd>
          The result has at least one direct stored child that matches the
          enclosed expression.
        </dd>
        <dt><code>descendant:(…)</code></dt>
        <dd>
          The result has a matching stored descendant at any depth. Direct
          children are included.
        </dd>
        <dt><code>parent:(…)</code></dt>
        <dd>
          The result's direct stored parent matches the enclosed expression.
        </dd>
      </dl>
      <p>
        Structural predicates change which record is returned: the enclosed
        expression identifies a related record while the outer expression
        filters the result itself. They may contain <code>name</code>,
        <code>type</code>, <code>class</code>, <code>value</code>, numeric
        comparisons and nested structural predicates. Route-relative qualifiers
        such as <code>path</code>, <code>owner</code> and
        <code>ancestor</code> belong outside them.
      </p>
    </section>

    <section>
      <h3>Examples</h3>
      <div class="examples">
        {#each examples as example}
          <article class="search-example">
            <h4>{example.name}</h4>
            <code>{example.query}</code>
            <p>{example.explanation}</p>
            <button type="button" onclick={() => onuse(example.query)}
              >Use query</button
            >
          </article>
        {/each}
      </div>
    </section>

    <section>
      <h3>Reference paths and limits</h3>
      <p>
        Search follows internal object references and evaluates all conditions
        on the same logical route. A record reachable by several routes is shown
        once, using its first matching route. Cycles stop before revisiting an
        object. If a depth or traversal limit is reached, the result state says
        Incomplete and explains why.
      </p>
    </section>
  </div>
  <div class="dialog-actions">
    <button type="button" class="primary" onclick={onclose}>Close</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(860px, calc(100vw - 32px));
    max-height: calc(100dvh - 32px);
    overflow: hidden;
    padding: 0;
    color: var(--cream);
    background: #272832;
    border: 1px solid #656771;
  }
  .help-content {
    max-height: calc(100dvh - 140px);
    overflow: auto;
    padding: 20px 24px;
  }
  section + section {
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid #444750;
  }
  h3,
  h4 {
    margin: 0 0 8px;
    color: #d9cdb7;
  }
  h3 {
    font-size: 16px;
  }
  h4 {
    font-size: 13px;
  }
  p {
    margin: 8px 0;
    color: #b4afa5;
    line-height: 1.5;
  }
  code {
    color: #dec778;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    overflow-wrap: anywhere;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  th,
  td {
    padding: 8px;
    border: 1px solid #4d5059;
    text-align: left;
    vertical-align: top;
  }
  th {
    color: #d4c8b3;
    background: #30323a;
  }
  dl {
    grid-template-columns: minmax(90px, auto) 1fr;
    gap: 10px 16px;
    font-size: 13px;
  }
  dd {
    color: #b4afa5;
  }
  .examples {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .search-example {
    display: grid;
    align-content: start;
    gap: 7px;
    padding: 12px;
    border: 1px solid #4d5059;
    background: #22242c;
  }
  .search-example p {
    margin: 0;
    font-size: 12px;
  }
  .search-example button {
    justify-self: start;
    margin-top: 3px;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    padding: 12px 24px;
    border-top: 1px solid #555760;
    background: #22242b;
  }
  @media (max-width: 700px) {
    .examples {
      grid-template-columns: 1fr;
    }
    .help-content {
      padding: 16px;
    }
  }
</style>
