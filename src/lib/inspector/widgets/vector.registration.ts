import Vector from "./Vector.svelte";
import type { Registration } from "./index";
import type { NodeView, TreeField } from "../../save-api";

function vectorType(node: NodeView) {
  return node.typeName?.match(
    /^UnityEngine\.(Vector([234])(Int)?), UnityEngine(?:\.CoreModule)?(?:,|$)/,
  );
}

function validComponents(
  node: NodeView,
  children: Pick<TreeField, "name" | "tag">[],
) {
  const type = vectorType(node);
  if (!type || children.length !== Number(type[2])) return undefined;
  if (type[3] && type[2] === "4") return undefined;
  const names = ["x", "y", "z", "w"];
  return children.every(
    (child, i) =>
      (child.name === null ||
        child.name === names[i] ||
        child.name === `m_${names[i]}`) &&
      child.tag === (type[3] ? (child.name ? 23 : 24) : child.name ? 31 : 32),
  )
    ? type
    : undefined;
}
export default {
  id: "unity-vector",
  priority: 100,
  component: Vector,
  matches(node, children) {
    return Boolean(validComponents(node, children));
  },
  tree(node) {
    const type = validComponents(node, node.treeFields);
    if (!type) return undefined;
    return {
      typeLabel: type[1],
      compact: true,
      badges: node.treeFields.map((field, index) => ({
        label: ["x", "y", "z", "w"][index],
        value: field.value,
        tone: "value",
      })),
    };
  },
} satisfies Registration;
