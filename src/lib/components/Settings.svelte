<script lang="ts">
  import { desktop, type Settings } from "../document.svelte";
  import { notify, notifyError } from "../toasts.svelte";
  import { t, languages, type Language } from "../i18n.svelte";
  let {
    settings,
    onsettings,
  }: { settings: Settings; onsettings: (s: Settings) => Promise<void> } =
    $props();
  async function update(s: Settings) {
    try {
      await onsettings(s);
      notify(t("Settings saved"));
    } catch (e) {
      notifyError(e);
    }
  }
</script>

<header class="page-heading">
  <h1>{t("Settings")}</h1>
  <p>{t("Configure the editor.")}</p>
</header>
<section class="panel settings-panel">
  <h2 class="strip">{t("Interface")}</h2>
  <div class="panel-body">
    <label class="field"
      >{t("Language")}<select
        value={settings.language ?? "auto"}
        onchange={(e) =>
          update({
            ...settings,
            language:
              e.currentTarget.value === "auto"
                ? null
                : (e.currentTarget.value as Language),
          })}
        ><option value="auto">{t("Auto (system language)")}</option
        >{#each languages as language}<option value={language.value}
            >{language.label}</option
          >{/each}</select
      ></label
    >
    <label class="field"
      >{t("Interface scale")}<select
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
  <h2 class="strip">{t("Editing")}</h2>
  <div class="panel-body">
    <label
      ><input
        type="checkbox"
        checked={settings.outOfBoundsEdits}
        onchange={(e) =>
          update({ ...settings, outOfBoundsEdits: e.currentTarget.checked })}
      /> {t("Allow out of bounds edits")}</label
    >
    <p class="hint">
      {t(
        "Allow inventory capacity changes and item amounts above the normal stack size.",
      )}
    </p>
  </div>
</section>
{#if desktop}<section class="panel settings-panel">
    <h2 class="strip">{t("Application updates")}</h2>
    <div class="panel-body">
      <label class="field"
        >{t("Check for updates")}<select
          value={settings.updateCheckInterval ?? "weekly"}
          onchange={(e) =>
            update({
              ...settings,
              updateCheckInterval: e.currentTarget.value as
                "never" | "daily" | "weekly",
            })}
        >
          <option value="never">{t("Never")}</option>
          <option value="daily">{t("Daily")}</option>
          <option value="weekly">{t("Weekly")}</option>
        </select></label
      >
      <p class="hint">
        {t(
          "The desktop application checks on this schedule while it is open and when it starts. You can still check manually from About.",
        )}
      </p>
    </div>
  </section>
  <section class="panel settings-panel">
    <h2 class="strip">{t("Save protection")}</h2>
    <div class="panel-body">
      <label class="field"
        >{t("Backups per save")}<input
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
        {t(
          "Each editor backup is a compressed ZIP containing the previous .dat and .info together, separately from game backups. Set to 0 to disable them.",
        )}
      </p>
      <h3>{t("Custom save directories")}</h3>
      {#each settings.customDirectories as path}<div class="copy-path">
          <code>{path}</code><button
            onclick={() =>
              update({
                ...settings,
                customDirectories: settings.customDirectories.filter(
                  (p) => p !== path,
                ),
              })}>{t("Remove")}</button
          >
        </div>{:else}<p class="hint">
          {t("Use Choose Folder on Load Saves to add a directory.")}
        </p>{/each}
    </div>
  </section>{:else}<p class="hint">
    {t(
      "Saving downloads a copy. Automatic backups and folder discovery are unavailable.",
    )}
  </p>{/if}
