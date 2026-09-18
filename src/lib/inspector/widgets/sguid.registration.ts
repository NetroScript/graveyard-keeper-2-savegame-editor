import type { NodeView } from "../../save-api";
import SGuid from "./SGuid.svelte";
import type { Registration } from "./index";

function isSGuid(node: NodeView) {
  return /^SGuid, Assembly-CSharp(?:,|$)/.test(node.typeName ?? "");
}

function idField(node: NodeView) {
  const fields = node.treeFields.filter(
    (field) =>
      field.name === "id" && field.kind === "string" && field.tag === 39,
  );
  return fields.length === 1 ? fields[0] : undefined;
}

export default {
  id: "sguid",
  priority: 90,
  component: SGuid,
  matches(node, children) {
    return (
      isSGuid(node) &&
      children.filter(
        (child) =>
          child.name === "id" && child.kind === "string" && child.tag === 39,
      ).length === 1
    );
  },
  tree(node) {
    const id = idField(node);
    if (!isSGuid(node) || !id) return undefined;
    return {
      typeLabel: "SGuid",
      compact: true,
      badges: [{ value: id.value, tone: "key" }],
    };
  },
} satisfies Registration;
