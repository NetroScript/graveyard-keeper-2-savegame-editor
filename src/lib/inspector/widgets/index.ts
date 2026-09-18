import type { Component } from "svelte";
import type { NodeView, Operation } from "../../save-api";
export interface WidgetProps {
  node: NodeView;
  children: NodeView[];
  transact: (operations: Operation[]) => Promise<void>;
}
export interface TreeBadge {
  label?: string;
  value: string;
  tone?: "type" | "key" | "value";
}
export interface TreePresentation {
  typeLabel?: string;
  badges: TreeBadge[];
  compact?: boolean;
}
export interface Registration {
  id: string;
  priority: number;
  matches?: (node: NodeView, children: NodeView[]) => boolean;
  component?: Component<WidgetProps>;
  tree?: (node: NodeView) => TreePresentation | undefined;
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
    .filter((r) => r.component && r.matches && r.matches(node, children))
    .sort((a, b) => b.priority - a.priority);
  return matches.length &&
    (matches.length === 1 || matches[0].priority !== matches[1].priority)
    ? matches[0]
    : undefined;
}
export function matchTree(node: NodeView) {
  const matches = registrations
    .map((registration) => ({
      registration,
      presentation: registration.tree?.(node),
    }))
    .filter((match) => match.presentation)
    .sort((a, b) => b.registration.priority - a.registration.priority);
  return matches.length &&
    (matches.length === 1 ||
      matches[0].registration.priority !== matches[1].registration.priority)
    ? matches[0].presentation
    : undefined;
}
