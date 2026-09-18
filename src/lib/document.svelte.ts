import type { SaveBackend, Summary, Operation, NodeView } from "./save-api";
export interface Preview {
  path?: string;
  name: string;
  metadata?: Record<string, unknown> | null;
  metadataError?: string | null;
}
export interface GeneralField {
  key: string;
  node: number | null;
  value: string | null;
  error: string | null;
}
export class SaveDocument {
  summary = $state<Summary>();
  name = $state("");
  path = $state<string>();
  metadata = $state<Record<string, unknown> | null>();
  busy = $state(false);
  error = $state("");
  private queue = Promise.resolve();
  constructor(
    public backend: SaveBackend,
    summary: Summary,
    preview: Preview,
    public infoBytes?: Uint8Array,
  ) {
    this.summary = summary;
    this.updatePreview(preview);
  }
  get id() {
    return this.summary!.documentId;
  }
  updatePreview(p: Preview) {
    this.name = p.name;
    this.path = p.path;
    this.metadata = p.metadata;
  }
  query<T>(request: Operation) {
    return this.backend.request<T>(this.id, request);
  }
  nodes(ids: number[]) {
    return this.query<NodeView[]>({ op: "nodes", ids });
  }
  mutate(op: string, operations?: Operation[]) {
    const revision = this.summary!.revision;
    const task = this.queue.then(async () => {
      this.busy = true;
      this.error = "";
      try {
        const result = await this.query<{ summary: Summary }>({
          op,
          revision,
          ...(operations ? { operations } : {}),
        });
        this.summary = result.summary;
      } catch (e) {
        this.error = String(e);
        throw e;
      } finally {
        this.busy = false;
      }
    });
    this.queue = task.catch(() => {});
    return task;
  }
  transact(operations: Operation[]) {
    return this.mutate("transact", operations);
  }
}
export const desktop = import.meta.env.MODE === "desktop";
export interface Settings {
  backupRetention: number;
  customDirectories: string[];
  interfaceScale: number;
}
export const defaultSettings: Settings = {
  backupRetention: 5,
  customDirectories: [],
  interfaceScale: 1,
};
export async function native<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(command, args);
}
export function download(bytes: Uint8Array, name: string) {
  const url = URL.createObjectURL(new Blob([bytes.slice().buffer]));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
