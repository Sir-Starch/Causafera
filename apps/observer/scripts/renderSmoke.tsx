/**
 * Render smoke check.
 *
 * Seeds the observer session store with a captured runtime session and renders every area,
 * every inspector dock, both locales, with and without a selection. It catches the whole
 * class of failures that type checking cannot: a selector reading a field that is not there,
 * a reducer that divides by an empty window, a branch that only appears once real data has an
 * unusual shape.
 *
 * The capture holds real protocol bytes; run
 * `cargo run -p causafera-observer --example capture_replay -- apps/observer/dev/replay/capture.json`
 * to produce it. Without a capture the check reports that and exits successfully, so it never
 * blocks a workspace that has not built the Rust side.
 */

import { existsSync, readFileSync } from "node:fs";
import { createElement, type ComponentType } from "react";
import { renderToString } from "react-dom/server";

import {
  decodeConnectResponse,
  decodeExplanationReport,
  decodeFieldRaster,
  decodeQueryResponse,
  decodeRuntimeSummary,
  decodeStreamEnvelope,
  decodeWorldChunkSnapshot,
  type FieldRaster,
} from "@causafera/observer-protocol";

import { AssayArea, AssayDock } from "../src/areas/AssayArea";
import { FluxArea, FluxDock } from "../src/areas/FluxArea";
import { InstrumentArea, InstrumentDock } from "../src/areas/InstrumentArea";
import { StationArea, StationDock } from "../src/areas/StationArea";
import { ChartArea, ChartDock } from "../src/areas/ChartArea";
import { Unattached } from "../src/components/Unattached";
import { session } from "../src/observer/instance";
import { AREA_IDS, type AreaId, type AreaProps } from "../src/workspace";

interface Capture {
  seed: number;
  connect: number[];
  frames: {
    ticks: number;
    runtime: number[];
    world: number[];
    rasters?: Record<string, number[]>;
  }[];
  explanation: number[];
}

const capturePath = process.argv[2] ?? "dev/replay/capture.json";
if (!existsSync(capturePath)) {
  process.stdout.write(
    `render-smoke: no capture at ${capturePath}; run the capture_replay example to enable this check.\n`,
  );
  process.exit(0);
}

const capture = JSON.parse(readFileSync(capturePath, "utf8")) as Capture;
const bytes = (values: number[]) => Uint8Array.from(values);

const summaries = capture.frames.map((frame) =>
  decodeRuntimeSummary(decodeStreamEnvelope(bytes(frame.runtime)).payload),
);
const latest = capture.frames[capture.frames.length - 1]!;
const world = decodeWorldChunkSnapshot(decodeQueryResponse(bytes(latest.world)).payload);
const explanation = decodeExplanationReport(
  decodeQueryResponse(bytes(capture.explanation)).payload,
);
const connect = decodeConnectResponse(bytes(capture.connect));
const current = summaries[summaries.length - 1]!;

// The capture keys its rasters exactly as the session keys its cache, so the
// check renders against the per-cell lattices a real session would hold.
const rasters = new Map<string, FieldRaster>();
for (const frame of [capture.frames[0], latest]) {
  for (const [key, encoded] of Object.entries(frame?.rasters ?? {})) {
    const response = decodeQueryResponse(bytes(encoded));
    if (response.payload.length > 0) rasters.set(key, decodeFieldRaster(response.payload));
  }
}

session.store.patch({
  connection: "connected",
  transportLabel: `replay:${capture.seed}`,
  replaying: true,
  negotiation: {
    protocolVersion: connect.selectedProtocolVersion,
    capabilities: connect.capabilities,
    timeAtConnect: connect.currentTime,
  },
  summary: current,
  previous: summaries[summaries.length - 2],
  history: summaries,
  world,
  worldTicks: world.simulationTicks,
  rasters,
  explanation,
  explanationTicks: current.simulationTicks,
  seed: capture.seed,
  exchanges: [
    {
      id: 1,
      command: "observer_connect",
      requestBytes: capture.connect.length,
      responseBytes: 40,
      durationMs: 1.2,
      outcome: "ok",
      at: Date.now(),
    },
    {
      id: 2,
      command: "observer_query",
      detail: "WorldChunks",
      requestBytes: 6,
      responseBytes: latest.world.length,
      durationMs: 3.4,
      outcome: "ok",
      at: Date.now(),
    },
  ],
});

type AreaComponent = ComponentType<AreaProps>;

const AREAS: Record<AreaId, { view: AreaComponent; dock: AreaComponent }> = {
  station: { view: StationArea, dock: StationDock },
  chart: { view: ChartArea, dock: ChartDock },
  flux: { view: FluxArea, dock: FluxDock },
  assay: { view: AssayArea, dock: AssayDock },
  instrument: { view: InstrumentArea, dock: InstrumentDock },
};

const firstChunk = world.chunks[0];
const firstDelta = world.materialSurfaceDeltas[0];
const selections: Partial<AreaProps["workspace"]>[] = [
  {},
  {
    selectedChunk:
      firstChunk === undefined
        ? undefined
        : `${firstChunk.chartId}:${firstChunk.chunkX}:${firstChunk.chunkY}:${firstChunk.chunkZ}`,
    selectedSurface:
      firstDelta === undefined
        ? undefined
        : `${firstDelta.chartId}:${firstDelta.chunkX}:${firstDelta.chunkY}:${firstDelta.chunkZ}#${firstDelta.cellOrdinal}`,
    traceFilter: firstDelta?.contactTraceId,
  },
];

let checks = 0;
let failures = 0;

function check(name: string, render: () => string): void {
  checks += 1;
  try {
    const html = render();
    if (html.length === 0) throw new Error("rendered nothing");
    process.stdout.write(`  ok   ${name} (${html.length} chars)\n`);
  } catch (error) {
    failures += 1;
    process.stdout.write(`  FAIL ${name}\n${(error as Error).stack}\n`);
  }
}

for (const area of AREA_IDS) {
  for (const locale of ["ru-RU", "en-US"] as const) {
    session.store.patch({ locale });
    for (const [index, selection] of selections.entries()) {
      const props: AreaProps = {
        workspace: {
          area,
          dockCollapsed: false,
          railCollapsed: false,
          railWidth: 232,
          dockWidth: 344,
          primaryLens: "relief",
          overlayLenses: [
            "population",
            "surface",
            "contours",
            "mana-isolines",
            "mana-provenance",
            "mana-gradient",
            "trace-anchors",
          ],
          ...selection,
        },
        update: () => {},
        goTo: () => {},
      };
      check(`${area}/view  ${locale} selection=${index}`, () =>
        renderToString(createElement(AREAS[area].view, props)),
      );
      check(`${area}/dock  ${locale} selection=${index}`, () =>
        renderToString(createElement(AREAS[area].dock, props)),
      );
    }
  }
}

// The raster lenses, with the lattices a session would hold and then without
// them: a lens that draws a measured surface must also render before one has
// arrived, because that is the state every session starts in.
for (const primaryLens of ["mana", "mana-peak", "relief", "roughness"]) {
  for (const held of [true, false]) {
    session.store.patch({ rasters: held ? rasters : new Map() });
    const props: AreaProps = {
      workspace: {
        area: "chart",
        dockCollapsed: false,
        railCollapsed: false,
        railWidth: 232,
        dockWidth: 344,
        primaryLens,
        overlayLenses: ["contours", "mana-isolines", "mana-provenance"],
        traceFilter: world.materialSurfaceDeltas[0]?.localManaTransitionTraceId,
      },
      update: () => {},
      goTo: () => {},
    };
    check(`chart/view  ${primaryLens} rasters=${held}`, () =>
      renderToString(createElement(ChartArea, props)),
    );
  }
}
session.store.patch({ rasters });

// The unattached state must render without any observer payload at all.
session.store.patch({
  connection: "unavailable",
  summary: undefined,
  world: undefined,
  explanation: undefined,
  rasters: new Map(),
  history: [],
});
check("unattached", () => renderToString(createElement(Unattached)));

process.stdout.write(
  failures === 0
    ? `render-smoke: ${checks} checks passed\n`
    : `render-smoke: ${failures} of ${checks} checks failed\n`,
);
process.exit(failures === 0 ? 0 : 1);
