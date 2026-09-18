interface WasmSession {
  open(bytes: Uint8Array): string;
  request(document: number, json: string): string;
  export(document: number): Uint8Array;
}
// Generated into static/wasm, not required for desktop compilation.
async function load(url: string): Promise<WasmSession> {
  const wasm = await import(/* @vite-ignore */ url);
  await wasm.default();
  return new wasm.SaveSession();
}
let session: Promise<WasmSession> | undefined;
let queue = Promise.resolve();
self.onmessage = ({ data }) => {
  queue = queue.then(async () => {
    try {
      const core = await (session ??= load(data.moduleUrl));
      let result: unknown;
      switch (data.op) {
        case "open":
          result = JSON.parse(core.open(data.payload));
          break;
        case "request":
          result = JSON.parse(
            core.request(
              data.payload.document,
              JSON.stringify(data.payload.request),
            ),
          );
          break;
        case "export": {
          const bytes = core.export(data.payload).slice();
          self.postMessage(
            { id: data.id, result: bytes.buffer },
            { transfer: [bytes.buffer] },
          );
          return;
        }
        default:
          throw new Error("Unknown backend operation");
      }
      self.postMessage({ id: data.id, result });
    } catch (error) {
      self.postMessage({ id: data.id, error: String(error) });
    }
  });
};
export {};
