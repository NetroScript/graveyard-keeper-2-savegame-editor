import type { Component } from "svelte";
import type { NodeView, Operation } from "../../save-api";
export interface WidgetProps {
  node: NodeView;
  children: NodeView[];
  transact: (operations: Operation[]) => Promise<void>;
}
export interface Registration {
  id: string;
  priority: number;
  matches: (node: NodeView, children: NodeView[]) => boolean;
  component: Component<WidgetProps>;
}
const modules = import.meta.glob<{ default: Registration }>(
  "./*.registration.ts",
  { eager: true },
);
export const registrations = Object.values(modules).map(
  (module) => module.default,
);
export function matchWidget(node: NodeView, children: NodeView[]) {
  const matches = registrations
    .filter((r) => r.matches(node, children))
    .sort((a, b) => b.priority - a.priority);
  return matches.length &&
    (matches.length === 1 || matches[0].priority !== matches[1].priority)
    ? matches[0]
    : undefined;
}
