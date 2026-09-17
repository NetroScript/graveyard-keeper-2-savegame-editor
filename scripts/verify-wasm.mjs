// Tests the actual web-target .wasm artifact in Node, without a server or any file writes.
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import init, { SaveSession } from "../static/wasm/gk2_save_wasm.js";

await init({ module_or_path: await readFile(new URL("../static/wasm/gk2_save_wasm_bg.wasm", import.meta.url)) });
const bytes = process.argv[2] ? await readFile(process.argv[2]) : Uint8Array.of(4, 46, 44, 0, 5);
const session = new SaveSession();
try {
  const summary = JSON.parse(session.open(bytes));
  assert.deepEqual(Buffer.from(session.export()), Buffer.from(bytes));
  const request = (value) => JSON.parse(session.request(JSON.stringify(value)));
  const roots = request({ op: "children", parent: null, offset: 0, limit: 200 }).data;
  const fields = request({ op: "children", parent: roots.nodes[0].id, offset: 0, limit: 200 }).data;
  const field = fields.nodes.find((node) => node.kind === "bool");
  assert.ok(field, "Expected a boolean field for the edit probe");
  const edited = request({ op: "set_value", edit: { node: field.id, expectedTag: field.tag, revision: summary.revision, value: field.value === "true" ? "false" : "true" } }).data;
  assert.equal(Buffer.from(session.export()).filter((byte, index) => byte !== bytes[index]).length, 1);
  request({ op: "set_value", edit: { node: field.id, expectedTag: field.tag, revision: edited.revision, value: field.value } });
  assert.deepEqual(Buffer.from(session.export()), Buffer.from(bytes));
  assert.throws(() => session.open(Uint8Array.of(255)));
  assert.deepEqual(Buffer.from(session.export()), Buffer.from(bytes));
  console.log(`WASM: exact round trip and reversible edit passed (${summary.originalBytes} bytes; ${summary.records} records).`);
} finally { session.free(); }
