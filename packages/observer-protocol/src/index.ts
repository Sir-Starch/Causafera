/** Canonical observer v1 client codec. Field numbers come from proto/causafera/observer/v1. */
export const OBSERVER_PROTOCOL_V1 = 1;
export const MAX_MATERIAL_SURFACE_DELTAS = 64;
export const MATERIAL_SURFACE_DELTA_SCHEMA_V3 = 3;
export const MAX_THERMAL_DELTAS = 64;
export const THERMAL_DELTA_SCHEMA_V1 = 1;

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
}

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
}

export interface WorldChunkSnapshot {
  simulationTicks: bigint;
  chunks: SpatialChunkSummary[];
  materialSurfaceDeltaSchemaVersion: number;
  materialSurfaceDeltas: MaterialSurfaceDelta[];
  materialSurfaceGateDeltas: MaterialSurfaceGateDelta[];
  thermalDeltaSchemaVersion: number;
  thermalDeltas: ThermalFieldDelta[];
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
    else if (field === 4 && wire === 2) payload = cursor.bytes();
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
  while (!cursor.empty) {
    const [field, wire] = cursor.key();
    if (wire === 0) values.set(field, cursor.varint());
    else if (wire === 2 && field === 3) physicalDigest = cursor.bytes();
    else if (wire === 2 && field === 4) historyDigest = cursor.bytes();
    else if (wire === 2 && field === 24) thermalTotalCellEnergy = decodeZigzagI128(cursor.bytes());
    else if (wire === 2 && field === 25) thermalTotalReservoirBudget = decodeZigzagI128(cursor.bytes());
    else cursor.skip(wire);
  }
  if (physicalDigest.length !== 32 || historyDigest.length !== 32) {
    throw new Error("observer digest must contain 32 bytes");
  }
  const value = requiredValue(values, "RuntimeSummary");
  return {
    simulationTicks: value(1),
    digestSchemaVersion: Number(value(2)),
    physicalDigest,
    historyDigest,
    manaTotal: BigInt.asIntN(64, value(5)),
    manaMaximum: BigInt.asIntN(64, value(6)),
    activeChunkCount: Number(value(7)),
    resolutionRelevance: BigInt.asIntN(64, value(8)),
    resolutionLevel: Number(value(9)),
    causalTraceCount: value(10),
    actorCount: Number(value(11)),
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
    thermalActiveChunkCount: Number(values.get(26) ?? 0n),
    thermalActiveCellCount: Number(values.get(27) ?? 0n),
  };
}

export function decodeWorldChunkSnapshot(input: Uint8Array): WorldChunkSnapshot {
  const cursor = new Cursor(input);
  let simulationTicks: bigint | undefined;
  const chunks: SpatialChunkSummary[] = [];
  const materialSurfaceDeltaBytes: Uint8Array[] = [];
  const materialSurfaceGateDeltas: MaterialSurfaceGateDelta[] = [];
  const thermalDeltaBytes: Uint8Array[] = [];
  let materialSurfaceDeltaSchemaVersion = 0;
  let schemaVersionSeen = false;
  let thermalDeltaSchemaVersion = 0;
  let thermalSchemaVersionSeen = false;
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
  return {
    simulationTicks,
    chunks,
    materialSurfaceDeltaSchemaVersion,
    materialSurfaceDeltas,
    materialSurfaceGateDeltas,
    thermalDeltaSchemaVersion,
    thermalDeltas,
  };
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
      value |= BigInt(byte & 0x7f) << shift;
      if ((byte & 0x80) === 0) return value;
    }
    throw new Error("invalid protobuf varint");
  }

  key(): [number, number] {
    const key = this.varint();
    const field = Number(key >> 3n);
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
    if (wire === 5) {
      this.advance(4);
      return;
    }
    throw new Error(`unsupported protobuf wire type ${wire}`);
  }

  private advance(count: number): void {
    this.offset += count;
    if (this.offset > this.input.length) throw new Error("truncated protobuf field");
  }
}
