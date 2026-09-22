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
        <dt>Asset game version</dt>
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
