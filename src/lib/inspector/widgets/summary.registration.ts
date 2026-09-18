import type { NodeView, TreeField } from "../../save-api";
import type { Registration, TreeBadge } from "./index";

function className(node: NodeView) {
  return node.typeName?.split(",")[0].split(".").pop() ?? "";
}

function fieldsByName(node: NodeView) {
  return new Map(
    node.treeFields
      .filter((field) => field.name)
      .map((field) => [field.name!.toLowerCase(), field]),
  );
}

function badge(
  field: TreeField | undefined,
  label: string,
  tone: TreeBadge["tone"] = "key",
) {
  return field ? { label, value: field.value, tone } : undefined;
}

export default {
  id: "game-object-summary",
  priority: 20,
  tree(node) {
    const type = className(node);
    const fields = fieldsByName(node);
    const id = fields.get("id") ?? fields.get("wgoid");
    const badges: (TreeBadge | undefined)[] = [];

    if (type.endsWith("WgoData")) {
      if (node.value) badges.push({ value: node.value, tone: "type" });
      badges.push(badge(id, "id"));
    } else if (type === "GameSceneData") {
      badges.push(badge(id, "id"));
    } else if (type === "Item") {
      badges.push(
        badge(id, "id"),
        badge(fields.get("count"), "count", "value"),
      );
    } else if (type === "GameResAtom") {
      badges.push(
        badge(fields.get("type"), "type"),
        badge(fields.get("value"), "value", "value"),
      );
    } else if (id) {
      badges.push(badge(id, "id"));
    } else {
      return undefined;
    }

    return {
      typeLabel: type || undefined,
      badges: badges.filter((entry): entry is TreeBadge => Boolean(entry)),
    };
  },
} satisfies Registration;
