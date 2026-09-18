// Tests the actual web-target .wasm artifact in Node, without a server or any file writes.
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import init, { SaveSession } from "../static/wasm/gk2_save_wasm.js";

await init({ module_or_path: await readFile(new URL("../static/wasm/gk2_save_wasm_bg.wasm", import.meta.url)) });
const bytes = process.argv[2] ? await readFile(process.argv[2]) : Uint8Array.of(4, 46, 44, 0, 5);
const session = new SaveSession();
try {
  const summary = JSON.parse(session.open(bytes));
  assert.deepEqual(Buffer.from(session.export(summary.documentId)), Buffer.from(bytes));
  const request = (value) => JSON.parse(session.request(summary.documentId, JSON.stringify(value)));
  const roots = request({ op: "children", parent: null, offset: 0, limit: 200 });
  const fields = request({ op: "children", parent: roots.nodes[0].id, offset: 0, limit: 200 });
  const field = fields.nodes.find((node) => node.kind === "bool");
  assert.ok(field, "Expected a boolean field for the edit probe");
  const edited = request({ op: "transact", revision: summary.revision, operations: [{ op: "set", node: field.id, tag: field.tag, value: field.value === "true" ? "false" : "true" }] }).summary;
  assert.equal(Buffer.from(session.export(summary.documentId)).filter((byte, index) => byte !== bytes[index]).length, 1);
  request({ op: "undo", revision: edited.revision });
  assert.deepEqual(Buffer.from(session.export(summary.documentId)), Buffer.from(bytes));
  assert.throws(() => session.open(Uint8Array.of(255)));
  assert.deepEqual(Buffer.from(session.export(summary.documentId)), Buffer.from(bytes));
  console.log(`WASM: exact round trip and reversible edit passed (${summary.originalBytes} bytes; ${summary.records} records).`);
} finally { session.free(); }
