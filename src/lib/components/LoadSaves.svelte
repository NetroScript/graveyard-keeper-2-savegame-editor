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
  import GameIcon from "./GameIcon.svelte";
  import { gameDayIcon } from "../assets/game-icons";
  let {
    onfiles,
    onpath,
    settings,
    onsettings,
  }: {
    onfiles: (files: File[]) => Promise<void>;
    onpath: (path: string) => Promise<void>;
    settings: Settings;
    onsettings: (s: Settings) => Promise<void>;
  } = $props();
  let input = $state<HTMLInputElement>();
  let hovering = $state(false);
  let error = $state("");
  let copied = $state(false);
  let saves = $state<Preview[]>([]);
  let includeBackups = $state(false);
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
    try {
      saves = (
        await native<{ saves: Preview[] }>("save_discover", { includeBackups })
      ).saves;
    } catch (e) {
      error = String(e);
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
        } else await onpath(path);
      }
    } catch (e) {
      error = String(e);
    }
  }
  async function files(list: File[]) {
    try {
      await onfiles(list);
      error = "";
    } catch (e) {
      error = String(e);
    }
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
  <div class="save-grid">
    {#each saves as save}<button
        class="save-card panel"
        onclick={() => save.path && onpath(save.path)}
        ><h3 class="strip">{save.name}</h3>
        <div class="panel-body">
          {#if save.metadata}{@const m = save.metadata}
            <div class="save-day">
              <GameIcon name={gameDayIcon(m.day)} />Day {String(
                m.day ?? "—",
              )}<span>{m.isDemoSave ? "Demo" : "Release"}</span>
            </div>
            <p>{String(m.saveDateTime ?? "Unknown save date")}</p>
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
        </div></button
      >{:else}<div class="empty-state panel">
        <h2>No saves found</h2>
        <p>Choose a save folder or open an individual .dat file.</p>
      </div>{/each}
  </div>
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
