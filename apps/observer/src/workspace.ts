/** Workspace state shared between an area and its inspector dock. */

import { DEFAULT_OVERLAYS, DEFAULT_PRIMARY_LENS } from "./map/lenses";

export const AREA_IDS = ["station", "chart", "flux", "assay", "instrument"] as const;

export type AreaId = (typeof AREA_IDS)[number];

export interface WorkspaceState {
  area: AreaId;
  /** Both side panels collapse to a strip that keeps its own expand control. */
  dockCollapsed: boolean;
  railCollapsed: boolean;
  /** Panel widths, in pixels, set by their drag handles. */
  railWidth: number;
  dockWidth: number;
  /** Chunk selected on the map, the chart profile or in the register. */
  selectedChunk?: string;
  /** Cell selected inside a chunk, once the map is zoomed to the lattice. */
  selectedCell?: { chunkKey: string; x: number; y: number };
  /** The base field drawn by the map. */
  primaryLens: string;
  /** Lenses drawn over the base field. */
  overlayLenses: string[];
  /** Which face of the chart inspector is showing. */
  dockView?: "selection" | "catalogue";
  /** Material surface selected on the condition ladder. */
  selectedSurface?: string;
  /** A trace anchor being followed across the ledger and the claim list. */
  traceFilter?: bigint;
}

export const INITIAL_WORKSPACE: WorkspaceState = {
  area: "station",
  dockCollapsed: false,
  railCollapsed: false,
  railWidth: 232,
  dockWidth: 344,
  primaryLens: DEFAULT_PRIMARY_LENS,
  overlayLenses: [...DEFAULT_OVERLAYS],
};

export interface AreaProps {
  workspace: WorkspaceState;
  update(patch: Partial<WorkspaceState>): void;
  goTo(area: AreaId): void;
}
