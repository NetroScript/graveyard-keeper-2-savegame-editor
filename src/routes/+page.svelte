<script lang="ts">
  import { onMount } from "svelte";
  import { base } from "$app/paths";
  import { createBackend, type SaveBackend, type Summary } from "$lib/save-api";
  import {
    SaveDocument,
    desktop,
    native,
    download,
    defaultSettings,
    type Settings,
    type Preview,
  } from "$lib/document.svelte";
  import LoadSaves from "$lib/components/LoadSaves.svelte";
  import SettingsView from "$lib/components/Settings.svelte";
  import About from "$lib/components/About.svelte";
  import DocumentView from "$lib/components/DocumentView.svelte";
  import { loadAssetManifest } from "$lib/assets/game-icons";
  import { preloadProgressionAssets } from "$lib/progression/catalog";
  import FolderOpen from "~icons/ph/folder-open";
  import Gear from "~icons/ph/gear-six";
  import File from "~icons/ph/file-text";
  import X from "~icons/ph/x";
  import List from "~icons/ph/list";
  import Info from "~icons/ph/info";
  import "@fontsource/roboto/latin-400.css";
  import "@fontsource/roboto/latin-500.css";
  import "@fontsource/roboto/latin-700.css";
  import "$lib/style.css";
  let backend: SaveBackend;
  let ready = $state(false);
  let documents = $state<SaveDocument[]>([]);
  let active = $state<number | "load" | "settings" | "about">("load");
  let settings = $state<Settings>({ ...defaultSettings });
  let drawer = $state(false);
  let error = $state("");
  let conflict = $state<SaveDocument>();
  let closing = $state<SaveDocument>();
  let assetVersion = $state("Loading…");
  onMount(() => {
    let alive = true;
    void preloadProgressionAssets();
    loadAssetManifest()
      .then((manifest) => {
        if (alive) assetVersion = manifest.gameVersion || "Unknown";
      })
      .catch(() => {
        if (alive) assetVersion = "Unavailable";
      });
    createBackend()
      .then(async (b) => {
        backend = b;
        if (desktop) settings = await native<Settings>("settings_get");
        else {
          try {
            settings = {
              ...defaultSettings,
              ...JSON.parse(localStorage.getItem("gk2-settings") ?? "{}"),
            };
          } catch {}
        }
        if (alive) ready = true;
      })
      .catch((e) => (error = String(e)));
    return () => {
      alive = false;
      backend?.dispose();
    };
  });
  async function onsettings(value: Settings) {
    if (
      !Number.isFinite(value.interfaceScale) ||
      value.interfaceScale < 0.75 ||
      value.interfaceScale > 1.5
    )
      throw new Error("Invalid scale");
    if (desktop) await native("settings_set", { settings: value });
    else localStorage.setItem("gk2-settings", JSON.stringify(value));
    settings = value;
  }
  function activate(id: typeof active) {
    active = id;
    drawer = false;
  }
  async function onfiles(files: File[]) {
    const infos = new Map(
      files
        .filter((f) => f.name.toLowerCase().endsWith(".info"))
        .map((f) => [f.name.slice(0, -5).toLowerCase(), f]),
    );
    const saves = files.filter((f) => f.name.toLowerCase().endsWith(".dat"));
    if (!saves.length)
      throw new Error(
        "Choose at least one .dat file, optionally with its matching .info.",
      );
    const failures = [];
    for (const file of saves) {
      try {
        const companion = infos.get(file.name.slice(0, -4).toLowerCase());
        const infoBytes = companion
          ? new Uint8Array(await companion.arrayBuffer())
          : undefined;
        let metadata = null;
        try {
          if (infoBytes)
            metadata = JSON.parse(new TextDecoder().decode(infoBytes));
        } catch {}
        const summary = await backend.open(
          new Uint8Array(await file.arrayBuffer()),
        );
        const doc = new SaveDocument(
          backend,
          summary,
          { name: file.name, metadata },
          infoBytes,
        );
        documents.push(doc);
        activate(doc.id);
      } catch (e) {
        failures.push(`${file.name}: ${e}`);
      }
    }
    if (failures.length) throw new Error(failures.join("\n"));
  }
  async function onpath(path: string) {
    try {
      const result = await native<{ summary: Summary; preview: Preview }>(
        "save_open_path",
        { path },
      );
      if (!documents.some((d) => d.id === result.summary.documentId))
        documents.push(
          new SaveDocument(backend, result.summary, result.preview),
        );
      activate(result.summary.documentId);
    } catch (e) {
      error = String(e);
    }
  }
  async function save(doc: SaveDocument, as: boolean) {
    try {
      await doc.settled();
      if (desktop) {
        const destination =
          as || !doc.path
            ? await native<string | null>("save_dialog", {
                kind: "save",
                document: doc.id,
              })
            : null;
        if ((as || !doc.path) && !destination) return;
        const result = await native<{ summary: Summary; preview: Preview }>(
          "save_write",
          { document: doc.id, revision: doc.summary!.revision, destination },
        );
        doc.summary = result.summary;
        doc.updatePreview(result.preview);
      } else {
        const revision = doc.summary!.revision;
        download(await backend.export(doc.id), doc.name);
        if (doc.infoBytes)
          download(doc.infoBytes, doc.name.replace(/\.dat$/i, ".info"));
        doc.summary = await doc.query<Summary>({ op: "mark_saved", revision });
      }
      conflict = undefined;
      doc.error = "";
    } catch (e) {
      if (String(e).includes("EXTERNAL_CHANGE")) conflict = doc;
      else doc.error = String(e);
    }
  }
  async function close(doc: SaveDocument) {
    await doc.query({ op: "close" });
    documents = documents.filter((d) => d.id !== doc.id);
    if (active === doc.id) activate("load");
    closing = undefined;
  }
  function showModal(node: HTMLDialogElement) {
    node.showModal();
    return { destroy: () => node.close() };
  }
  async function reload(doc: SaveDocument) {
    const path = doc.path;
    if (path) {
      await close(doc);
      await onpath(path);
    }
    conflict = undefined;
  }
</script>

<svelte:head
  ><title>Graveyard Keeper 2 Save Editor</title><meta
    name="description"
    content="Save editor for Graveyard Keeper 2."
  /></svelte:head
>
<div class="workspace" style={`--interface-scale:${settings.interfaceScale}`}>
  <button
    class="mobile-menu"
    aria-label="Toggle navigation"
    onclick={() => (drawer = !drawer)}><List /></button
  >
  {#if drawer}<button
      class="drawer-scrim"
      aria-label="Close navigation"
      onclick={() => (drawer = false)}
    ></button>{/if}
  <aside class:open={drawer} class="rail">
    <div class="brand">
      <img class="brand-icon" src={`${base}/app-icon.png`} alt="" />
      <div>GRAVEYARD KEEPER <b>2</b><small>SAVE EDITOR</small></div>
    </div>
    <nav aria-label="Workspace">
      <button class:active={active === "load"} onclick={() => activate("load")}
        ><FolderOpen /><span>Load Saves</span></button
      >
      <div class="rail-caption">OPEN SAVES <span>{documents.length}</span></div>
      {#each documents as doc (doc.id)}<div
          class="rail-document"
          class:active={active === doc.id}
        >
          <button onclick={() => activate(doc.id)} title={doc.name}
            ><File /><span>{doc.name}</span>{#if doc.summary?.dirty}<i
                aria-label="Unsaved changes">●</i
              >{/if}</button
          ><button
            class="close-save"
            aria-label={`Close ${doc.name}`}
            onclick={() => (doc.summary?.dirty ? (closing = doc) : close(doc))}
            ><X /></button
          >
        </div>{/each}
    </nav>
    <div
      class="asset-version"
      title="Game version represented by the loaded asset pack"
    >
      Asset game version <b>{assetVersion}</b>
    </div>
    <button
      class="about-link"
      class:active={active === "about"}
      onclick={() => activate("about")}><Info />About</button
    ><button
      class="settings-link"
      class:active={active === "settings"}
      onclick={() => activate("settings")}><Gear />Settings</button
    >
  </aside>
  <main>
    {#if error}<p class="error-banner" role="alert">
        {error}
      </p>{/if}{#if ready}<div class="page-content" hidden={active !== "load"}>
        <LoadSaves {onfiles} {onpath} {settings} {onsettings} />
      </div>
      <div class="page-content" hidden={active !== "settings"}>
        <SettingsView {settings} {onsettings} />
      </div>
      <div class="page-content" hidden={active !== "about"}>
        <About {assetVersion} />
      </div>
      {#each documents as doc (doc.id)}<div
          class="document-workspace"
          hidden={active !== doc.id}
        >
          <DocumentView {doc} {settings} onsave={save} />
        </div>{/each}{:else}<p class="page-content">Loading editor…</p>{/if}
  </main>
</div>
{#if conflict}<div class="modal-backdrop">
    <dialog
      use:showModal
      class="panel modal"
      aria-label="Save changed on disk"
      oncancel={() => (conflict = undefined)}
    >
      <h2 class="strip">Save changed on disk</h2>
      <div class="panel-body">
        <p>
          Another process changed {conflict.name}. Reload the disk version, or
          save your edits to another file.
        </p>
        <div class="form-actions">
          <button onclick={() => conflict && reload(conflict)}>Reload</button
          ><button
            class="primary"
            onclick={() => conflict && save(conflict, true)}>Save As</button
          ><button onclick={() => (conflict = undefined)}>Cancel</button>
        </div>
      </div>
    </dialog>
  </div>{/if}
{#if closing}<div class="modal-backdrop">
    <dialog
      use:showModal
      class="panel modal"
      aria-label="Close unsaved document"
      oncancel={() => (closing = undefined)}
    >
      <h2 class="strip">Unsaved changes</h2>
      <div class="panel-body">
        <p>Close {closing.name} and discard its edits?</p>
        <div class="form-actions">
          <button class="danger" onclick={() => closing && close(closing)}
            >Discard and close</button
          ><button onclick={() => (closing = undefined)}>Keep editing</button>
        </div>
      </div>
    </dialog>
  </div>{/if}
