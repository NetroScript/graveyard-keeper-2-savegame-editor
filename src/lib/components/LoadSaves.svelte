<script lang="ts">
  import {
    desktop,
    native,
    type Preview,
    type Settings,
  } from "../document.svelte";
  import Upload from "~icons/ph/upload-simple";
  import FolderOpen from "~icons/ph/folder-open";
  import Copy from "~icons/ph/copy";
  import ArrowClockwise from "~icons/ph/arrow-clockwise";
  import DesktopTower from "~icons/ph/desktop-tower";
  import Tag from "~icons/ph/tag";
  import Archive from "~icons/ph/archive";
  import GameIcon from "./GameIcon.svelte";
  import LoadingIndicator from "./LoadingIndicator.svelte";
  import { gameDayIcon } from "../assets/game-icons";
  let {
    onfiles,
    onpath,
    settings,
    onsettings,
    assetVersion,
  }: {
    onfiles: (files: File[]) => Promise<void>;
    onpath: (path: string) => Promise<void>;
    settings: Settings;
    onsettings: (s: Settings) => Promise<void>;
    assetVersion: string;
  } = $props();
  let input = $state<HTMLInputElement>();
  let hovering = $state(false);
  let error = $state("");
  let copied = $state(false);
  let saves = $state<Preview[]>([]);
  let discovering = $state(desktop);
  let includeBackups = $state(false);
  interface BackupPreview {
    name: string;
    modified: number;
    bytes: number;
    metadata?: Record<string, unknown> | null;
    metadataError?: string | null;
  }
  let backupSave = $state<Preview>();
  let backups = $state<BackupPreview[]>([]);
  let selectedBackup = $state<BackupPreview>();
  let backupsLoading = $state(false);
  let restoring = $state(false);
  let opening = $state(false);
  let backupMessage = $state("");
  let platform = $state(
    navigator.platform.startsWith("Win")
      ? "Windows"
      : navigator.platform.includes("Mac")
        ? "macOS"
        : "Linux",
  );
  const paths: Record<string, string> = {
    Windows:
      "%USERPROFILE%\\AppData\\LocalLow\\Lazy Bear Games\\Graveyard Keeper 2",
    macOS: "~/Library/Application Support/Lazy Bear Games/Graveyard Keeper 2",
    Linux: "~/.config/unity3d/Lazy Bear Games/Graveyard Keeper 2",
    Proton:
      "~/.local/share/Steam/steamapps/compatdata/4358690/pfx/drive_c/users/steamuser/AppData/LocalLow/Lazy Bear Games/Graveyard Keeper 2",
  };
  async function refresh() {
    if (!saves.length) discovering = true;
    try {
      saves = (
        await native<{ saves: Preview[] }>("save_discover", { includeBackups })
      ).saves;
    } catch (e) {
      error = String(e);
    } finally {
      discovering = false;
    }
  }
  $effect(() => {
    if (desktop) {
      const backups = includeBackups;
      void refresh();
    }
  });
  async function dialog(folder: boolean) {
    try {
      const path = await native<string | null>("save_dialog", {
        kind: folder ? "folder" : "open",
        document: null,
      });
      if (path) {
        if (folder) {
          await onsettings({
            ...settings,
            customDirectories: [
              ...new Set([...settings.customDirectories, path]),
            ],
          });
          await refresh();
        } else await openPath(path);
      }
    } catch (e) {
      error = String(e);
    }
  }
  async function files(list: File[]) {
    opening = true;
    try {
      await onfiles(list);
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      opening = false;
    }
  }
  async function openPath(path: string) {
    opening = true;
    try {
      await onpath(path);
    } finally {
      opening = false;
    }
  }
  async function manageBackups(save: Preview) {
    if (!save.path) return;
    backupSave = save;
    backups = [];
    selectedBackup = undefined;
    backupMessage = "";
    backupsLoading = true;
    try {
      backups = (
        await native<{ backups: BackupPreview[] }>("save_backups", {
          path: save.path,
        })
      ).backups;
    } catch (e) {
      backupMessage = String(e);
    } finally {
      backupsLoading = false;
    }
  }
  async function restoreBackup() {
    if (!backupSave?.path || !selectedBackup || restoring) return;
    restoring = true;
    backupMessage = "";
    try {
      await native("save_restore_backup", {
        path: backupSave.path,
        backup: selectedBackup.name,
      });
      backupMessage =
        "Backup restored. The previous current save was retained in the backup history.";
      selectedBackup = undefined;
      await refresh();
      backups = (
        await native<{ backups: BackupPreview[] }>("save_backups", {
          path: backupSave.path,
        })
      ).backups;
    } catch (e) {
      backupMessage = String(e);
    } finally {
      restoring = false;
    }
  }
  function backupDate(seconds: number) {
    return seconds ? new Date(seconds * 1000).toLocaleString() : "Unknown date";
  }
  function backupSize(bytes: number) {
    return `${(bytes / 1024 / 1024).toLocaleString(undefined, { maximumFractionDigits: 1 })} MB`;
  }
  function saveMayBeNewer(version: unknown) {
    const save = String(version ?? "")
      .match(/^\d+(?:\.\d+)+$/)?.[0]
      .split(".")
      .map(Number);
    const assets = assetVersion
      .match(/^\d+(?:\.\d+)+$/)?.[0]
      .split(".")
      .map(Number);
    if (!save || !assets) return false;
    for (let index = 0; index < Math.max(save.length, assets.length); index++) {
      const difference = (save[index] ?? 0) - (assets[index] ?? 0);
      if (difference) return difference > 0;
    }
    return false;
  }
  function showModal(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
</script>

<header class="page-heading">
  <h1>Save files</h1>
  <p>Select a save file to open.</p>
</header>
{#if desktop}
  <div class="toolbar load-toolbar">
    <button onclick={refresh}><ArrowClockwise />Refresh</button><button
      onclick={() => dialog(true)}><FolderOpen />Choose Folder</button
    ><button class="primary" onclick={() => dialog(false)}
      ><Upload />Open File</button
    ><label class="check"
      ><input type="checkbox" bind:checked={includeBackups} />Include game
      backups</label
    >
  </div>
  {#if discovering}<LoadingIndicator label="Finding save files…" />{:else}<div class="save-grid">
    {#each saves as save}<article class="save-card panel">
        <button
          class="save-card-open"
          onclick={() => save.path && void openPath(save.path)}
        >
          <h3 class="strip">{save.name}</h3>
          <div class="panel-body">
            {#if save.metadata}{@const m = save.metadata}
              <div class="save-day">
                <GameIcon name={gameDayIcon(m.day)} />Day {String(
                  m.day ?? "—",
                )}<span class="save-kind"
                  >{m.isDemoSave ? "Demo" : "Release"}</span
                >
              </div>
              <p>{String(m.saveDateTime ?? "Unknown save date")}</p>
              {#if saveMayBeNewer(m.gameSaveVersion)}<p
                  class="save-compat warning"
                >
                  Newer than the loaded {assetVersion} asset data. Some content may
                  be unavailable in the editor.
                </p>{/if}
              <dl>
                <dt><Tag />Version</dt>
                <dd>{String(m.gameSaveVersion ?? "—")}</dd>
                <dt><DesktopTower />Platform</dt>
                <dd>{String(m.platform ?? "—")}</dd>
                <dt><GameIcon name="wskull" />Graveyard</dt>
                <dd>{String(m.graveyardQuality ?? "—")}</dd>
                <dt><GameIcon name="cross" variant="church" />Church</dt>
                <dd>{String(m.churchQuality ?? "—")}</dd>
                <dt><GameIcon name="village_REP" />Village reputation</dt>
                <dd>{String(m.villageRep ?? "—")}</dd>
              </dl>{:else}<p>
                {save.metadataError
                  ? "Metadata could not be read."
                  : "No companion metadata."}
              </p>
              <small>The save can still be opened.</small>{/if}
          </div>
        </button>
        {#if save.editorBackups}<div class="save-card-actions">
            <button onclick={() => manageBackups(save)}
              ><Archive />Restore backup</button
            >
          </div>{/if}
      </article>{:else}<div class="empty-state panel">
        <h2>No saves found</h2>
        <p>Choose a save folder or open an individual .dat file.</p>
      </div>{/each}
    </div>{/if}
{:else}
  <input
    class="visually-hidden"
    bind:this={input}
    type="file"
    accept=".dat,.info"
    multiple
    onchange={(e) => {
      void files(Array.from(e.currentTarget.files ?? []));
      e.currentTarget.value = "";
    }}
    aria-label="Select save files"
  />
  <button
    class="drop-zone panel"
    class:hovering
    onclick={() => input?.click()}
    ondragover={(e) => {
      e.preventDefault();
      hovering = true;
    }}
    ondragleave={() => (hovering = false)}
    ondrop={(e) => {
      e.preventDefault();
      hovering = false;
      void files(Array.from(e.dataTransfer?.files ?? []));
    }}
    ><div class="upload-emblem"><Upload /></div>
    <h2>Open save files</h2>
    <p>Drop .dat files here or click to select files.</p>
    <span>Matching .info files are optional and provide save metadata.</span
    ><small>Files are processed locally and are not uploaded.</small></button
  >
  <aside class="web-backup-note panel">
    <strong>Keep your original save files</strong>
    <p>
      The web editor downloads edited copies and cannot manage backups in the
      game's save folder. Keep the original <code>.dat</code> and matching
      <code>.info</code> files until the edited save works in the game.
    </p>
    <a
      href="https://github.com/NetroScript/graveyard-keeper-2-savegame-editor/releases"
      target="_blank"
      rel="noreferrer"
      >Download the desktop application for automatic paired backups</a
    >
  </aside>
  <section class="panel location-panel">
    <h3 class="strip">Save file locations</h3>
    <div class="panel-body">
      <label class="field"
        >Platform<select bind:value={platform}
          >{#each Object.keys(paths) as p}<option>{p}</option>{/each}</select
        ></label
      >
      <div class="copy-path">
        <code>{paths[platform]}</code><button
          aria-label="Copy save location"
          onclick={async () => {
            try {
              await navigator.clipboard.writeText(paths[platform]);
              copied = true;
              setTimeout(() => (copied = false), 2000);
            } catch {
              error = "Clipboard unavailable. Select and copy the path above.";
            }
          }}><Copy />{copied ? "Copied" : "Copy"}</button
        >
      </div>
      <p class="hint">
        Steam libraries may be stored elsewhere. Linux also respects
        XDG_CONFIG_HOME.
      </p>
    </div>
  </section>
{/if}
{#if error}<p class="error-banner" role="alert">{error}</p>{/if}
{#if backupSave}<div class="modal-backdrop">
    <dialog
      use:showModal
      class="panel modal backup-dialog"
      aria-labelledby="backup-dialog-title"
      oncancel={() => !restoring && (backupSave = undefined)}
    >
      <h2 id="backup-dialog-title" class="strip">Restore backup</h2>
      <div class="panel-body">
        <p>Select an earlier version of <strong>{backupSave.name}</strong>.</p>
        {#if backupsLoading}<LoadingIndicator label="Loading backups…" compact />
        {:else if backups.length}<div class="backup-list">
            {#each backups as backup (backup.name)}<button
                class:selected={selectedBackup?.name === backup.name}
                aria-pressed={selectedBackup?.name === backup.name}
                onclick={() => (selectedBackup = backup)}
              >
                <strong>{backupDate(backup.modified)}</strong>
                <span>{backupSize(backup.bytes)}</span>
                {#if backup.metadata}<small
                    >Day {String(backup.metadata.day ?? "—")} · {String(
                      backup.metadata.gameSaveVersion ?? "Unknown version",
                    )}</small
                  >{:else if backup.metadataError}<small
                    >Companion metadata could not be read</small
                  >
                {:else}<small>No companion metadata</small>{/if}
              </button>{/each}
          </div>
        {:else}<p class="hint">
            No editor backups are available for this save.
          </p>{/if}
        {#if selectedBackup}<p class="warning backup-warning">
            Restoring replaces the current <code>.dat</code> and matching
            <code>.info</code>. {settings.backupRetention > 0
              ? "The current pair will first be retained as another rolling backup."
              : "Automatic backups are disabled, so the current pair will not be retained."}
          </p>{/if}
        {#if backupMessage}<p class="hint" role="status">
            {backupMessage}
          </p>{/if}
        {#if restoring}<LoadingIndicator label="Restoring backup…" compact />{/if}
        <div class="form-actions">
          <button disabled={restoring} onclick={() => (backupSave = undefined)}
            >Close</button
          >
          <button
            class="primary"
            disabled={!selectedBackup || restoring}
            onclick={restoreBackup}
            >{restoring ? "Restoring…" : "Restore selected"}</button
          >
        </div>
      </div>
    </dialog>
  </div>{/if}
{#if opening}<div class="modal-backdrop">
    <div class="panel open-loader">
      <LoadingIndicator label="Opening save…" compact />
    </div>
  </div>{/if}

<style>
  .web-backup-note {
    margin-top: 16px;
    padding: 14px 16px;
  }
  .web-backup-note strong {
    color: #e3c36c;
  }
  .web-backup-note p {
    margin: 6px 0;
    color: #b8bdc7;
  }
  .web-backup-note a {
    color: #e3bd78;
  }
  .save-card-open {
    display: block;
    width: 100%;
    padding: 0;
    text-align: left;
    color: inherit;
    background: transparent;
    border: 0;
  }
  .save-card-open:hover {
    background: #ffffff05;
  }
  .save-card-actions {
    display: flex;
    justify-content: flex-end;
    padding: 0 12px 12px;
  }
  .save-card-actions button {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .save-card-actions :global(svg) {
    width: 16px;
    height: 16px;
  }
  .backup-dialog {
    width: min(620px, calc(100vw - 32px));
  }
  .backup-list {
    display: grid;
    max-height: min(420px, 55dvh);
    overflow: auto;
    border: 1px solid var(--border);
  }
  .backup-list button {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 3px 16px;
    padding: 11px 13px;
    text-align: left;
    background: #24262e;
    border: 0;
    border-bottom: 1px solid #484b55;
  }
  .backup-list button:last-child {
    border-bottom: 0;
  }
  .backup-list button.selected {
    color: var(--cream);
    background: #4b4335;
    outline: 1px solid #b49157;
    outline-offset: -1px;
  }
  .backup-list small {
    grid-column: 1 / -1;
    color: var(--muted);
  }
  .backup-warning {
    margin-top: 14px;
    padding: 10px 12px;
  }
  .save-compat {
    margin: 8px 0;
    padding: 7px 9px;
  }
  .open-loader {
    width: min(420px, calc(100vw - 32px));
  }
</style>
