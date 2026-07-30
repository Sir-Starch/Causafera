/** Canonical observer v1 client codec. Field numbers come from proto/causafera/observer/v1. */
export const OBSERVER_PROTOCOL_V1 = 1;
export const MAX_MATERIAL_SURFACE_DELTAS = 64;
export const MATERIAL_SURFACE_DELTA_SCHEMA_V3 = 3;
export const MATERIAL_SURFACE_DELTA_SCHEMA_V4 = 4;
export const MAX_THERMAL_DELTAS = 64;
export const THERMAL_DELTA_SCHEMA_V1 = 1;
/**
 * Zero is not a hydrology summary version: it is what a payload written before
 * hydrology existed decodes to, and it means "no hydrology evidence in this
 * payload" rather than "a session holding no water".
 */
export const HYDROLOGY_SUMMARY_SCHEMA_ABSENT = 0;
export const HYDROLOGY_SUMMARY_SCHEMA_V1 = 1;
export const HYDROLOGY_DELTA_SCHEMA_V1 = 1;
export const HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1 = 1;
export const HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1 = 1;
export const HYDROLOGY_RASTER_VALUES_SCHEMA_V1 = 1;
export const MAX_HYDROLOGY_DELTAS = 64;
export const MAX_HYDROLOGY_TRANSFER_SUMMARIES = 64;
export const MAX_HYDROLOGY_CONVEYANCE_SUMMARIES = 64;
/**
 * The response cap, distinct from the request cap. One bounds what a peer may
 * ask the runtime to parse; this one bounds what a client must be willing to
 * allocate.
 */
export const MAX_QUERY_RESPONSE_PAYLOAD_BYTES = 1 << 20;

export interface ConnectRequest {
  supportedProtocolVersions: number[];
  observerLocale: string;
}

export interface ConnectResponse {
  selectedProtocolVersion: number;
  currentTime: bigint;
  capabilities: number[];
}

export enum QueryKind {
  RuntimeSummary = 1,
  ExplanationIr = 2,
  WorldChunks = 3,
  FieldRaster = 4,
}

export enum FieldRasterKind {
  TerrainElevation = 1,
  TerrainRoughness = 2,
  ManaIntensity = 3,
  HydrologySurfaceWater = 4,
  HydrologySoilWater = 5,
  HydrologyGroundwater = 6,
}

/**
 * Whether a lattice carries unsigned water volumes.
 *
 * The distinction is not cosmetic. A water volume is a `u64` and half of its
 * range has no image in the signed `values` band, so the two bands are mutually
 * exclusive: a hydrology raster with signed values, or any other raster with
 * unsigned ones, describes a lattice neither producer writes.
 */
export function carriesUnsignedValues(field: FieldRasterKind): boolean {
  return (
    field === FieldRasterKind.HydrologySurfaceWater ||
    field === FieldRasterKind.HydrologySoilWater ||
    field === FieldRasterKind.HydrologyGroundwater
  );
}

/** The coarsest terrain reduction the projection offers; level 0 is 32 x 32. */
export const MAX_FIELD_RASTER_DETAIL_LEVEL = 2;

export interface FieldRasterRequest {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  field: FieldRasterKind;
  detailLevel: number;
}

/**
 * One chunk of one measured lattice.
 *
 * `values` is row-major over `edge` columns and `edge` rows, repeated `depth`
 * times through z. A surface field has depth 1; the mana volume has depth equal
 * to its edge, and the reduction to plan view is the reader's choice rather than
 * a property of the field, so the runtime performs none of it.
 */
export interface FieldRaster {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  field: FieldRasterKind;
  detailLevel: number;
  edge: number;
  depth: number;
  values: Float64Array;
  /** A second band over the same lattice: roughness under elevation, else empty. */
  auxiliary: Float64Array;
  /** Per-cell provenance, zero where the cell has never changed. */
  cellTraces: BigUint64Array;
  generationTraceId: bigint;
  /**
   * The lossless unsigned band, carried only by hydrology lattices.
   *
   * `BigUint64Array` rather than `Float64Array`: a water volume above
   * `Number.MAX_SAFE_INTEGER` is a state this engine reaches, and a double would
   * round it. Empty for every other field.
   */
  unsignedValues: BigUint64Array;
  /** `HYDROLOGY_RASTER_VALUES_SCHEMA_V1` when the band is present, else 0. */
  unsignedValuesSchemaVersion: number;
}

export const BOOTSTRAP_SUMMARY_SCHEMA_ABSENT = 0;
export const BOOTSTRAP_SUMMARY_SCHEMA_V1 = 1;
/** The current production bootstrap runs six stages; more is a record this build cannot produce. */
export const MAX_BOOTSTRAP_RECEIPT_SUMMARIES = 6;
export const MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES = 8;

export enum QueryStatus {
  Ok = 1,
  InvalidRequest = 2,
  Unsupported = 3,
  NotAvailable = 4,
}

export interface QueryResponse {
  requestId: bigint;
  protocolVersion: number;
  status: QueryStatus;
  payload: Uint8Array;
}

export interface RuntimeSummary {
  simulationTicks: bigint;
  digestSchemaVersion: number;
  physicalDigest: Uint8Array;
  historyDigest: Uint8Array;
  manaTotal: bigint;
  manaMaximum: bigint;
  activeChunkCount: number;
  resolutionRelevance: bigint;
  resolutionLevel: number;
  causalTraceCount: bigint;
  actorCount: number;
  populationTotal: bigint;
  physicalEvents: bigint;
  manaCellChanges: bigint;
  manaPhysicalEffects: bigint;
  resolutionTransitions: bigint;
  actorActionsCommitted: bigint;
  actorActionsRejected: bigint;
  populationBirths: bigint;
  populationDeaths: bigint;
  populationMovements: bigint;
  bytesPerChunk: bigint;
  latestTraceId: bigint;
  thermalTotalCellEnergy: bigint;
  thermalTotalReservoirBudget: bigint;
  thermalActiveChunkCount: number;
  thermalActiveCellCount: number;
  bootstrap: BootstrapSummary;
  hydrology: HydrologySummary;
}

/**
 * The bounded whole-session water summary.
 *
 * `schemaVersion === HYDROLOGY_SUMMARY_SCHEMA_ABSENT` means the payload was
 * written before hydrology existed, not that the world holds no water — a build
 * that has hydrology writes the group even when the domain is disabled.
 */
export interface HydrologySummary {
  schemaVersion: number;
  totalSurface: bigint;
  totalSoil: bigint;
  totalGroundwater: bigint;
  totalConveyance: bigint;
  /** Exactly zero for every committed batch. */
  latestResidual: bigint;
  activeChunkCount: number;
  /** The greatest applied record, or null when none is still evidenced. */
  latestForcing: HydrologyForcing | null;
}

export interface HydrologyForcing {
  tick: bigint;
  forcingId: bigint;
  originTrace: bigint;
  /** Precipitation plus external inflow, as accepted. */
  acceptedSource: bigint;
  acceptedEvapotranspiration: bigint;
}

/**
 * The bounded, read-only projection of the canonical production bootstrap
 * record. It carries equality and trace anchors only: no runtime state, no
 * authoritative actor or place identity, and no rendered process names.
 *
 * `schemaVersion === BOOTSTRAP_SUMMARY_SCHEMA_ABSENT` means the payload was
 * written before this summary existed, not that the record is empty.
 */
export interface BootstrapSummary {
  schemaVersion: number;
  planId: bigint;
  worldSeed: bigint;
  stageCount: number;
  complete: boolean;
  configuredPopulation: bigint;
  configuredPromotionLimit: number;
  receipts: BootstrapReceipt[];
  /**
   * The appended hydrology stage's receipt, when a session ran one.
   *
   * Carried separately from `receipts`: the six-summary bound, `stageCount`, and
   * `complete` are frozen V1 contract, and a seventh entry would change what an
   * existing consumer reads. A frozen decoder skips field 48 entirely, which is
   * what makes this additive.
   */
  stageSeven: BootstrapReceipt | null;
}

export interface BootstrapReceipt {
  stage: bigint;
  completedAt: bigint;
  result: Uint8Array;
  completionTrace: bigint;
  dependencyTraces: bigint[];
}

export interface WorldChunkSnapshot {
  simulationTicks: bigint;
  chunks: SpatialChunkSummary[];
  materialSurfaceDeltaSchemaVersion: number;
  materialSurfaceDeltas: MaterialSurfaceDelta[];
  materialSurfaceGateDeltas: MaterialSurfaceGateDelta[];
  materialSurfaceThermalDeltas: MaterialSurfaceThermalDelta[];
  thermalDeltaSchemaVersion: number;
  thermalDeltas: ThermalFieldDelta[];
  hydrologyDeltas: HydrologyCellDelta[];
  hydrologyDeltaSchemaVersion: number;
  hydrologyTransferSummaries: HydrologyTransferSummary[];
  hydrologyTransferSchemaVersion: number;
  hydrologyConveyanceSummaries: HydrologyConveyanceSummary[];
  hydrologyConveyanceSchemaVersion: number;
}

/** One cell's committed storage change over one tick. */
export interface HydrologyCellDelta {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  surfaceBefore: bigint;
  surfaceAfter: bigint;
  soilBefore: bigint;
  soilAfter: bigint;
  groundwaterBefore: bigint;
  groundwaterAfter: bigint;
  /** Signed: precipitation and inflow positive, evapotranspiration not. */
  netForcing: bigint;
  netLateralFlow: bigint;
  transitionTraceId: bigint;
  conservationTraceId: bigint;
  transitionTick: bigint;
}

/**
 * One accepted, partly accepted, or wholly rejected movement of water. All three
 * volumes travel because a limiter that engaged is evidence.
 */
export interface HydrologyTransferSummary {
  processKind: number;
  /** The canonical carrier-key encoding, opaque at this boundary. */
  sourceKey: Uint8Array;
  targetKey: Uint8Array;
  requestedVolume: bigint;
  acceptedVolume: bigint;
  unacceptedVolume: bigint;
  transferTraceId: bigint;
  conservationTraceId: bigint;
  tick: bigint;
  forcingOriginTraceId: bigint | null;
}

/** One conveyance edge's current storage and this tick's accepted exchange. */
export interface HydrologyConveyanceSummary {
  edgeKey: Uint8Array;
  storage: bigint;
  capacity: bigint;
  acceptedInflow: bigint;
  acceptedRelease: bigint;
  lastChangeTraceId: bigint;
  tick: bigint;
}

export interface MaterialSurfaceDelta {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  beforeCondition: bigint;
  afterCondition: bigint;
  manaTotal: bigint;
  contactTraceId?: bigint;
  manaEffectTraceId?: bigint;
  transitionTick: bigint;
  manaTransitionTraceId?: bigint;
  manaBefore?: bigint;
  manaAfter?: bigint;
  localManaBefore?: bigint;
  localManaAfter?: bigint;
  localManaTransitionTraceId?: bigint;
}

export interface MaterialSurfaceGateDelta {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  beforeActive: boolean;
  afterActive: boolean;
  localManaBefore: bigint;
  localManaAfter: bigint;
  localManaTransitionTraceId: bigint;
  gateTransitionTraceId: bigint;
  contactTraceId?: bigint;
  transitionTick: bigint;
}

export interface ThermalFieldDelta {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  preStateEnergy: bigint;
  postStateEnergy: bigint;
  reservoirScheduledInjection: bigint;
  reservoirAcceptedInjection: bigint;
  reservoirRejectedInjection: bigint;
  netFaceFlux: bigint;
  faceCount: number;
}

/**
 * A material surface's retained-heat exchange with its co-located thermal cell
 * (TODO-THERMAL-002), gated by materialSurfaceDeltaSchemaVersion >= 4 - a distinct
 * addressed-object family from MaterialSurfaceDelta's condition/mana pair, even
 * though both are keyed by the same surface.
 */
export interface MaterialSurfaceThermalDelta {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  cellOrdinal: number;
  beforeRetained: bigint;
  afterRetained: bigint;
  cellPreState: bigint;
  signedFlux: bigint;
  thermalExchangeTraceId: bigint;
  transitionTick: bigint;
}

export interface SpatialChunkSummary {
  chartId: bigint;
  chunkX: number;
  chunkY: number;
  chunkZ: number;
  minimumElevationMm: number;
  maximumElevationMm: number;
  meanRoughnessMm: number;
  manaTotal: bigint;
  resolutionRelevance: bigint;
  resolutionLevel: number;
  populationTotal: bigint;
  causalEventCount: bigint;
  latestTraceId: bigint;
}

export enum StreamKind {
  RuntimeSummary = 1,
  Explanation = 2,
  Metrics = 3,
}

export interface StreamEnvelope {
  header: {
    streamId: bigint;
    schemaVersion: number;
    sequenceNumber: bigint;
    simulationTime: bigint;
    physicalDigest: Uint8Array;
    historyDigest: Uint8Array;
    isSnapshot: boolean;
  };
  kind: StreamKind;
  chunkId?: bigint;
  payload: Uint8Array;
}

export enum EvidenceState {
  Supported = 1,
  Unsupported = 2,
  Unknown = 3,
}

export enum Assessment {
  Supported = 1,
  Partial = 2,
  Unsupported = 3,
  Unknown = 4,
}

export type NumericClaimValue =
  | { kind: "scalar"; value: bigint }
  | { kind: "range"; start: bigint; end: bigint }
  | { kind: "ratio"; numerator: bigint; denominator: bigint };

export interface ExplanationClaim {
  schemaId: bigint;
  value: NumericClaimValue;
  confidence: number;
  evidenceTraceIds: bigint[];
  comparison: { kind: number; cohortId?: bigint };
  evidenceState: EvidenceState;
}

export interface ExplanationFrame {
  checkpointTicks: bigint;
  claims: ExplanationClaim[];
  overallAssessment: Assessment;
}

export interface ExplanationReport {
  experimentId: bigint;
  frames: ExplanationFrame[];
  overallAssessment: Assessment;
}

export function encodeConnectRequest(request: ConnectRequest): Uint8Array {
  const output: number[] = [];
  for (const version of request.supportedProtocolVersions) {
    fieldVarint(output, 1, BigInt(version));
  }
  if (request.observerLocale.length > 0) {
    fieldBytes(output, 2, new TextEncoder().encode(request.observerLocale));
  }
  return Uint8Array.from(output);
}

export function decodeConnectResponse(input: Uint8Array): ConnectResponse {
  const cursor = new Cursor(input);
  let selectedProtocolVersion: number | undefined;
  let currentTime: bigint | undefined;
  const capabilities: number[] = [];
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) selectedProtocolVersion = Number(cursor.varint());
    else if (field === 2 && wire === 2) currentTime = decodeSimulationTime(cursor.bytes());
    else if (field === 3 && wire === 0) capabilities.push(Number(cursor.varint()));
    else cursor.skip(wire);
  }
  if (selectedProtocolVersion === undefined || currentTime === undefined) {
    throw new Error("incomplete observer connect response");
  }
  return { selectedProtocolVersion, currentTime, capabilities };
}

export function encodeQuery(kind: QueryKind, requestId: bigint): Uint8Array {
  const output: number[] = [];
  fieldVarint(output, 1, requestId);
  fieldVarint(output, 2, BigInt(OBSERVER_PROTOCOL_V1));
  fieldVarint(output, 3, BigInt(kind));
  return Uint8Array.from(output);
}

export function encodeRuntimeSummaryQuery(requestId: bigint): Uint8Array {
  return encodeQuery(QueryKind.RuntimeSummary, requestId);
}

/** A raster asks for one chunk of one field, so its parameters ride the payload. */
export function encodeFieldRasterQuery(
  requestId: bigint,
  request: FieldRasterRequest,
): Uint8Array {
  const payload: number[] = [];
  fieldVarint(payload, 1, request.chartId);
  fieldVarint(payload, 2, zigzagEncode(BigInt(request.chunkX)));
  fieldVarint(payload, 3, zigzagEncode(BigInt(request.chunkY)));
  fieldVarint(payload, 4, zigzagEncode(BigInt(request.chunkZ)));
  fieldVarint(payload, 5, BigInt(request.field));
  fieldVarint(payload, 6, BigInt(request.detailLevel));

  const output: number[] = [];
  fieldVarint(output, 1, requestId);
  fieldVarint(output, 2, BigInt(OBSERVER_PROTOCOL_V1));
  fieldVarint(output, 3, BigInt(QueryKind.FieldRaster));
  fieldBytes(output, 5, Uint8Array.from(payload));
  return Uint8Array.from(output);
}

export function decodeFieldRaster(input: Uint8Array): FieldRaster {
  const cursor = new Cursor(input);
  let chartId: bigint | undefined;
  let chunkX = 0;
  let chunkY = 0;
  let chunkZ = 0;
  let field: FieldRasterKind | undefined;
  let detailLevel = 0;
  let edge: number | undefined;
  let depth: number | undefined;
  let values: FieldRaster["values"] = new Float64Array();
  let auxiliary: FieldRaster["auxiliary"] = new Float64Array();
  let cellTraces: FieldRaster["cellTraces"] = new BigUint64Array();
  let generationTraceId = 0n;
  let unsignedValues: FieldRaster["unsignedValues"] = new BigUint64Array();
  let unsignedValuesSchemaVersion = 0;
  let unsignedSeen = false;
  let unsignedSchemaSeen = false;
  while (!cursor.empty) {
    const [number, wire] = cursor.key();
    if (number === 1 && wire === 0) chartId = cursor.varint();
    else if (number === 2 && wire === 0) chunkX = Number(zigzagDecode(cursor.varint()));
    else if (number === 3 && wire === 0) chunkY = Number(zigzagDecode(cursor.varint()));
    else if (number === 4 && wire === 0) chunkZ = Number(zigzagDecode(cursor.varint()));
    else if (number === 5 && wire === 0) field = Number(cursor.varint()) as FieldRasterKind;
    else if (number === 6 && wire === 0) detailLevel = Number(cursor.varint());
    else if (number === 7 && wire === 0) edge = Number(cursor.varint());
    else if (number === 8 && wire === 0) depth = Number(cursor.varint());
    else if (number === 9 && wire === 2) values = decodeDeltaBand(cursor.bytes());
    else if (number === 10 && wire === 2) auxiliary = decodeDeltaBand(cursor.bytes());
    else if (number === 11 && wire === 2) cellTraces = decodeTraceBand(cursor.bytes());
    else if (number === 12 && wire === 0) generationTraceId = cursor.varint();
    else if (number === 13 && wire === 2) {
      if (unsignedSeen) throw new Error("duplicate field raster field 13");
      unsignedSeen = true;
      unsignedValues = decodeUnsignedBand(cursor.bytes());
    }
    else if (number === 14 && wire === 0) {
      if (unsignedSchemaSeen) throw new Error("duplicate field raster field 14");
      unsignedSchemaSeen = true;
      unsignedValuesSchemaVersion = requireU32(cursor.varint(), 14);
    }
    else if (number === 13 || number === 14) {
      throw new Error(`unexpected wire type for field raster field ${number}`);
    }
    else cursor.skip(wire);
  }
  if (chartId === undefined || field === undefined || edge === undefined || depth === undefined) {
    throw new Error("incomplete observer field raster");
  }
  const cells = edge * edge * depth;
  // The two bands are mutually exclusive, and which one a raster carries is
  // decided by its kind rather than by what arrived: a hydrology lattice in the
  // signed band would have rounded every volume past 2^53, and any other lattice
  // in the unsigned one would have lost every negative elevation.
  if (carriesUnsignedValues(field)) {
    if (!unsignedSchemaSeen) throw new Error("missing hydrology raster field 14");
    if (unsignedValuesSchemaVersion !== HYDROLOGY_RASTER_VALUES_SCHEMA_V1) {
      throw new Error(
        `unsupported hydrology raster values schema ${unsignedValuesSchemaVersion}`,
      );
    }
    if (values.length !== 0 || auxiliary.length !== 0) {
      throw new Error("a hydrology raster must not carry the signed bands");
    }
    if (unsignedValues.length !== cells) {
      throw new Error(
        `field raster declares ${cells} cells but carries ${unsignedValues.length}`,
      );
    }
  } else {
    if (unsignedSeen || unsignedSchemaSeen) {
      throw new Error("a non-hydrology raster must not carry the unsigned band");
    }
    // A lattice whose payload does not fill it cannot be drawn at real
    // positions, and a renderer must never be left to guess the shape.
    if (values.length !== cells) {
      throw new Error(`field raster declares ${cells} cells but carries ${values.length}`);
    }
  }
  if (auxiliary.length !== 0 && auxiliary.length !== cells) {
    throw new Error(`field raster auxiliary band carries ${auxiliary.length} of ${cells} cells`);
  }
  if (cellTraces.length !== 0 && cellTraces.length !== cells) {
    throw new Error(`field raster trace band carries ${cellTraces.length} of ${cells} cells`);
  }
  return {
    chartId,
    chunkX,
    chunkY,
    chunkZ,
    field,
    detailLevel,
    edge,
    depth,
    values,
    auxiliary,
    cellTraces,
    generationTraceId,
    unsignedValues,
    unsignedValuesSchemaVersion,
  };
}

/**
 * Successive differences along the scan order, undone with the same wrapping
 * arithmetic the encoder used so every 64-bit value round-trips exactly.
 *
 * The result is `Float64Array` because it feeds a renderer: elevation in
 * millimetres and mana intensity both sit far inside the exactly representable
 * range, and a typed array is what an image blit wants.
 */
function decodeDeltaBand(input: Uint8Array): Float64Array {
  const cursor = new Cursor(input);
  const decoded: number[] = [];
  let previous = 0n;
  while (!cursor.empty) {
    previous = BigInt.asIntN(64, previous + zigzagDecode(cursor.varint()));
    decoded.push(Number(previous));
  }
  return new Float64Array(decoded);
}

function decodeTraceBand(input: Uint8Array): BigUint64Array {
  const cursor = new Cursor(input);
  const decoded: bigint[] = [];
  while (!cursor.empty) decoded.push(cursor.varint());
  return new BigUint64Array(decoded);
}

export function decodeQueryResponse(input: Uint8Array): QueryResponse {
  const cursor = new Cursor(input);
  let requestId: bigint | undefined;
  let protocolVersion: number | undefined;
  let status: QueryStatus | undefined;
  let payload: Uint8Array = new Uint8Array();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) requestId = cursor.varint();
    else if (field === 2 && wire === 0) protocolVersion = Number(cursor.varint());
    else if (field === 3 && wire === 0) status = Number(cursor.varint()) as QueryStatus;
    else if (field === 4 && wire === 2) {
      // Bounded before it is handed on: the response cap exists so a client
      // never has to accept a payload it did not agree to receive.
      payload = cursor.bytes();
      if (payload.length > MAX_QUERY_RESPONSE_PAYLOAD_BYTES) {
        throw new Error("observer query response payload exceeds its cap");
      }
    }
    else cursor.skip(wire);
  }
  if (requestId === undefined || protocolVersion === undefined || status === undefined) {
    throw new Error("incomplete observer query response");
  }
  return { requestId, protocolVersion, status, payload };
}

export function decodeRuntimeSummary(input: Uint8Array): RuntimeSummary {
  const cursor = new Cursor(input);
  const values = new Map<number, bigint>();
  let physicalDigest: Uint8Array = new Uint8Array();
  let historyDigest: Uint8Array = new Uint8Array();
  let thermalTotalCellEnergy = 0n;
  let thermalTotalReservoirBudget = 0n;
  const bootstrapReceipts: BootstrapReceipt[] = [];
  let bootstrapStageSeven: BootstrapReceipt | null = null;
  // Fields 36..=47, each single-valued. Two different water totals in one
  // payload is a contradiction, not a later value winning.
  const hydrologyBytes = new Map<number, bigint>();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    // Checked before the generic varint branch, or field 35 on a varint wire
    // would be stored as a scalar and then silently dropped, while Rust rejects
    // it. Any bootstrap field on a wire type it does not use is malformed.
    if (field >= 28 && field <= 35 && wire !== bootstrapWireType(field)) {
      throw new Error(`observer bootstrap field ${field} has the wrong wire type`);
    }
    // A hydrology field on the wrong wire type is malformed, not unknown:
    // skipping it would let a summary whose every member is mistyped fall
    // through to "this payload predates hydrology".
    if (field >= 36 && field <= 47 && wire !== hydrologyWireType(field)) {
      throw new Error(`observer hydrology field ${field} has the wrong wire type`);
    }
    if (field >= 36 && field <= 47) {
      if (hydrologyBytes.has(field)) {
        throw new Error(`duplicate observer hydrology summary field ${field}`);
      }
      if (wire === 0) hydrologyBytes.set(field, cursor.varint());
      else if (field === 41) hydrologyBytes.set(field, decodeCanonicalZigzagI128(cursor.bytes()));
      else if (field === 47) hydrologyBytes.set(field, decodeCanonicalU64(cursor.bytes()));
      else hydrologyBytes.set(field, decodeCanonicalU128(cursor.bytes()));
      continue;
    }
    else if (wire === 0) {
      // The summary's scalars are single-valued. Elsewhere in this payload a
      // repeated field silently wins with its last value, which is a decision
      // this group does not inherit: two different stage counts in one payload
      // is a contradiction, not an update.
      if (field >= 28 && field <= 34 && values.has(field)) {
        throw new Error(`duplicate observer bootstrap summary field ${field}`);
      }
      values.set(field, cursor.varint());
    }
    else if (wire === 2 && field === 3) physicalDigest = cursor.bytes();
    else if (wire === 2 && field === 4) historyDigest = cursor.bytes();
    else if (wire === 2 && field === 24) thermalTotalCellEnergy = decodeZigzagI128(cursor.bytes());
    else if (wire === 2 && field === 25) thermalTotalReservoirBudget = decodeZigzagI128(cursor.bytes());
    else if (wire === 2 && field === 35) {
      // Bounded before the list grows: a payload claiming more receipts than
      // this build's bootstrap can produce is rejected, not truncated.
      if (bootstrapReceipts.length === MAX_BOOTSTRAP_RECEIPT_SUMMARIES) {
        throw new Error("observer bootstrap summary exceeds its receipt bound");
      }
      bootstrapReceipts.push(decodeBootstrapReceipt(cursor.bytes()));
    }
    else if (wire === 2 && field === 48) {
      // Separately bounded: exactly one appended stage, so a second occurrence
      // describes two seventh stages.
      if (bootstrapStageSeven !== null) {
        throw new Error("observer bootstrap summary repeats its appended stage");
      }
      bootstrapStageSeven = decodeBootstrapReceipt(cursor.bytes());
    }
    else cursor.skip(wire);
  }
  if (physicalDigest.length !== 32 || historyDigest.length !== 32) {
    throw new Error("observer digest must contain 32 bytes");
  }
  const value = requiredValue(values, "RuntimeSummary");
  return {
    simulationTicks: value(1),
    digestSchemaVersion: requireU32(value(2), 2),
    physicalDigest,
    historyDigest,
    manaTotal: BigInt.asIntN(64, value(5)),
    manaMaximum: BigInt.asIntN(64, value(6)),
    activeChunkCount: requireU32(value(7), 7),
    resolutionRelevance: BigInt.asIntN(64, value(8)),
    resolutionLevel: requireU32(value(9), 9),
    causalTraceCount: value(10),
    actorCount: requireU32(value(11), 11),
    populationTotal: value(12),
    physicalEvents: value(13),
    manaCellChanges: value(14),
    manaPhysicalEffects: value(15),
    resolutionTransitions: value(16),
    actorActionsCommitted: value(17),
    actorActionsRejected: value(18),
    populationBirths: value(19),
    populationDeaths: value(20),
    populationMovements: value(21),
    bytesPerChunk: value(22),
    latestTraceId: value(23),
    thermalTotalCellEnergy,
    thermalTotalReservoirBudget,
    thermalActiveChunkCount: requireU32(values.get(26) ?? 0n, 26),
    thermalActiveCellCount: requireU32(values.get(27) ?? 0n, 27),
    bootstrap: decodeBootstrapSummary(values, bootstrapReceipts, bootstrapStageSeven),
    hydrology: decodeHydrologySummary(hydrologyBytes),
  };
}

/** The one wire type each hydrology summary field is allowed to arrive on. */
function hydrologyWireType(field: number): number {
  // 36 and 42..=45 are varints; every other member is a length-delimited
  // canonical byte integer.
  return field === 36 || (field >= 42 && field <= 45) ? 0 : 2;
}

/**
 * Decode fields 36..=47 as one required group and one all-or-nothing subgroup.
 *
 * Absence is tolerated only wholesale: a payload carrying none of them was
 * written before hydrology existed. Every other incompleteness fails closed,
 * because a partially present group is not an older peer — it is a summary whose
 * missing halves would otherwise be filled in with zeroes and read as
 * measurements.
 */
function decodeHydrologySummary(values: Map<number, bigint>): HydrologySummary {
  const required = [36, 37, 38, 39, 40, 41, 42];
  const forcing = [43, 44, 45, 46, 47];
  const present = (field: number): boolean => values.has(field);
  if (!required.some(present)) {
    if (forcing.some(present)) {
      throw new Error("observer hydrology forcing group has no summary to attribute it to");
    }
    return {
      schemaVersion: HYDROLOGY_SUMMARY_SCHEMA_ABSENT,
      totalSurface: 0n,
      totalSoil: 0n,
      totalGroundwater: 0n,
      totalConveyance: 0n,
      latestResidual: 0n,
      activeChunkCount: 0,
      latestForcing: null,
    };
  }
  for (const field of required) {
    if (!present(field)) {
      throw new Error(`missing observer hydrology summary field ${field}`);
    }
  }
  const schemaVersion = requireU32(values.get(36)!, 36);
  if (schemaVersion !== HYDROLOGY_SUMMARY_SCHEMA_V1) {
    throw new Error(`unsupported observer hydrology summary schema ${schemaVersion}`);
  }
  let latestForcing: HydrologyForcing | null = null;
  if (forcing.every(present)) {
    latestForcing = {
      tick: values.get(43)!,
      forcingId: values.get(44)!,
      originTrace: values.get(45)!,
      acceptedSource: values.get(46)!,
      acceptedEvapotranspiration: values.get(47)!,
    };
  } else if (forcing.some(present)) {
    const missing = forcing.find((field) => !present(field));
    throw new Error(`missing observer hydrology forcing field ${missing}`);
  }
  return {
    schemaVersion,
    totalSurface: values.get(37)!,
    totalSoil: values.get(38)!,
    totalGroundwater: values.get(39)!,
    totalConveyance: values.get(40)!,
    latestResidual: values.get(41)!,
    activeChunkCount: requireU32(values.get(42)!, 42),
    latestForcing,
  };
}

/** The one wire type each bootstrap field is allowed to arrive on. */
function bootstrapWireType(field: number): number {
  return field === 35 ? 2 : 0;
}

/**
 * Decode the bounded bootstrap summary as one atomic optional group.
 *
 * Fields 28..=35 are additive, so a payload carrying none of them was written
 * before the summary existed and decodes to the explicit absent schema — that is
 * the only tolerated incompleteness. A payload carrying *part* of the group is
 * not an older peer, it is a contradiction, and every way of being partially
 * present fails closed here rather than being filled in with zeroes. Kept
 * deliberately identical to `decode_bootstrap_summary` in
 * `causafera-observer-wire`: two decoders of one wire contract that disagree on
 * what is valid are worse than one.
 */
function decodeBootstrapSummary(
  values: Map<number, bigint>,
  receipts: BootstrapReceipt[],
  stageSeven: BootstrapReceipt | null,
): BootstrapSummary {
  const group = [28, 29, 30, 31, 32, 33, 34];
  const declared = group.some((field) => values.has(field));
  if (!declared) {
    if (receipts.length > 0) {
      // Receipts with no summary to interpret them would otherwise be parsed
      // and then silently dropped.
      throw new Error("observer bootstrap receipts carry no summary");
    }
    return {
      schemaVersion: BOOTSTRAP_SUMMARY_SCHEMA_ABSENT,
      planId: 0n,
      worldSeed: 0n,
      stageCount: 0,
      complete: false,
      configuredPopulation: 0n,
      configuredPromotionLimit: 0,
      receipts: [],
      stageSeven: null,
    };
  }
  for (const field of group) {
    if (!values.has(field)) {
      throw new Error(`incomplete observer bootstrap summary: missing field ${field}`);
    }
  }

  // Compared as a bigint before widening: a value past 2^53 would otherwise be
  // rounded, and could round onto the accepted version.
  if (values.get(28)! !== BigInt(BOOTSTRAP_SUMMARY_SCHEMA_V1)) {
    throw new Error(`unsupported observer bootstrap summary schema ${values.get(28)}`);
  }
  const schemaVersion = Number(values.get(28));
  if (schemaVersion !== BOOTSTRAP_SUMMARY_SCHEMA_V1) {
    throw new Error(`unsupported observer bootstrap summary schema ${schemaVersion}`);
  }
  if (values.get(31)! > BigInt(MAX_BOOTSTRAP_RECEIPT_SUMMARIES)) {
    throw new Error("observer bootstrap summary exceeds its stage bound");
  }
  const stageCount = Number(values.get(31));
  if (stageCount > MAX_BOOTSTRAP_RECEIPT_SUMMARIES) {
    throw new Error("observer bootstrap summary exceeds its stage bound");
  }
  const completeRaw = values.get(32);
  if (completeRaw !== 0n && completeRaw !== 1n) {
    throw new Error("observer bootstrap completeness must be zero or one");
  }
  for (let index = 1; index < receipts.length; index += 1) {
    if (receipts[index - 1].stage >= receipts[index].stage) {
      throw new Error("observer bootstrap receipts are not in canonical order");
    }
  }
  if (receipts.length !== stageCount) {
    throw new Error("observer bootstrap receipt count does not match its stage count");
  }
  const complete = completeRaw === 1n;
  // A record is complete exactly when it closed every stage it declared.
  if (complete !== stageCount > 0) {
    throw new Error("observer bootstrap completeness disagrees with its stage count");
  }
  const configuredPromotionLimit = requireU32(values.get(34)!, 34);
  return {
    schemaVersion,
    planId: values.get(29)!,
    worldSeed: values.get(30)!,
    stageCount,
    complete,
    configuredPopulation: values.get(33)!,
    configuredPromotionLimit,
    receipts,
    stageSeven,
  };
}

function decodeBootstrapReceipt(input: Uint8Array): BootstrapReceipt {
  const cursor = new Cursor(input);
  const values = new Map<number, bigint>();
  let result: Uint8Array | undefined;
  const dependencyTraces: bigint[] = [];
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (wire === 2 && field === 3) {
      if (result !== undefined) {
        throw new Error("duplicate observer bootstrap receipt result");
      }
      result = cursor.bytes();
    }
    else if (wire === 0 && field === 5) {
      if (dependencyTraces.length === MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES) {
        throw new Error("observer bootstrap receipt exceeds its dependency bound");
      }
      dependencyTraces.push(cursor.varint());
    }
    else if (wire === 0 && field >= 1 && field <= 4) {
      // A receipt's scalars are single-valued, exactly like the summary's.
      if (values.has(field)) {
        throw new Error(`duplicate observer bootstrap receipt field ${field}`);
      }
      values.set(field, cursor.varint());
    }
    // A known field arriving on the wrong wire type is a malformed receipt, not
    // an unknown field to skip past.
    else if (field >= 1 && field <= 5) {
      throw new Error(`observer bootstrap receipt field ${field} has the wrong wire type`);
    }
    else if (wire === 0) values.set(field, cursor.varint());
    else cursor.skip(wire);
  }
  if (result === undefined || result.length !== 32) {
    throw new Error("observer bootstrap receipt result must contain 32 bytes");
  }
  for (let index = 1; index < dependencyTraces.length; index += 1) {
    if (dependencyTraces[index - 1] >= dependencyTraces[index]) {
      throw new Error("observer bootstrap receipt dependencies are not in canonical order");
    }
  }
  const value = requiredValue(values, "BootstrapReceipt");
  return {
    stage: value(1),
    completedAt: value(2),
    result,
    completionTrace: value(4),
    dependencyTraces,
  };
}

export function decodeWorldChunkSnapshot(input: Uint8Array): WorldChunkSnapshot {
  const cursor = new Cursor(input);
  let simulationTicks: bigint | undefined;
  const chunks: SpatialChunkSummary[] = [];
  const materialSurfaceDeltaBytes: Uint8Array[] = [];
  const materialSurfaceGateDeltas: MaterialSurfaceGateDelta[] = [];
  const materialSurfaceThermalDeltas: MaterialSurfaceThermalDelta[] = [];
  const thermalDeltaBytes: Uint8Array[] = [];
  let materialSurfaceDeltaSchemaVersion = 0;
  let schemaVersionSeen = false;
  let thermalDeltaSchemaVersion = 0;
  let thermalSchemaVersionSeen = false;
  const hydrologyDeltaBytes: Uint8Array[] = [];
  const hydrologyTransferBytes: Uint8Array[] = [];
  const hydrologyConveyanceBytes: Uint8Array[] = [];
  let hydrologyDeltaSchemaVersion = 0;
  let hydrologyDeltaSchemaSeen = false;
  let hydrologyTransferSchemaVersion = 0;
  let hydrologyTransferSchemaSeen = false;
  let hydrologyConveyanceSchemaVersion = 0;
  let hydrologyConveyanceSchemaSeen = false;
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) simulationTicks = cursor.varint();
    else if (field === 2 && wire === 2) chunks.push(decodeSpatialChunk(cursor.bytes()));
    else if (field === 3 && wire === 2) {
      const delta = cursor.bytes();
      if (materialSurfaceDeltaBytes.length < MAX_MATERIAL_SURFACE_DELTAS) {
        materialSurfaceDeltaBytes.push(delta);
      }
    }
    else if (field === 4 && wire === 0) {
      if (schemaVersionSeen) throw new Error("duplicate WorldChunkSnapshot field 4");
      const version = cursor.varint();
      if (version > 0xFFFFFFFFn) throw new Error("WorldChunkSnapshot schema version overflows u32");
      materialSurfaceDeltaSchemaVersion = Number(version);
      schemaVersionSeen = true;
    }
    else if (field === 5) {
      if (materialSurfaceDeltaSchemaVersion < MATERIAL_SURFACE_DELTA_SCHEMA_V3) {
        throw new Error("MaterialSurfaceGateDelta field 5 is not allowed for schema version " + materialSurfaceDeltaSchemaVersion);
      }
      if (wire !== 2) throw new Error("unexpected wire type for MaterialSurfaceGateDelta field 5");
      const delta = cursor.bytes();
      if (materialSurfaceGateDeltas.length < MAX_MATERIAL_SURFACE_DELTAS) {
        materialSurfaceGateDeltas.push(decodeMaterialSurfaceGateDelta(delta));
      }
    }
    else if (field === 6 && wire === 2) {
      const delta = cursor.bytes();
      if (thermalDeltaBytes.length < MAX_THERMAL_DELTAS) {
        thermalDeltaBytes.push(delta);
      }
    }
    else if (field === 6) throw new Error("unexpected wire type for ThermalFieldDelta field 6");
    else if (field === 7 && wire === 0) {
      if (thermalSchemaVersionSeen) throw new Error("duplicate WorldChunkSnapshot field 7");
      const version = cursor.varint();
      if (version > 0xFFFFFFFFn) throw new Error("WorldChunkSnapshot thermal schema version overflows u32");
      thermalDeltaSchemaVersion = Number(version);
      thermalSchemaVersionSeen = true;
    }
    else if (field === 7) throw new Error("duplicate WorldChunkSnapshot field 7");
    else if (field === 8) {
      if (materialSurfaceDeltaSchemaVersion < MATERIAL_SURFACE_DELTA_SCHEMA_V4) {
        throw new Error("MaterialSurfaceThermalDelta field 8 is not allowed for schema version " + materialSurfaceDeltaSchemaVersion);
      }
      if (wire !== 2) throw new Error("unexpected wire type for MaterialSurfaceThermalDelta field 8");
      const delta = cursor.bytes();
      if (materialSurfaceThermalDeltas.length < MAX_MATERIAL_SURFACE_DELTAS) {
        materialSurfaceThermalDeltas.push(decodeMaterialSurfaceThermalDelta(delta));
      }
    }
    // Hydrology's three lists reject at `limit + 1` rather than skipping past
    // it. The older lists above silently drop the excess, which reports a
    // truncated projection as a complete one.
    else if (field === 9 && wire === 2) {
      if (hydrologyDeltaBytes.length === MAX_HYDROLOGY_DELTAS) {
        throw new Error("hydrology cell deltas exceed their bound");
      }
      hydrologyDeltaBytes.push(cursor.bytes());
    }
    else if (field === 10 && wire === 0) {
      if (hydrologyDeltaSchemaSeen) throw new Error("duplicate WorldChunkSnapshot field 10");
      hydrologyDeltaSchemaVersion = requireU32(cursor.varint(), 10);
      hydrologyDeltaSchemaSeen = true;
    }
    else if (field === 11 && wire === 2) {
      if (hydrologyTransferBytes.length === MAX_HYDROLOGY_TRANSFER_SUMMARIES) {
        throw new Error("hydrology transfer summaries exceed their bound");
      }
      hydrologyTransferBytes.push(cursor.bytes());
    }
    else if (field === 12 && wire === 0) {
      if (hydrologyTransferSchemaSeen) throw new Error("duplicate WorldChunkSnapshot field 12");
      hydrologyTransferSchemaVersion = requireU32(cursor.varint(), 12);
      hydrologyTransferSchemaSeen = true;
    }
    else if (field === 13 && wire === 2) {
      if (hydrologyConveyanceBytes.length === MAX_HYDROLOGY_CONVEYANCE_SUMMARIES) {
        throw new Error("hydrology conveyance summaries exceed their bound");
      }
      hydrologyConveyanceBytes.push(cursor.bytes());
    }
    else if (field === 14 && wire === 0) {
      if (hydrologyConveyanceSchemaSeen) throw new Error("duplicate WorldChunkSnapshot field 14");
      hydrologyConveyanceSchemaVersion = requireU32(cursor.varint(), 14);
      hydrologyConveyanceSchemaSeen = true;
    }
    else if (field >= 9 && field <= 14) {
      throw new Error(`unexpected wire type for WorldChunkSnapshot field ${field}`);
    }
    else cursor.skip(wire);
  }
  if (simulationTicks === undefined) throw new Error("missing WorldChunkSnapshot field 1");
  const materialSurfaceDeltas = materialSurfaceDeltaBytes.map((delta) =>
    decodeMaterialSurfaceDelta(delta, materialSurfaceDeltaSchemaVersion)
  );
  if (thermalDeltaBytes.length > 0 && thermalDeltaSchemaVersion < THERMAL_DELTA_SCHEMA_V1) {
    throw new Error("ThermalFieldDelta field 6 is not allowed for schema version " + thermalDeltaSchemaVersion);
  }
  const thermalDeltas = thermalDeltaBytes.map((delta) =>
    decodeThermalFieldDelta(delta, thermalDeltaSchemaVersion)
  );
  // Entries with no schema to interpret them would be read under a contract the
  // payload never declared.
  if (hydrologyDeltaBytes.length > 0 && hydrologyDeltaSchemaVersion < HYDROLOGY_DELTA_SCHEMA_V1) {
    throw new Error("HydrologyCellDelta field 9 is not allowed without field 10");
  }
  if (
    hydrologyTransferBytes.length > 0 &&
    hydrologyTransferSchemaVersion < HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1
  ) {
    throw new Error("HydrologyTransferSummary field 11 is not allowed without field 12");
  }
  if (
    hydrologyConveyanceBytes.length > 0 &&
    hydrologyConveyanceSchemaVersion < HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1
  ) {
    throw new Error("HydrologyConveyanceSummary field 13 is not allowed without field 14");
  }
  const hydrologyDeltas = hydrologyDeltaBytes.map(decodeHydrologyCellDelta);
  const hydrologyTransferSummaries = hydrologyTransferBytes.map(decodeHydrologyTransferSummary);
  const hydrologyConveyanceSummaries = hydrologyConveyanceBytes.map(
    decodeHydrologyConveyanceSummary,
  );
  requireDistinctHydrologyKeys(
    hydrologyDeltas,
    hydrologyTransferSummaries,
    hydrologyConveyanceSummaries,
  );
  return {
    simulationTicks,
    chunks,
    materialSurfaceDeltaSchemaVersion,
    materialSurfaceDeltas,
    materialSurfaceGateDeltas,
    materialSurfaceThermalDeltas,
    thermalDeltaSchemaVersion,
    thermalDeltas,
    hydrologyDeltas,
    hydrologyDeltaSchemaVersion,
    hydrologyTransferSummaries,
    hydrologyTransferSchemaVersion,
    hydrologyConveyanceSummaries,
    hydrologyConveyanceSchemaVersion,
  };
}

function decodeHydrologyCellDelta(input: Uint8Array): HydrologyCellDelta {
  const cursor = new Cursor(input);
  const scalars = new Map<number, bigint>();
  const volumes = new Map<number, bigint>();
  let netForcing: bigint | undefined;
  let netLateralFlow: bigint | undefined;
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field >= 1 && field <= 5 && wire === 0) {
      if (scalars.has(field)) throw new Error(`duplicate HydrologyCellDelta field ${field}`);
      scalars.set(field, cursor.varint());
    }
    else if (field >= 6 && field <= 11 && wire === 2) {
      if (volumes.has(field)) throw new Error(`duplicate HydrologyCellDelta field ${field}`);
      volumes.set(field, decodeCanonicalU64(cursor.bytes()));
    }
    else if (field === 12 && wire === 2) {
      if (netForcing !== undefined) throw new Error("duplicate HydrologyCellDelta field 12");
      netForcing = decodeCanonicalZigzagI128(cursor.bytes());
    }
    else if (field === 13 && wire === 2) {
      if (netLateralFlow !== undefined) throw new Error("duplicate HydrologyCellDelta field 13");
      netLateralFlow = decodeCanonicalZigzagI128(cursor.bytes());
    }
    else if (field >= 14 && field <= 16 && wire === 0) {
      if (scalars.has(field)) throw new Error(`duplicate HydrologyCellDelta field ${field}`);
      scalars.set(field, cursor.varint());
    }
    else if (field >= 1 && field <= 16) {
      throw new Error(`unexpected wire type for HydrologyCellDelta field ${field}`);
    }
    else cursor.skip(wire);
  }
  const scalar = requiredValue(scalars, "HydrologyCellDelta");
  const volume = requiredValue(volumes, "HydrologyCellDelta");
  if (netForcing === undefined) throw new Error("missing HydrologyCellDelta field 12");
  if (netLateralFlow === undefined) throw new Error("missing HydrologyCellDelta field 13");
  return {
    chartId: scalar(1),
    chunkX: Number(zigzagDecode(scalar(2))),
    chunkY: Number(zigzagDecode(scalar(3))),
    chunkZ: Number(zigzagDecode(scalar(4))),
    cellOrdinal: requireU16(scalar(5), 5),
    surfaceBefore: volume(6),
    surfaceAfter: volume(7),
    soilBefore: volume(8),
    soilAfter: volume(9),
    groundwaterBefore: volume(10),
    groundwaterAfter: volume(11),
    netForcing,
    netLateralFlow,
    transitionTraceId: scalar(14),
    conservationTraceId: scalar(15),
    transitionTick: scalar(16),
  };
}

function decodeHydrologyTransferSummary(input: Uint8Array): HydrologyTransferSummary {
  const cursor = new Cursor(input);
  const scalars = new Map<number, bigint>();
  const volumes = new Map<number, bigint>();
  let sourceKey: Uint8Array | undefined;
  let targetKey: Uint8Array | undefined;
  let forcingOriginTraceId: bigint | null = null;
  let forcingSeen = false;
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) {
      if (scalars.has(1)) throw new Error("duplicate HydrologyTransferSummary field 1");
      scalars.set(1, cursor.varint());
    }
    else if (field === 2 && wire === 2) {
      if (sourceKey !== undefined) throw new Error("duplicate HydrologyTransferSummary field 2");
      sourceKey = cursor.bytes();
    }
    else if (field === 3 && wire === 2) {
      if (targetKey !== undefined) throw new Error("duplicate HydrologyTransferSummary field 3");
      targetKey = cursor.bytes();
    }
    else if (field >= 4 && field <= 6 && wire === 2) {
      if (volumes.has(field)) {
        throw new Error(`duplicate HydrologyTransferSummary field ${field}`);
      }
      volumes.set(field, decodeCanonicalU64(cursor.bytes()));
    }
    else if (field >= 7 && field <= 9 && wire === 0) {
      if (scalars.has(field)) {
        throw new Error(`duplicate HydrologyTransferSummary field ${field}`);
      }
      scalars.set(field, cursor.varint());
    }
    else if (field === 10 && wire === 0) {
      if (forcingSeen) throw new Error("duplicate HydrologyTransferSummary field 10");
      forcingSeen = true;
      forcingOriginTraceId = cursor.varint();
    }
    else if (field >= 1 && field <= 10) {
      throw new Error(`unexpected wire type for HydrologyTransferSummary field ${field}`);
    }
    else cursor.skip(wire);
  }
  if (sourceKey === undefined) throw new Error("missing HydrologyTransferSummary field 2");
  if (targetKey === undefined) throw new Error("missing HydrologyTransferSummary field 3");
  const sourceVariant = validateHydrologyCarrierKey(sourceKey);
  validateHydrologyCarrierKey(targetKey);
  // A cell is the one carrier a transfer may name twice: infiltration,
  // percolation, and evapotranspiration move water between buckets inside one
  // cell, and the buckets are not part of the key. Every other carrier is a
  // single store or a single face, so naming it as both ends describes a
  // transfer to nowhere.
  if (
    sourceVariant !== HYDROLOGY_CARRIER_CELL &&
    sourceKey.length === targetKey.length &&
    sourceKey.every((byte, index) => byte === targetKey![index])
  ) {
    throw new Error("a hydrology transfer names the same carrier as source and target");
  }
  const scalar = requiredValue(scalars, "HydrologyTransferSummary");
  const volume = requiredValue(volumes, "HydrologyTransferSummary");
  const requestedVolume = volume(4);
  const acceptedVolume = volume(5);
  const unacceptedVolume = volume(6);
  // The three volumes are one statement, not three: a payload where they do not
  // close has either invented water or lost some.
  if (requestedVolume - acceptedVolume !== unacceptedVolume || acceptedVolume > requestedVolume) {
    throw new Error("hydrology transfer volumes do not close");
  }
  return {
    processKind: requireU32(scalar(1), 1),
    sourceKey,
    targetKey,
    requestedVolume,
    acceptedVolume,
    unacceptedVolume,
    transferTraceId: scalar(7),
    conservationTraceId: scalar(8),
    tick: scalar(9),
    forcingOriginTraceId,
  };
}

function decodeHydrologyConveyanceSummary(input: Uint8Array): HydrologyConveyanceSummary {
  const cursor = new Cursor(input);
  const scalars = new Map<number, bigint>();
  const volumes = new Map<number, bigint>();
  let edgeKey: Uint8Array | undefined;
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 2) {
      if (edgeKey !== undefined) throw new Error("duplicate HydrologyConveyanceSummary field 1");
      edgeKey = cursor.bytes();
    }
    else if (field >= 2 && field <= 5 && wire === 2) {
      if (volumes.has(field)) {
        throw new Error(`duplicate HydrologyConveyanceSummary field ${field}`);
      }
      volumes.set(field, decodeCanonicalU64(cursor.bytes()));
    }
    else if ((field === 6 || field === 7) && wire === 0) {
      if (scalars.has(field)) {
        throw new Error(`duplicate HydrologyConveyanceSummary field ${field}`);
      }
      scalars.set(field, cursor.varint());
    }
    else if (field >= 1 && field <= 7) {
      throw new Error(`unexpected wire type for HydrologyConveyanceSummary field ${field}`);
    }
    else cursor.skip(wire);
  }
  if (edgeKey === undefined) throw new Error("missing HydrologyConveyanceSummary field 1");
  // A conveyance summary describes one edge. Any other carrier here would be a
  // storage-and-discharge report about something that has neither.
  if (validateHydrologyCarrierKey(edgeKey) !== HYDROLOGY_CARRIER_EDGE) {
    throw new Error("a hydrology conveyance summary must name an edge");
  }
  const scalar = requiredValue(scalars, "HydrologyConveyanceSummary");
  const volume = requiredValue(volumes, "HydrologyConveyanceSummary");
  return {
    edgeKey,
    storage: volume(2),
    capacity: volume(3),
    acceptedInflow: volume(4),
    acceptedRelease: volume(5),
    lastChangeTraceId: scalar(6),
    tick: scalar(7),
  };
}

/**
 * Refuse a projection that describes the same thing twice.
 *
 * A duplicate is not a redundant row. Two deltas for one cell in one tick
 * disagree about what that cell did, and a reader summing accepted volumes over
 * a repeated transfer counts water that moved once as water that moved twice.
 */
function requireDistinctHydrologyKeys(
  deltas: HydrologyCellDelta[],
  transfers: HydrologyTransferSummary[],
  conveyance: HydrologyConveyanceSummary[],
): void {
  const hex = (bytes: Uint8Array): string =>
    Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
  const cells = new Set<string>();
  for (const delta of deltas) {
    const key = [
      delta.transitionTick,
      delta.chartId,
      delta.chunkX,
      delta.chunkY,
      delta.chunkZ,
      delta.cellOrdinal,
    ].join(":");
    if (cells.has(key)) throw new Error("duplicate hydrology cell delta key");
    cells.add(key);
  }
  const keys = new Set<string>();
  for (const summary of transfers) {
    const key = [
      summary.tick,
      summary.processKind,
      hex(summary.sourceKey),
      hex(summary.targetKey),
      summary.transferTraceId,
    ].join(":");
    if (keys.has(key)) throw new Error("duplicate hydrology transfer key");
    keys.add(key);
  }
  const edges = new Set<string>();
  for (const summary of conveyance) {
    const key = `${summary.tick}:${hex(summary.edgeKey)}`;
    if (edges.has(key)) throw new Error("duplicate hydrology conveyance key");
    edges.add(key);
  }
}

export function decodeStreamEnvelope(input: Uint8Array): StreamEnvelope {
  const cursor = new Cursor(input);
  let header: StreamEnvelope["header"] | undefined;
  let kind: StreamKind | undefined;
  let chunkId: bigint | undefined;
  let payload: Uint8Array = new Uint8Array();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 2) header = decodeStreamHeader(cursor.bytes());
    else if (field === 2 && wire === 0) kind = Number(cursor.varint()) as StreamKind;
    else if (field === 3 && wire === 2) chunkId = decodeChunkScope(cursor.bytes());
    else if (field === 4 && wire === 2) payload = cursor.bytes();
    else cursor.skip(wire);
  }
  if (header === undefined || kind === undefined) throw new Error("incomplete stream envelope");
  return { header, kind, chunkId, payload };
}

export function decodeExplanationReport(input: Uint8Array): ExplanationReport {
  const cursor = new Cursor(input);
  let experimentId: bigint | undefined;
  let overallAssessment: Assessment | undefined;
  const frames: ExplanationFrame[] = [];
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) experimentId = cursor.varint();
    else if (field === 2 && wire === 2) frames.push(decodeExplanationFrame(cursor.bytes()));
    else if (field === 3 && wire === 0) overallAssessment = Number(cursor.varint()) as Assessment;
    else cursor.skip(wire);
  }
  if (experimentId === undefined || overallAssessment === undefined) {
    throw new Error("incomplete ExplanationReport");
  }
  return { experimentId, frames, overallAssessment };
}

export function digestHex(value: Uint8Array, length = value.length): string {
  return Array.from(value.slice(0, length), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function decodeSpatialChunk(input: Uint8Array): SpatialChunkSummary {
  const cursor = new Cursor(input);
  const values = new Map<number, bigint>();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (wire === 0) values.set(field, cursor.varint());
    else cursor.skip(wire);
  }
  const value = requiredValue(values, "SpatialChunkSummary");
  return {
    chartId: value(1),
    chunkX: Number(zigzagDecode(value(2))),
    chunkY: Number(zigzagDecode(value(3))),
    chunkZ: Number(zigzagDecode(value(4))),
    minimumElevationMm: Number(zigzagDecode(value(5))),
    maximumElevationMm: Number(zigzagDecode(value(6))),
    meanRoughnessMm: Number(value(7)),
    manaTotal: zigzagDecode(value(8)),
    resolutionRelevance: zigzagDecode(value(9)),
    resolutionLevel: Number(value(10)),
    populationTotal: value(11),
    causalEventCount: value(12),
    latestTraceId: value(13),
  };
}

function decodeMaterialSurfaceDelta(input: Uint8Array, schemaVersion: number): MaterialSurfaceDelta {
  const values = decodeVarintFields(input);
  const value = requiredValue(values, "MaterialSurfaceDelta");
  const contactTraceId = values.get(9);
  const manaEffectTraceId = values.get(10);
  const manaTransitionTraceId = values.get(12);
  if (schemaVersion < MATERIAL_SURFACE_DELTA_SCHEMA_V3) {
    const cursor = new Cursor(input);
    while (!cursor.empty) {
      const [field, wire] = cursor.key();
      if (field >= 15 && field <= 17) {
        throw new Error("MaterialSurfaceDelta fields 15-17 are not allowed for schema version " + schemaVersion);
      }
      cursor.skip(wire);
    }
  }
  return {
    chartId: value(1),
    chunkX: Number(zigzagDecode(value(2))),
    chunkY: Number(zigzagDecode(value(3))),
    chunkZ: Number(zigzagDecode(value(4))),
    cellOrdinal: Number(value(5)),
    beforeCondition: zigzagDecode(value(6)),
    afterCondition: zigzagDecode(value(7)),
    manaTotal: zigzagDecode(value(8)),
    contactTraceId,
    manaEffectTraceId,
    transitionTick: value(11),
    manaTransitionTraceId,
    manaBefore: values.has(13) ? zigzagDecode(value(13)) : undefined,
    manaAfter: values.has(14) ? zigzagDecode(value(14)) : undefined,
    localManaBefore: values.has(15) ? zigzagDecode(value(15)) : undefined,
    localManaAfter: values.has(16) ? zigzagDecode(value(16)) : undefined,
    localManaTransitionTraceId: values.get(17),
  };
}

function decodeMaterialSurfaceGateDelta(input: Uint8Array): MaterialSurfaceGateDelta {
  const values = decodeVarintFields(input);
  const value = requiredValue(values, "MaterialSurfaceGateDelta");
  const beforeActive = value(6);
  const afterActive = value(7);
  if ((beforeActive !== 0n && beforeActive !== 1n) || (afterActive !== 0n && afterActive !== 1n)) {
    throw new Error("MaterialSurfaceGateDelta boolean is malformed");
  }
  return {
    chartId: value(1),
    chunkX: Number(zigzagDecode(value(2))),
    chunkY: Number(zigzagDecode(value(3))),
    chunkZ: Number(zigzagDecode(value(4))),
    cellOrdinal: Number(value(5)),
    beforeActive: beforeActive === 1n,
    afterActive: afterActive === 1n,
    localManaBefore: zigzagDecode(value(8)),
    localManaAfter: zigzagDecode(value(9)),
    localManaTransitionTraceId: value(10),
    gateTransitionTraceId: value(11),
    contactTraceId: values.get(12),
    transitionTick: value(13),
  };
}

function decodeMaterialSurfaceThermalDelta(input: Uint8Array): MaterialSurfaceThermalDelta {
  const values = decodeVarintFields(input);
  const value = requiredValue(values, "MaterialSurfaceThermalDelta");
  return {
    chartId: value(1),
    chunkX: Number(zigzagDecode(value(2))),
    chunkY: Number(zigzagDecode(value(3))),
    chunkZ: Number(zigzagDecode(value(4))),
    cellOrdinal: Number(value(5)),
    beforeRetained: zigzagDecode(value(6)),
    afterRetained: zigzagDecode(value(7)),
    cellPreState: zigzagDecode(value(8)),
    signedFlux: zigzagDecode(value(9)),
    thermalExchangeTraceId: value(10),
    transitionTick: value(11),
  };
}

function decodeThermalFieldDelta(input: Uint8Array, schemaVersion: number): ThermalFieldDelta {
  if (schemaVersion < THERMAL_DELTA_SCHEMA_V1) {
    throw new Error("ThermalFieldDelta fields are not allowed for schema version " + schemaVersion);
  }
  const values = decodeVarintFields(input);
  const value = requiredValue(values, "ThermalFieldDelta");
  return {
    chartId: value(1),
    chunkX: Number(zigzagDecode(value(2))),
    chunkY: Number(zigzagDecode(value(3))),
    chunkZ: Number(zigzagDecode(value(4))),
    cellOrdinal: Number(value(5)),
    preStateEnergy: zigzagDecode(value(6)),
    postStateEnergy: zigzagDecode(value(7)),
    reservoirScheduledInjection: zigzagDecode(value(8)),
    reservoirAcceptedInjection: zigzagDecode(value(9)),
    reservoirRejectedInjection: zigzagDecode(value(10)),
    netFaceFlux: zigzagDecode(value(11)),
    faceCount: Number(value(12)),
  };
}

function decodeStreamHeader(input: Uint8Array): StreamEnvelope["header"] {
  const cursor = new Cursor(input);
  let streamId: bigint | undefined;
  let schemaVersion: number | undefined;
  let sequenceNumber: bigint | undefined;
  let simulationTime: bigint | undefined;
  let physicalDigest: Uint8Array = new Uint8Array();
  let historyDigest: Uint8Array = new Uint8Array();
  let isSnapshot = false;
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) streamId = cursor.varint();
    else if (field === 2 && wire === 0) schemaVersion = Number(cursor.varint());
    else if (field === 3 && wire === 0) sequenceNumber = cursor.varint();
    else if (field === 4 && wire === 2) simulationTime = decodeSimulationTime(cursor.bytes());
    else if (field === 5 && wire === 2) physicalDigest = decodeDigest(cursor.bytes());
    else if (field === 6 && wire === 2) historyDigest = decodeDigest(cursor.bytes());
    else if (field === 7 && wire === 0) isSnapshot = cursor.varint() !== 0n;
    else cursor.skip(wire);
  }
  if (
    streamId === undefined ||
    schemaVersion === undefined ||
    sequenceNumber === undefined ||
    simulationTime === undefined ||
    physicalDigest.length !== 32 ||
    historyDigest.length !== 32
  ) {
    throw new Error("incomplete stream header");
  }
  return {
    streamId,
    schemaVersion,
    sequenceNumber,
    simulationTime,
    physicalDigest,
    historyDigest,
    isSnapshot,
  };
}

function decodeExplanationFrame(input: Uint8Array): ExplanationFrame {
  const cursor = new Cursor(input);
  let checkpointTicks: bigint | undefined;
  let overallAssessment: Assessment | undefined;
  const claims: ExplanationClaim[] = [];
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) checkpointTicks = cursor.varint();
    else if (field === 2 && wire === 2) claims.push(decodeExplanationClaim(cursor.bytes()));
    else if (field === 3 && wire === 0) overallAssessment = Number(cursor.varint()) as Assessment;
    else cursor.skip(wire);
  }
  if (checkpointTicks === undefined || overallAssessment === undefined) {
    throw new Error("incomplete ExplanationFrame");
  }
  return { checkpointTicks, claims, overallAssessment };
}

function decodeExplanationClaim(input: Uint8Array): ExplanationClaim {
  const cursor = new Cursor(input);
  let schemaId: bigint | undefined;
  let value: NumericClaimValue | undefined;
  let confidence: number | undefined;
  let comparison: ExplanationClaim["comparison"] | undefined;
  let evidenceState: EvidenceState | undefined;
  const evidenceTraceIds: bigint[] = [];
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) schemaId = cursor.varint();
    else if (field === 2 && wire === 2) value = decodeNumericClaimValue(cursor.bytes());
    else if (field === 3 && wire === 1) confidence = cursor.float64();
    else if (field === 4 && wire === 0) evidenceTraceIds.push(cursor.varint());
    else if (field === 5 && wire === 2) comparison = decodeComparison(cursor.bytes());
    else if (field === 6 && wire === 0) evidenceState = Number(cursor.varint()) as EvidenceState;
    else cursor.skip(wire);
  }
  if (
    schemaId === undefined ||
    value === undefined ||
    confidence === undefined ||
    comparison === undefined ||
    evidenceState === undefined
  ) {
    throw new Error("incomplete ExplanationClaim");
  }
  return { schemaId, value, confidence, evidenceTraceIds, comparison, evidenceState };
}

function decodeNumericClaimValue(input: Uint8Array): NumericClaimValue {
  const cursor = new Cursor(input);
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 1 && wire === 0) return { kind: "scalar", value: zigzagDecode(cursor.varint()) };
    if (field === 2 && wire === 2) {
      const values = decodeVarintFields(cursor.bytes());
      const value = requiredValue(values, "NumericRange");
      return { kind: "range", start: zigzagDecode(value(1)), end: zigzagDecode(value(2)) };
    }
    if (field === 3 && wire === 2) {
      const values = decodeVarintFields(cursor.bytes());
      const value = requiredValue(values, "NumericRatio");
      return { kind: "ratio", numerator: value(1), denominator: value(2) };
    }
    cursor.skip(wire);
  }
  throw new Error("missing NumericClaimValue variant");
}

function decodeComparison(input: Uint8Array): ExplanationClaim["comparison"] {
  const values = decodeVarintFields(input);
  const kind = Number(values.get(1) ?? 0n);
  const cohortId = values.get(2);
  return cohortId === undefined ? { kind } : { kind, cohortId };
}

function decodeSimulationTime(input: Uint8Array): bigint {
  const values = decodeVarintFields(input);
  const value = values.get(1);
  if (value === undefined) throw new Error("missing SimulationTime field 1");
  return value;
}

function decodeDigest(input: Uint8Array): Uint8Array {
  const cursor = new Cursor(input);
  let value: Uint8Array = new Uint8Array();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (field === 2 && wire === 2) value = cursor.bytes();
    else cursor.skip(wire);
  }
  return value;
}

function decodeChunkScope(input: Uint8Array): bigint | undefined {
  const values = decodeVarintFields(input);
  return values.get(2);
}

function decodeVarintFields(input: Uint8Array): Map<number, bigint> {
  const cursor = new Cursor(input);
  const values = new Map<number, bigint>();
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (wire === 0) values.set(field, cursor.varint());
    else cursor.skip(wire);
  }
  return values;
}

/**
 * Narrow to 32 bits with the same bound Rust's `to_u32` applies.
 *
 * `Number()` on a bigint widens silently and lossily; every field the Rust
 * decoder narrows must be checked here or the two disagree on validity.
 */

export const HYDROLOGY_CARRIER_CELL = 0x01;
export const HYDROLOGY_CARRIER_EDGE = 0x02;
export const HYDROLOGY_CARRIER_EXTERIOR_FACE = 0x03;
export const HYDROLOGY_CARRIER_FORCING_RECORD = 0x04;
export const HYDROLOGY_CARRIER_RESOLUTION_CHUNK = 0x05;
export const HYDROLOGY_CARRIER_BATCH_NODE = 0x06;

/** A cell body: chart u64, chunk x/y/z i32, ordinal u16 — all big-endian. */
const HYDROLOGY_CELL_BODY_LEN = 22;

function hydrologyCarrierKeyLength(variant: number): number {
  switch (variant) {
    case HYDROLOGY_CARRIER_CELL:
      return 1 + HYDROLOGY_CELL_BODY_LEN;
    case HYDROLOGY_CARRIER_EDGE:
      return 1 + 2 * HYDROLOGY_CELL_BODY_LEN;
    case HYDROLOGY_CARRIER_EXTERIOR_FACE:
      return 1 + HYDROLOGY_CELL_BODY_LEN + 1;
    case HYDROLOGY_CARRIER_FORCING_RECORD:
      return 1 + 8 + 8;
    case HYDROLOGY_CARRIER_RESOLUTION_CHUNK:
      return 1 + 8 + 12;
    case HYDROLOGY_CARRIER_BATCH_NODE:
      return 1 + 8;
    default:
      throw new Error(`unknown hydrology carrier key variant ${variant}`);
  }
}

/**
 * The ordering identity of a cell body, as its own fields rather than its bytes.
 *
 * A byte-wise comparison would be wrong and quietly so: the chunk coordinates
 * are two's-complement `i32`, so a negative coordinate's leading byte sorts
 * above a positive one's. Two peers disagreeing about which endpoint of an edge
 * is canonical would give one face two identities.
 */
function hydrologyCellBodyOrder(body: Uint8Array): bigint[] {
  const view = new DataView(body.buffer, body.byteOffset, body.byteLength);
  return [
    view.getBigUint64(0, false),
    BigInt(view.getInt32(8, false)),
    BigInt(view.getInt32(12, false)),
    BigInt(view.getInt32(16, false)),
    BigInt(view.getUint16(20, false)),
  ];
}

function compareCellBodies(low: bigint[], high: bigint[]): number {
  for (let index = 0; index < low.length; index += 1) {
    const a = low[index]!;
    const b = high[index]!;
    if (a !== b) return a < b ? -1 : 1;
  }
  return 0;
}

/**
 * Validate one canonical carrier key and return its variant.
 *
 * Written from the declared encoding rather than shared with the producer, so a
 * payload from an untrusted peer is checked structurally rather than trusted to
 * have come from a well-behaved one.
 */
export function validateHydrologyCarrierKey(bytes: Uint8Array): number {
  const variant = bytes[0];
  if (variant === undefined) throw new Error("empty hydrology carrier key");
  const expected = hydrologyCarrierKeyLength(variant);
  if (bytes.length !== expected) {
    throw new Error(
      `hydrology carrier key variant ${variant} must be ${expected} bytes, got ${bytes.length}`,
    );
  }
  if (variant === HYDROLOGY_CARRIER_EDGE) {
    const low = hydrologyCellBodyOrder(bytes.slice(1, 23));
    const high = hydrologyCellBodyOrder(bytes.slice(23, 45));
    // Equal endpoints are an edge from a cell to itself; reversed ones are the
    // same face under a second name.
    if (compareCellBodies(low, high) >= 0) {
      throw new Error("hydrology edge carrier key is not in canonical endpoint order");
    }
  }
  if (variant === HYDROLOGY_CARRIER_EXTERIOR_FACE && bytes[23]! > 3) {
    throw new Error(`unknown hydrology exterior face direction ${bytes[23]}`);
  }
  return variant;
}

/**
 * Reject a length-delimited LEB128 integer that is not in shortest form.
 *
 * A byte integer that admits several encodings admits several byte strings for
 * one payload, and the digest of a payload is an identity. An encoding whose
 * last byte is `0x00` completed one byte earlier, and the only value whose
 * shortest form ends in `0x00` is zero itself.
 */
function requireShortestForm(input: Uint8Array): void {
  const last = input[input.length - 1];
  if (last === undefined) throw new Error("empty canonical byte integer");
  if (last === 0 && input.length > 1) {
    throw new Error("byte integer is not in shortest canonical form");
  }
  if ((last & 0x80) !== 0) throw new Error("truncated canonical byte integer");
}

function decodeCanonicalUnsigned(input: Uint8Array, bits: bigint): bigint {
  requireShortestForm(input);
  const maximum = (1n << bits) - 1n;
  let value = 0n;
  for (let index = 0; index < input.length; index += 1) {
    const byte = input[index]!;
    const shift = BigInt(index) * 7n;
    if (shift >= bits) throw new Error("canonical byte integer overflows its domain");
    const part = BigInt(byte & 0x7f);
    if (part > maximum >> shift) throw new Error("canonical byte integer overflows its domain");
    value |= part << shift;
    if ((byte & 0x80) === 0) {
      if (index + 1 !== input.length) throw new Error("invalid canonical byte integer");
      return value;
    }
  }
  throw new Error("invalid canonical byte integer");
}

const decodeCanonicalU64 = (input: Uint8Array): bigint => decodeCanonicalUnsigned(input, 64n);
const decodeCanonicalU128 = (input: Uint8Array): bigint => decodeCanonicalUnsigned(input, 128n);

/**
 * The thermal totals in fields 24 and 25 predate the shortest-form rule and keep
 * their existing tolerance; every hydrology byte integer decodes through this.
 */
function decodeCanonicalZigzagI128(input: Uint8Array): bigint {
  requireShortestForm(input);
  return decodeZigzagI128(input);
}

/** A packed band of shortest-form `u64` varints. */
function decodeUnsignedBand(input: Uint8Array): BigUint64Array {
  const cursor = new Cursor(input);
  const decoded: bigint[] = [];
  while (!cursor.empty) decoded.push(cursor.varintShortest());
  return new BigUint64Array(decoded);
}

function requireU16(value: bigint, field: number): number {
  if (value > 0xffffn) throw new Error(`observer field ${field} overflows its 16-bit range`);
  return Number(value);
}

function requireU32(value: bigint, field: number): number {
  if (value > 0xffff_ffffn) {
    throw new Error(`observer field ${field} overflows its 32-bit range`);
  }
  return Number(value);
}

function requiredValue(values: Map<number, bigint>, name: string): (field: number) => bigint {
  return (field: number): bigint => {
    const found = values.get(field);
    if (found === undefined) throw new Error(`missing ${name} field ${field}`);
    return found;
  };
}

function zigzagDecode(value: bigint): bigint {
  return (value >> 1n) ^ -(value & 1n);
}

function zigzagEncode(value: bigint): bigint {
  return BigInt.asUintN(64, (value << 1n) ^ (value >> 63n));
}

function decodeZigzagI128(input: Uint8Array): bigint {
  let encoded = 0n;
  const maximum = (1n << 128n) - 1n;
  for (let index = 0; index < input.length; index += 1) {
    const byte = input[index];
    if (byte === undefined) throw new Error("truncated i128 zigzag varint");
    const shift = BigInt(index) * 7n;
    if (shift >= 128n) throw new Error("i128 zigzag varint overflows");
    const part = BigInt(byte & 0x7f);
    if (part > (maximum >> shift)) throw new Error("i128 zigzag varint overflows");
    encoded |= part << shift;
    if ((byte & 0x80) === 0) {
      if (index + 1 !== input.length) throw new Error("invalid i128 zigzag varint");
      return zigzagDecode(encoded);
    }
  }
  throw new Error("invalid i128 zigzag varint");
}

function fieldVarint(output: number[], field: number, value: bigint): void {
  writeVarint(output, BigInt(field << 3));
  writeVarint(output, value);
}

function fieldBytes(output: number[], field: number, value: Uint8Array): void {
  writeVarint(output, BigInt((field << 3) | 2));
  writeVarint(output, BigInt(value.length));
  output.push(...value);
}

function writeVarint(output: number[], initial: bigint): void {
  let value = initial;
  while (value >= 0x80n) {
    output.push(Number(value & 0x7fn) | 0x80);
    value >>= 7n;
  }
  output.push(Number(value));
}

class Cursor {
  private offset = 0;

  constructor(private readonly input: Uint8Array) {}

  get empty(): boolean {
    return this.offset === this.input.length;
  }

  varint(): bigint {
    let value = 0n;
    for (let shift = 0n; shift <= 63n; shift += 7n) {
      const byte = this.input[this.offset++];
      if (byte === undefined) throw new Error("truncated protobuf varint");
      // The tenth byte contributes only bit 63. A bigint cannot truncate, so
      // without this an over-wide varint would be accepted here and silently
      // truncated by the Rust decoder — the two disagreeing on validity rather
      // than on meaning. Kept identical to `Cursor::varint` in
      // `causafera-observer-wire`.
      if (shift === 63n && (byte & 0x7f) > 1) {
        throw new Error("protobuf varint exceeds 64 bits");
      }
      value |= BigInt(byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) return value;
    }
    throw new Error("invalid protobuf varint");
  }

  /**
   * A varint that must be in shortest canonical form. Used for the packed
   * unsigned raster band, where a redundant continuation byte would give one
   * lattice more than one byte string.
   */
  varintShortest(): bigint {
    const start = this.offset;
    const value = this.varint();
    requireShortestForm(this.input.slice(start, this.offset));
    return value;
  }

  key(): [number, number] {
    const key = this.varint();
    const rawField = key >> 3n;
    // Bounded before widening, matching Rust's `to_u32(key >> 3)`. `Number()` on
    // a bigint neither rejects nor saturates, so without this a field number
    // past 2^32 is accepted here and rejected there.
    if (rawField > 0xffff_ffffn) throw new Error("protobuf field number overflows");
    const field = Number(rawField);
    if (field === 0) throw new Error("invalid protobuf field number");
    return [field, Number(key & 7n)];
  }

  bytes(): Uint8Array {
    const length = Number(this.varint());
    const end = this.offset + length;
    if (end > this.input.length) throw new Error("truncated protobuf bytes");
    const bytes = this.input.slice(this.offset, end);
    this.offset = end;
    return bytes;
  }

  float64(): number {
    const end = this.offset + 8;
    if (end > this.input.length) throw new Error("truncated protobuf fixed64");
    const value = new DataView(
      this.input.buffer,
      this.input.byteOffset + this.offset,
      8,
    ).getFloat64(0, true);
    this.offset = end;
    return value;
  }

  skip(wire: number): void {
    if (wire === 0) {
      this.varint();
      return;
    }
    if (wire === 1) {
      this.advance(8);
      return;
    }
    if (wire === 2) {
      this.bytes();
      return;
    }
    // Wire type 5 (fixed32) is legal protobuf but no encoder here emits it, and
    // `Cursor::skip` in `causafera-observer-wire` rejects it. Accepting it would
    // mean this decoder walks past a field the Rust decoder refuses the whole
    // message over.
    throw new Error(`unsupported protobuf wire type ${wire}`);
  }

  private advance(count: number): void {
    this.offset += count;
    if (this.offset > this.input.length) throw new Error("truncated protobuf field");
  }
}
