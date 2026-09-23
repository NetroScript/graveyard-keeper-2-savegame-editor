<script lang="ts">
  import { desktop, type Settings } from "../document.svelte";
  let {
    settings,
    onsettings,
  }: { settings: Settings; onsettings: (s: Settings) => Promise<void> } =
    $props();
  let message = $state("");
  async function update(s: Settings) {
    try {
      await onsettings(s);
      message = "Settings saved";
    } catch (e) {
      message = String(e);
    }
  }
</script>

<header class="page-heading">
  <h1>Settings</h1>
  <p>Configure the editor.</p>
</header>
<section class="panel settings-panel">
  <h2 class="strip">Interface</h2>
  <div class="panel-body">
    <label class="field"
      >Interface scale<select
        value={settings.interfaceScale}
        onchange={(e) =>
          update({
            ...settings,
            interfaceScale: Number(e.currentTarget.value),
          })}
        >{#each [0.75, 0.85, 1, 1.15, 1.25, 1.5] as scale}<option value={scale}
            >{Math.round(scale * 100)}%</option
          >{/each}</select
      ></label
    >
  </div>
</section>
<section class="panel settings-panel">
  <h2 class="strip">Editing</h2>
  <div class="panel-body">
    <label
      ><input
        type="checkbox"
        checked={settings.outOfBoundsEdits}
        onchange={(e) =>
          update({ ...settings, outOfBoundsEdits: e.currentTarget.checked })}
      /> Allow out of bounds edits</label
    >
    <p class="hint">
      Allow inventory capacity changes and item amounts above the normal stack
      size.
    </p>
  </div>
</section>
{#if desktop}<section class="panel settings-panel">
    <h2 class="strip">Save protection</h2>
    <div class="panel-body">
      <label class="field"
        >Backups per save<input
          type="number"
          min="0"
          max="50"
          value={settings.backupRetention}
          onchange={(e) =>
            update({
              ...settings,
              backupRetention: Number(e.currentTarget.value),
            })}
        /></label
      >
      <p class="hint">
        Each editor backup is a compressed ZIP containing the previous .dat and
        .info together, separately from game backups. Set to 0 to disable them.
      </p>
      <h3>Custom save directories</h3>
      {#each settings.customDirectories as path}<div class="copy-path">
          <code>{path}</code><button
            onclick={() =>
              update({
                ...settings,
                customDirectories: settings.customDirectories.filter(
                  (p) => p !== path,
                ),
              })}>Remove</button
          >
        </div>{:else}<p class="hint">
          Use Choose Folder on Load Saves to add a directory.
        </p>{/each}
    </div>
  </section>{:else}<p class="hint">
    Saving downloads a copy. Automatic backups and folder discovery are
    unavailable.
  </p>{/if}
<p role="status">{message}</p>
