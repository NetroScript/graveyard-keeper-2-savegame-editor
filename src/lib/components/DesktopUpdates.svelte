<script module lang="ts">
  import type { Update } from "@tauri-apps/plugin-updater";

  export interface UpdateState {
    availability: "checking" | "supported" | "manual";
    checking: boolean;
    installing: boolean;
    update: Update | null;
    message: string;
    progress: number | undefined;
    check: () => Promise<void>;
    install: () => Promise<void>;
  }
</script>

<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import type { DownloadEvent } from "@tauri-apps/plugin-updater";
  import {
    desktop,
    native,
    type Settings,
    type UpdateCheckInterval,
  } from "../document.svelte";

  let {
    ready,
    settings,
    onsettings,
    onstate,
  }: {
    ready: boolean;
    settings: Settings;
    onsettings: (settings: Settings) => Promise<void>;
    onstate: (state: UpdateState) => void;
  } = $props();

  let availability = $state<UpdateState["availability"]>("checking");
  let checking = $state(false);
  let installing = $state(false);
  let update = $state<Update | null>(null);
  let message = $state("");
  let progress = $state<number>();
  let welcomeInterval = $state<UpdateCheckInterval>("weekly");
  let welcomeError = $state("");
  let savingWelcome = $state(false);
  let initialized = false;
  let checkTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (!desktop || !ready || initialized) return;
    initialized = true;
    void initialize();
  });

  $effect(() => {
    const interval = settings.updateCheckInterval;
    const lastCheck = settings.lastUpdateCheck;
    if (!ready || availability !== "supported") return;
    untrack(() => schedule(interval, lastCheck));
  });

  $effect(() => {
    const state = {
      availability,
      checking,
      installing,
      update,
      message,
      progress,
      check: checkForUpdates,
      install: installUpdate,
    };
    untrack(() => onstate(state));
  });

  async function initialize() {
    try {
      availability = (await native<boolean>("update_install_supported"))
        ? "supported"
        : "manual";
    } catch {
      availability = "manual";
    }
  }

  function schedule(
    interval: UpdateCheckInterval | null,
    lastCheck: number | null,
  ) {
    clearTimeout(checkTimer);
    checkTimer = undefined;
    if (!interval || interval === "never") return;
    const day = 24 * 60 * 60 * 1000;
    const duration = interval === "daily" ? day : 7 * day;
    const elapsed = lastCheck === null ? duration : Date.now() - lastCheck;
    if (elapsed >= duration) {
      void checkForUpdates();
      return;
    }
    checkTimer = setTimeout(
      () => void checkForUpdates(),
      Math.max(0, duration - elapsed),
    );
  }

  async function checkForUpdates() {
    if (checking || installing) return;
    checking = true;
    message = "Checking for updates…";
    progress = undefined;
    const checkedAt = Date.now();
    try {
      await update?.close();
      update = null;
      const { check } = await import("@tauri-apps/plugin-updater");
      update = await check({ timeout: 20_000 });
      message = update
        ? `Version ${update.version} is available.`
        : "This application is up to date.";
    } catch (updateError) {
      message = `Update check failed: ${String(updateError)}`;
    } finally {
      checking = false;
      try {
        await onsettings({ ...settings, lastUpdateCheck: checkedAt });
      } catch {}
    }
  }

  async function installUpdate() {
    if (!update) return;
    installing = true;
    message = `Downloading version ${update.version}…`;
    let downloaded = 0;
    let total: number | undefined;
    try {
      await update.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") total = event.data.contentLength;
        if (event.event === "Progress") downloaded += event.data.chunkLength;
        if (total)
          progress = Math.min(100, Math.round((downloaded / total) * 100));
        if (event.event === "Finished") message = "Installing update…";
      });
      message = "Update installed. Restart the application to use it.";
    } catch (updateError) {
      message = `Update failed: ${String(updateError)}`;
    } finally {
      installing = false;
    }
  }

  async function confirmSchedule() {
    savingWelcome = true;
    welcomeError = "";
    try {
      await onsettings({
        ...settings,
        updateCheckInterval: welcomeInterval,
      });
    } catch (settingsError) {
      welcomeError = String(settingsError);
    } finally {
      savingWelcome = false;
    }
  }

  function showModal(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }

  onDestroy(() => {
    clearTimeout(checkTimer);
    void update?.close();
  });
</script>

{#if desktop && ready && settings.updateCheckInterval === null}<div
    class="modal-backdrop"
  >
    <dialog
      use:showModal
      class="panel modal"
      aria-labelledby="update-welcome-title"
      oncancel={(event) => event.preventDefault()}
    >
      <h2 id="update-welcome-title" class="strip">Welcome</h2>
      <div class="panel-body welcome-dialog">
        <p>
          Choose how often the desktop application should check for a new
          version. You can change this later in Settings.
        </p>
        <label class="field"
          >Check for updates<select bind:value={welcomeInterval}>
            <option value="never">Never</option>
            <option value="daily">Daily</option>
            <option value="weekly">Weekly</option>
          </select></label
        >
        {#if welcomeError}<p class="warning" role="alert">
            {welcomeError}
          </p>{/if}
        <div class="form-actions">
          <button
            class="primary"
            disabled={savingWelcome}
            onclick={confirmSchedule}
            >{savingWelcome ? "Saving…" : "Continue"}</button
          >
        </div>
      </div>
    </dialog>
  </div>{/if}

<style>
  .welcome-dialog {
    display: grid;
    gap: 16px;
  }
</style>
