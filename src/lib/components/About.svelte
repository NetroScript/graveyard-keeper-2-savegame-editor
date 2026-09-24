<script lang="ts">
  import { onDestroy } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Update } from "@tauri-apps/plugin-updater";
  import Info from "~icons/ph/info";
  import GithubLogo from "~icons/ph/github-logo";
  import Star from "~icons/ph/star";
  import Bug from "~icons/ph/bug";
  import Download from "~icons/ph/download-simple";
  import ArrowClockwise from "~icons/ph/arrow-clockwise";
  import { desktop } from "../document.svelte";
  import packageInfo from "../../../package.json";

  let { assetVersion }: { assetVersion: string } = $props();
  let checking = $state(false);
  let installing = $state(false);
  let update = $state<Update | null>(null);
  let message = $state("");
  let progress = $state<number>();

  const repository =
    "https://github.com/NetroScript/graveyard-keeper-2-savegame-editor";
  const webEditor =
    "https://netroscript.github.io/graveyard-keeper-2-savegame-editor/";
  const desktopDownloads = `${repository}/releases`;

  async function external(event: MouseEvent, url: string) {
    if (!desktop) return;
    event.preventDefault();
    await openUrl(url);
  }

  async function checkForUpdates() {
    checking = true;
    message = "Checking for updates…";
    progress = undefined;
    try {
      await update?.close();
      const { check } = await import("@tauri-apps/plugin-updater");
      update = await check({ timeout: 20_000 });
      message = update
        ? `Version ${update.version} is available.`
        : "This application is up to date.";
    } catch (error) {
      message = `Update check failed: ${String(error)}`;
    } finally {
      checking = false;
    }
  }

  async function installUpdate() {
    if (!update) return;
    installing = true;
    message = `Downloading version ${update.version}…`;
    let downloaded = 0;
    let total: number | undefined;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength;
        if (event.event === "Progress") downloaded += event.data.chunkLength;
        if (total)
          progress = Math.min(100, Math.round((downloaded / total) * 100));
        if (event.event === "Finished") message = "Installing update…";
      });
      message = "Update installed. Restart the application to use it.";
    } catch (error) {
      message = `Update failed: ${String(error)}`;
    } finally {
      installing = false;
    }
  }

  onDestroy(() => void update?.close());
</script>

<header class="page-heading">
  <h1>About</h1>
  <p>Graveyard Keeper 2 Save Editor</p>
</header>
<div class="about-grid">
  <section class="panel">
    <h2 class="strip"><Info />Application</h2>
    <div class="panel-body">
      <dl>
        <dt>Editor version</dt>
        <dd>{packageInfo.version}</dd>
        <dt>Game data version</dt>
        <dd>{assetVersion}</dd>
        <dt>License</dt>
        <dd>MIT</dd>
      </dl>
      <p>
        The MIT License applies only to the editor's original source code.
        Graveyard Keeper 2 and the game artwork and other game assets distributed
        with the editor remain the property of Lazy Bear Games and their
        respective rights holders. No ownership of, or license to, those assets
        is claimed, offered, or granted by this project. Their inclusion does not
        imply authorization, affiliation, or endorsement by Lazy Bear Games or
        any other rights holder.
      </p>
      <p>
        If Lazy Bear Games or another applicable rights holder requests their
        removal, the project maintainers will promptly remove the relevant assets
        from the hosted web editor and from future downloadable distributions
        under their control.
      </p>
    </div>
  </section>
  <section class="panel">
    <h2 class="strip"><GithubLogo />Project</h2>
    <div class="panel-body about-actions">
      <p>If the editor is useful, consider starring its repository.</p>
      <a
        href={repository}
        target="_blank"
        rel="noreferrer"
        onclick={(e) => external(e, repository)}
        ><Star />View and star on GitHub</a
      >
      <a
        href={`${repository}/issues`}
        target="_blank"
        rel="noreferrer"
        onclick={(e) => external(e, `${repository}/issues`)}
        ><Bug />Report an issue</a
      >
      <a
        href={`${repository}/blob/master/LICENSE`}
        target="_blank"
        rel="noreferrer"
        onclick={(e) => external(e, `${repository}/blob/master/LICENSE`)}
        >Read the MIT License</a
      >
    </div>
  </section>
  <section class="panel edition-panel">
    <h2 class="strip"><Download />Web and desktop</h2>
    <div class="panel-body">
      <p>You are using the <strong>{desktop ? "desktop application" : "web editor"}</strong>.</p>
      <div class="edition-comparison">
        <section>
          <h3>Web editor</h3>
          <p>Runs without installation and updates automatically with the website.</p>
          <p>You select saves manually and receive edited files as downloads. Keep your original <code>.dat</code> and <code>.info</code> files as your backup.</p>
        </section>
        <section>
          <h3>Desktop application</h3>
          <p>Finds installed saves and writes them back safely with configurable compressed backups containing the matching save pair, plus external-change checks.</p>
          <p>It can check for application updates, but must be downloaded or installed on your computer.</p>
        </section>
      </div>
      <a
        class="other-edition"
        href={desktop ? webEditor : desktopDownloads}
        target="_blank"
        rel="noreferrer"
        onclick={(e) => external(e, desktop ? webEditor : desktopDownloads)}
        ><Download />{desktop ? "Open the web editor" : "Download the desktop application"}</a
      >
    </div>
  </section>
  {#if desktop}<section class="panel updater-panel">
      <h2 class="strip"><ArrowClockwise />Updates</h2>
      <div class="panel-body">
        <p>Updates are checked only when you request one.</p>
        <div class="form-actions">
          <button disabled={checking || installing} onclick={checkForUpdates}
            ><ArrowClockwise />{checking
              ? "Checking…"
              : "Check for updates"}</button
          >
          {#if update}<button
              class="primary"
              disabled={installing}
              onclick={installUpdate}
              ><Download />{installing
                ? "Installing…"
                : `Download and install ${update.version}`}</button
            >{/if}
        </div>
        {#if progress !== undefined}<progress max="100" value={progress}
            >{progress}%</progress
          >{/if}
        {#if message}<p role="status" class="hint">{message}</p>{/if}
      </div>
  </section>{/if}
</div>

<style>
  .edition-panel { grid-column:1 / -1; }
  .edition-comparison { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); border:1px solid var(--border); background:#20222a; }
  .edition-comparison section { padding:12px 14px; }
  .edition-comparison section + section { border-left:1px solid var(--border); }
  .edition-comparison h3 { margin:0 0 7px; color:#e3c36c; font-size:14px; }
  .edition-comparison p { margin:5px 0; color:#b8bdc7; }
  .other-edition { display:inline-flex; align-items:center; gap:7px; margin-top:12px; color:#e3bd78; }
  .other-edition :global(svg) { width:18px; height:18px; }
  @media (max-width:720px) {
    .edition-comparison { grid-template-columns:1fr; }
    .edition-comparison section + section { border-left:0; border-top:1px solid var(--border); }
  }
</style>
