import Vector from "./Vector.svelte";
import type { Registration } from "./index";
export default {
  id: "unity-vector",
  priority: 10,
  component: Vector,
  matches(node, children) {
    const type = node.typeName?.match(
      /^UnityEngine\.(Vector([234])(Int)?), UnityEngine(?:\.CoreModule)?(?:,|$)/,
    );
    if (!type || children.length !== Number(type[2])) return false;
    if (type[3] && type[2] === "4") return false;
    const names = ["x", "y", "z", "w"];
    return children.every(
      (child, i) =>
        (child.name === null ||
          child.name === names[i] ||
          child.name === `m_${names[i]}`) &&
        child.tag === (type[3] ? (child.name ? 23 : 24) : child.name ? 31 : 32),
    );
  },
} satisfies Registration;
