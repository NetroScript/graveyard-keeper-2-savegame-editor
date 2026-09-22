export interface WorldDrop {
  node: number;
  source: "Dropped" | "Queued";
  type: string;
  id: string;
  count: string;
  location: {
    world: string;
    x: string;
    y: string;
    z: string;
  };
}

export interface DropSnapshot {
  drops: WorldDrop[];
}
