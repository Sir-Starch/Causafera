/** Workspace state shared between an area and its inspector dock. */

export const AREA_IDS = ["station", "survey", "flux", "assay", "instrument"] as const;

export type AreaId = (typeof AREA_IDS)[number];

export interface WorkspaceState {
  area: AreaId;
  dockOpen: boolean;
  railCollapsed: boolean;
  /** Chunk selected on the chart profile or in the register. */
  selectedChunk?: string;
  /** Material surface selected on the condition ladder. */
  selectedSurface?: string;
  /** A trace anchor being followed across the ledger and the claim list. */
  traceFilter?: bigint;
}

export const INITIAL_WORKSPACE: WorkspaceState = {
  area: "station",
  dockOpen: true,
  railCollapsed: false,
};

export interface AreaProps {
  workspace: WorkspaceState;
  update(patch: Partial<WorkspaceState>): void;
  goTo(area: AreaId): void;
}
