// Tests the actual web-target .wasm artifact in Node, without a server or any file writes.
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import init, { SaveSession } from "../static/wasm/gk2_save_wasm.js";

await init({
  module_or_path: await readFile(
    new URL("../static/wasm/gk2_save_wasm_bg.wasm", import.meta.url),
  ),
});
const bytes = process.argv[2]
  ? await readFile(process.argv[2])
  : Uint8Array.of(4, 46, 44, 0, 5);
const session = new SaveSession();
try {
  const summary = JSON.parse(session.open(bytes));
  assert.deepEqual(
    Buffer.from(session.export(summary.documentId)),
    Buffer.from(bytes),
  );
  const request = (value) =>
    JSON.parse(session.request(summary.documentId, JSON.stringify(value)));
  const roots = request({
    op: "children",
    parent: null,
    offset: 0,
    limit: 200,
  });
  const fields = request({
    op: "children",
    parent: roots.nodes[0].id,
    offset: 0,
    limit: 200,
  });
  const field = fields.nodes.find((node) => node.kind === "bool");
  assert.ok(field, "Expected a boolean field for the edit probe");
  const edited = request({
    op: "transact",
    revision: summary.revision,
    operations: [
      {
        op: "set",
        node: field.id,
        tag: field.tag,
        value: field.value === "true" ? "false" : "true",
      },
    ],
  }).summary;
  assert.equal(edited.encodedBytes, null, "Edits must not serialize to calculate size");
  assert.equal(
    Buffer.from(session.export(summary.documentId)).filter(
      (byte, index) => byte !== bytes[index],
    ).length,
    1,
  );
  let restored = request({ op: "undo", revision: edited.revision }).summary;
  assert.equal(restored.encodedBytes, null);

  const general = request({ op: "general" });
  const health = general.find((entry) => entry.key === "hp" && !entry.error);
  if (health) {
    const expected = String(Number(health.value) + 1);
    const changed = request({
      op: "transact",
      revision: restored.revision,
      operations: [{ op: "general", values: { hp: expected } }],
    }).summary;
    const editedBytes = session.export(summary.documentId);
    const reopened = new SaveSession();
    try {
      const reopenedSummary = JSON.parse(reopened.open(editedBytes));
      const reopenedGeneral = JSON.parse(
        reopened.request(
          reopenedSummary.documentId,
          JSON.stringify({ op: "general" }),
        ),
      );
      assert.equal(
        reopenedGeneral.find((entry) => entry.key === "hp").value,
        expected,
      );
    } finally {
      reopened.free();
    }
    restored = request({ op: "undo", revision: changed.revision }).summary;
  }

  assert.deepEqual(
    Buffer.from(session.export(summary.documentId)),
    Buffer.from(bytes),
  );
  assert.throws(() => session.open(Uint8Array.of(255)));
  assert.deepEqual(
    Buffer.from(session.export(summary.documentId)),
    Buffer.from(bytes),
  );
  console.log(
    `WASM: exact round trip, reversible edit and edited reopen passed (${summary.originalBytes} bytes; ${summary.records} records).`,
  );
} finally {
  session.free();
}
