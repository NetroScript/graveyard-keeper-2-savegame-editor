import { base } from "$app/paths";

// DTOs mirror gk2-save-core::service. All editable numeric values cross IPC as strings.
export interface Summary {
  originalBytes: number; encodedBytes: number; records: number;
  types: number; objects: number; revision: number;
}
export interface NodeView {
  id: number; parent: number | null; name: string | null; kind: string; tag: number;
  typeName: string | null; value: string | null; referenceTarget: number | null;
  childCount: number; originalOffset: number; editable: boolean;
}
export type SaveRequest =
  | { op: "children"; parent: number | null; offset: number; limit: number }
  | { op: "set_value"; edit: { node: number; expectedTag: number; revision: number; value: string } }
  | { op: "close" };
export type SaveResponse =
  | { op: "children"; data: { nodes: NodeView[]; total: number } }
  | { op: "set_value"; data: Summary }
  | { op: "close" };
export interface SaveBackend {
  open(bytes: Uint8Array): Promise<Summary>;
  request(request: SaveRequest): Promise<SaveResponse>;
  export(): Promise<Uint8Array>;
  dispose(): void;
}

export async function createBackend(): Promise<SaveBackend> {
  if (import.meta.env.MODE === "desktop") {
    const { invoke } = await import("@tauri-apps/api/core");
    return {
      open: (bytes) => invoke<Summary>("save_open", bytes),
      request: (request) => invoke<SaveResponse>("save_request", { request }),
      export: async () => new Uint8Array(await invoke<ArrayBuffer>("save_export")),
      dispose: () => { void invoke("save_request", { request: { op: "close" } }).catch(() => {}); },
    };
  }
  const worker = new Worker(new URL("./save-worker.ts", import.meta.url), { type: "module" });
  const moduleUrl = new URL(`${base}/wasm/gk2_save_wasm.js`, window.location.origin).href;
  let nextId = 0;
  let failure: Error | null = null;
  const pending = new Map<number, { resolve: (data: unknown) => void; reject: (error: Error) => void }>();
  const fail = (error: Error) => {
    failure = error;
    for (const task of pending.values()) task.reject(error);
    pending.clear();
  };
  worker.onerror = (event) => fail(new Error(event.message || "WebAssembly worker failed"));
  worker.onmessage = ({ data }) => {
    const task = pending.get(data.id);
    if (!task) return;
    pending.delete(data.id);
    if (data.error) task.reject(new Error(data.error)); else task.resolve(data.result);
  };
  function call<T>(op: string, payload?: unknown, transfer: Transferable[] = []): Promise<T> {
    if (failure) return Promise.reject(failure);
    const id = nextId++;
    return new Promise<T>((resolve, reject) => {
      pending.set(id, { resolve: (value) => resolve(value as T), reject });
      worker.postMessage({ id, op, payload, moduleUrl }, transfer);
    });
  }
  return {
    open: (bytes) => {
      const copy = bytes.slice(); // Transfer a copy; callers retain their input.
      return call<Summary>("open", copy, [copy.buffer]);
    },
    request: (request) => call<SaveResponse>("request", request),
    export: async () => new Uint8Array(await call<ArrayBuffer>("export")),
    dispose: () => { fail(new Error("Save session closed")); worker.terminate(); },
  };
}
