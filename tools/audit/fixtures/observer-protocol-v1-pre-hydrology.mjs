// FROZEN ARTEFACT — DO NOT EDIT.
//
// The pre-hydrology observer protocol V1 client codec, frozen by
// `plans/hydrology.md` Stage 1 as a test-only oracle. It exists to answer one
// question that no live decoder can answer about itself: does a client built
// before hydrology existed still accept a payload produced by an engine that
// has it? Protocol V1 stays V1 only if the answer is yes, so the answer has to
// come from a decoder that genuinely predates the additions rather than from
// the current one with the new branches mentally subtracted.
//
// Provenance of the freeze:
//   source        packages/observer-protocol/src/index.ts
//   source sha256 79c351d9cdf363d300d460a46b3628a998308ab746da1853bb9ac1340322fa76
//   commit        29fdf4dcbe36b3fd0a822779ba7e38c655bbdbe2
//   emitted by    tsc --target ES2020 --module ESNext --moduleResolution bundler
//
// `tools/audit/test-observer-hydrology-legacy-decoder.mjs` pins this file's own
// digest, so an edit here is a test failure rather than a quiet re-freeze, and
// proves at Stage 1 that the frozen copy and the live decoder agree on real
// payloads — which is what makes the freeze a faithful copy rather than an
// assertion about one.
//
// It is never imported by shipping code. Nothing outside `tools/audit` may
// depend on it.

/** Canonical observer v1 client codec. Field numbers come from proto/causafera/observer/v1. */
export const OBSERVER_PROTOCOL_V1 = 1;
export const MAX_MATERIAL_SURFACE_DELTAS = 64;
export const MATERIAL_SURFACE_DELTA_SCHEMA_V3 = 3;
export const MATERIAL_SURFACE_DELTA_SCHEMA_V4 = 4;
export const MAX_THERMAL_DELTAS = 64;
export const THERMAL_DELTA_SCHEMA_V1 = 1;
export var QueryKind;
(function (QueryKind) {
    QueryKind[QueryKind["RuntimeSummary"] = 1] = "RuntimeSummary";
    QueryKind[QueryKind["ExplanationIr"] = 2] = "ExplanationIr";
    QueryKind[QueryKind["WorldChunks"] = 3] = "WorldChunks";
    QueryKind[QueryKind["FieldRaster"] = 4] = "FieldRaster";
})(QueryKind || (QueryKind = {}));
export var FieldRasterKind;
(function (FieldRasterKind) {
    FieldRasterKind[FieldRasterKind["TerrainElevation"] = 1] = "TerrainElevation";
    FieldRasterKind[FieldRasterKind["TerrainRoughness"] = 2] = "TerrainRoughness";
    FieldRasterKind[FieldRasterKind["ManaIntensity"] = 3] = "ManaIntensity";
})(FieldRasterKind || (FieldRasterKind = {}));
/** The coarsest terrain reduction the projection offers; level 0 is 32 x 32. */
export const MAX_FIELD_RASTER_DETAIL_LEVEL = 2;
export const BOOTSTRAP_SUMMARY_SCHEMA_ABSENT = 0;
export const BOOTSTRAP_SUMMARY_SCHEMA_V1 = 1;
/** The current production bootstrap runs six stages; more is a record this build cannot produce. */
export const MAX_BOOTSTRAP_RECEIPT_SUMMARIES = 6;
export const MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES = 8;
export var QueryStatus;
(function (QueryStatus) {
    QueryStatus[QueryStatus["Ok"] = 1] = "Ok";
    QueryStatus[QueryStatus["InvalidRequest"] = 2] = "InvalidRequest";
    QueryStatus[QueryStatus["Unsupported"] = 3] = "Unsupported";
    QueryStatus[QueryStatus["NotAvailable"] = 4] = "NotAvailable";
})(QueryStatus || (QueryStatus = {}));
export var StreamKind;
(function (StreamKind) {
    StreamKind[StreamKind["RuntimeSummary"] = 1] = "RuntimeSummary";
    StreamKind[StreamKind["Explanation"] = 2] = "Explanation";
    StreamKind[StreamKind["Metrics"] = 3] = "Metrics";
})(StreamKind || (StreamKind = {}));
export var EvidenceState;
(function (EvidenceState) {
    EvidenceState[EvidenceState["Supported"] = 1] = "Supported";
    EvidenceState[EvidenceState["Unsupported"] = 2] = "Unsupported";
    EvidenceState[EvidenceState["Unknown"] = 3] = "Unknown";
})(EvidenceState || (EvidenceState = {}));
export var Assessment;
(function (Assessment) {
    Assessment[Assessment["Supported"] = 1] = "Supported";
    Assessment[Assessment["Partial"] = 2] = "Partial";
    Assessment[Assessment["Unsupported"] = 3] = "Unsupported";
    Assessment[Assessment["Unknown"] = 4] = "Unknown";
})(Assessment || (Assessment = {}));
export function encodeConnectRequest(request) {
    const output = [];
    for (const version of request.supportedProtocolVersions) {
        fieldVarint(output, 1, BigInt(version));
    }
    if (request.observerLocale.length > 0) {
        fieldBytes(output, 2, new TextEncoder().encode(request.observerLocale));
    }
    return Uint8Array.from(output);
}
export function decodeConnectResponse(input) {
    const cursor = new Cursor(input);
    let selectedProtocolVersion;
    let currentTime;
    const capabilities = [];
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            selectedProtocolVersion = Number(cursor.varint());
        else if (field === 2 && wire === 2)
            currentTime = decodeSimulationTime(cursor.bytes());
        else if (field === 3 && wire === 0)
            capabilities.push(Number(cursor.varint()));
        else
            cursor.skip(wire);
    }
    if (selectedProtocolVersion === undefined || currentTime === undefined) {
        throw new Error("incomplete observer connect response");
    }
    return { selectedProtocolVersion, currentTime, capabilities };
}
export function encodeQuery(kind, requestId) {
    const output = [];
    fieldVarint(output, 1, requestId);
    fieldVarint(output, 2, BigInt(OBSERVER_PROTOCOL_V1));
    fieldVarint(output, 3, BigInt(kind));
    return Uint8Array.from(output);
}
export function encodeRuntimeSummaryQuery(requestId) {
    return encodeQuery(QueryKind.RuntimeSummary, requestId);
}
/** A raster asks for one chunk of one field, so its parameters ride the payload. */
export function encodeFieldRasterQuery(requestId, request) {
    const payload = [];
    fieldVarint(payload, 1, request.chartId);
    fieldVarint(payload, 2, zigzagEncode(BigInt(request.chunkX)));
    fieldVarint(payload, 3, zigzagEncode(BigInt(request.chunkY)));
    fieldVarint(payload, 4, zigzagEncode(BigInt(request.chunkZ)));
    fieldVarint(payload, 5, BigInt(request.field));
    fieldVarint(payload, 6, BigInt(request.detailLevel));
    const output = [];
    fieldVarint(output, 1, requestId);
    fieldVarint(output, 2, BigInt(OBSERVER_PROTOCOL_V1));
    fieldVarint(output, 3, BigInt(QueryKind.FieldRaster));
    fieldBytes(output, 5, Uint8Array.from(payload));
    return Uint8Array.from(output);
}
export function decodeFieldRaster(input) {
    const cursor = new Cursor(input);
    let chartId;
    let chunkX = 0;
    let chunkY = 0;
    let chunkZ = 0;
    let field;
    let detailLevel = 0;
    let edge;
    let depth;
    let values = new Float64Array();
    let auxiliary = new Float64Array();
    let cellTraces = new BigUint64Array();
    let generationTraceId = 0n;
    while (!cursor.empty) {
        const [number, wire] = cursor.key();
        if (number === 1 && wire === 0)
            chartId = cursor.varint();
        else if (number === 2 && wire === 0)
            chunkX = Number(zigzagDecode(cursor.varint()));
        else if (number === 3 && wire === 0)
            chunkY = Number(zigzagDecode(cursor.varint()));
        else if (number === 4 && wire === 0)
            chunkZ = Number(zigzagDecode(cursor.varint()));
        else if (number === 5 && wire === 0)
            field = Number(cursor.varint());
        else if (number === 6 && wire === 0)
            detailLevel = Number(cursor.varint());
        else if (number === 7 && wire === 0)
            edge = Number(cursor.varint());
        else if (number === 8 && wire === 0)
            depth = Number(cursor.varint());
        else if (number === 9 && wire === 2)
            values = decodeDeltaBand(cursor.bytes());
        else if (number === 10 && wire === 2)
            auxiliary = decodeDeltaBand(cursor.bytes());
        else if (number === 11 && wire === 2)
            cellTraces = decodeTraceBand(cursor.bytes());
        else if (number === 12 && wire === 0)
            generationTraceId = cursor.varint();
        else
            cursor.skip(wire);
    }
    if (chartId === undefined || field === undefined || edge === undefined || depth === undefined) {
        throw new Error("incomplete observer field raster");
    }
    const cells = edge * edge * depth;
    // A lattice whose payload does not fill it cannot be drawn at real positions,
    // and a renderer must never be left to guess the shape.
    if (values.length !== cells) {
        throw new Error(`field raster declares ${cells} cells but carries ${values.length}`);
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
function decodeDeltaBand(input) {
    const cursor = new Cursor(input);
    const decoded = [];
    let previous = 0n;
    while (!cursor.empty) {
        previous = BigInt.asIntN(64, previous + zigzagDecode(cursor.varint()));
        decoded.push(Number(previous));
    }
    return new Float64Array(decoded);
}
function decodeTraceBand(input) {
    const cursor = new Cursor(input);
    const decoded = [];
    while (!cursor.empty)
        decoded.push(cursor.varint());
    return new BigUint64Array(decoded);
}
export function decodeQueryResponse(input) {
    const cursor = new Cursor(input);
    let requestId;
    let protocolVersion;
    let status;
    let payload = new Uint8Array();
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            requestId = cursor.varint();
        else if (field === 2 && wire === 0)
            protocolVersion = Number(cursor.varint());
        else if (field === 3 && wire === 0)
            status = Number(cursor.varint());
        else if (field === 4 && wire === 2)
            payload = cursor.bytes();
        else
            cursor.skip(wire);
    }
    if (requestId === undefined || protocolVersion === undefined || status === undefined) {
        throw new Error("incomplete observer query response");
    }
    return { requestId, protocolVersion, status, payload };
}
export function decodeRuntimeSummary(input) {
    const cursor = new Cursor(input);
    const values = new Map();
    let physicalDigest = new Uint8Array();
    let historyDigest = new Uint8Array();
    let thermalTotalCellEnergy = 0n;
    let thermalTotalReservoirBudget = 0n;
    const bootstrapReceipts = [];
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        // Checked before the generic varint branch, or field 35 on a varint wire
        // would be stored as a scalar and then silently dropped, while Rust rejects
        // it. Any bootstrap field on a wire type it does not use is malformed.
        if (field >= 28 && field <= 35 && wire !== bootstrapWireType(field)) {
            throw new Error(`observer bootstrap field ${field} has the wrong wire type`);
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
        else if (wire === 2 && field === 3)
            physicalDigest = cursor.bytes();
        else if (wire === 2 && field === 4)
            historyDigest = cursor.bytes();
        else if (wire === 2 && field === 24)
            thermalTotalCellEnergy = decodeZigzagI128(cursor.bytes());
        else if (wire === 2 && field === 25)
            thermalTotalReservoirBudget = decodeZigzagI128(cursor.bytes());
        else if (wire === 2 && field === 35) {
            // Bounded before the list grows: a payload claiming more receipts than
            // this build's bootstrap can produce is rejected, not truncated.
            if (bootstrapReceipts.length === MAX_BOOTSTRAP_RECEIPT_SUMMARIES) {
                throw new Error("observer bootstrap summary exceeds its receipt bound");
            }
            bootstrapReceipts.push(decodeBootstrapReceipt(cursor.bytes()));
        }
        else
            cursor.skip(wire);
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
        bootstrap: decodeBootstrapSummary(values, bootstrapReceipts),
    };
}
/** The one wire type each bootstrap field is allowed to arrive on. */
function bootstrapWireType(field) {
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
function decodeBootstrapSummary(values, receipts) {
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
        };
    }
    for (const field of group) {
        if (!values.has(field)) {
            throw new Error(`incomplete observer bootstrap summary: missing field ${field}`);
        }
    }
    // Compared as a bigint before widening: a value past 2^53 would otherwise be
    // rounded, and could round onto the accepted version.
    if (values.get(28) !== BigInt(BOOTSTRAP_SUMMARY_SCHEMA_V1)) {
        throw new Error(`unsupported observer bootstrap summary schema ${values.get(28)}`);
    }
    const schemaVersion = Number(values.get(28));
    if (schemaVersion !== BOOTSTRAP_SUMMARY_SCHEMA_V1) {
        throw new Error(`unsupported observer bootstrap summary schema ${schemaVersion}`);
    }
    if (values.get(31) > BigInt(MAX_BOOTSTRAP_RECEIPT_SUMMARIES)) {
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
    const configuredPromotionLimit = requireU32(values.get(34), 34);
    return {
        schemaVersion,
        planId: values.get(29),
        worldSeed: values.get(30),
        stageCount,
        complete,
        configuredPopulation: values.get(33),
        configuredPromotionLimit,
        receipts,
    };
}
function decodeBootstrapReceipt(input) {
    const cursor = new Cursor(input);
    const values = new Map();
    let result;
    const dependencyTraces = [];
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
        else if (wire === 0)
            values.set(field, cursor.varint());
        else
            cursor.skip(wire);
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
export function decodeWorldChunkSnapshot(input) {
    const cursor = new Cursor(input);
    let simulationTicks;
    const chunks = [];
    const materialSurfaceDeltaBytes = [];
    const materialSurfaceGateDeltas = [];
    const materialSurfaceThermalDeltas = [];
    const thermalDeltaBytes = [];
    let materialSurfaceDeltaSchemaVersion = 0;
    let schemaVersionSeen = false;
    let thermalDeltaSchemaVersion = 0;
    let thermalSchemaVersionSeen = false;
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            simulationTicks = cursor.varint();
        else if (field === 2 && wire === 2)
            chunks.push(decodeSpatialChunk(cursor.bytes()));
        else if (field === 3 && wire === 2) {
            const delta = cursor.bytes();
            if (materialSurfaceDeltaBytes.length < MAX_MATERIAL_SURFACE_DELTAS) {
                materialSurfaceDeltaBytes.push(delta);
            }
        }
        else if (field === 4 && wire === 0) {
            if (schemaVersionSeen)
                throw new Error("duplicate WorldChunkSnapshot field 4");
            const version = cursor.varint();
            if (version > 0xffffffffn)
                throw new Error("WorldChunkSnapshot schema version overflows u32");
            materialSurfaceDeltaSchemaVersion = Number(version);
            schemaVersionSeen = true;
        }
        else if (field === 5) {
            if (materialSurfaceDeltaSchemaVersion < MATERIAL_SURFACE_DELTA_SCHEMA_V3) {
                throw new Error("MaterialSurfaceGateDelta field 5 is not allowed for schema version " + materialSurfaceDeltaSchemaVersion);
            }
            if (wire !== 2)
                throw new Error("unexpected wire type for MaterialSurfaceGateDelta field 5");
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
        else if (field === 6)
            throw new Error("unexpected wire type for ThermalFieldDelta field 6");
        else if (field === 7 && wire === 0) {
            if (thermalSchemaVersionSeen)
                throw new Error("duplicate WorldChunkSnapshot field 7");
            const version = cursor.varint();
            if (version > 0xffffffffn)
                throw new Error("WorldChunkSnapshot thermal schema version overflows u32");
            thermalDeltaSchemaVersion = Number(version);
            thermalSchemaVersionSeen = true;
        }
        else if (field === 7)
            throw new Error("duplicate WorldChunkSnapshot field 7");
        else if (field === 8) {
            if (materialSurfaceDeltaSchemaVersion < MATERIAL_SURFACE_DELTA_SCHEMA_V4) {
                throw new Error("MaterialSurfaceThermalDelta field 8 is not allowed for schema version " + materialSurfaceDeltaSchemaVersion);
            }
            if (wire !== 2)
                throw new Error("unexpected wire type for MaterialSurfaceThermalDelta field 8");
            const delta = cursor.bytes();
            if (materialSurfaceThermalDeltas.length < MAX_MATERIAL_SURFACE_DELTAS) {
                materialSurfaceThermalDeltas.push(decodeMaterialSurfaceThermalDelta(delta));
            }
        }
        else
            cursor.skip(wire);
    }
    if (simulationTicks === undefined)
        throw new Error("missing WorldChunkSnapshot field 1");
    const materialSurfaceDeltas = materialSurfaceDeltaBytes.map((delta) => decodeMaterialSurfaceDelta(delta, materialSurfaceDeltaSchemaVersion));
    if (thermalDeltaBytes.length > 0 && thermalDeltaSchemaVersion < THERMAL_DELTA_SCHEMA_V1) {
        throw new Error("ThermalFieldDelta field 6 is not allowed for schema version " + thermalDeltaSchemaVersion);
    }
    const thermalDeltas = thermalDeltaBytes.map((delta) => decodeThermalFieldDelta(delta, thermalDeltaSchemaVersion));
    return {
        simulationTicks,
        chunks,
        materialSurfaceDeltaSchemaVersion,
        materialSurfaceDeltas,
        materialSurfaceGateDeltas,
        materialSurfaceThermalDeltas,
        thermalDeltaSchemaVersion,
        thermalDeltas,
    };
}
export function decodeStreamEnvelope(input) {
    const cursor = new Cursor(input);
    let header;
    let kind;
    let chunkId;
    let payload = new Uint8Array();
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 2)
            header = decodeStreamHeader(cursor.bytes());
        else if (field === 2 && wire === 0)
            kind = Number(cursor.varint());
        else if (field === 3 && wire === 2)
            chunkId = decodeChunkScope(cursor.bytes());
        else if (field === 4 && wire === 2)
            payload = cursor.bytes();
        else
            cursor.skip(wire);
    }
    if (header === undefined || kind === undefined)
        throw new Error("incomplete stream envelope");
    return { header, kind, chunkId, payload };
}
export function decodeExplanationReport(input) {
    const cursor = new Cursor(input);
    let experimentId;
    let overallAssessment;
    const frames = [];
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            experimentId = cursor.varint();
        else if (field === 2 && wire === 2)
            frames.push(decodeExplanationFrame(cursor.bytes()));
        else if (field === 3 && wire === 0)
            overallAssessment = Number(cursor.varint());
        else
            cursor.skip(wire);
    }
    if (experimentId === undefined || overallAssessment === undefined) {
        throw new Error("incomplete ExplanationReport");
    }
    return { experimentId, frames, overallAssessment };
}
export function digestHex(value, length = value.length) {
    return Array.from(value.slice(0, length), (byte) => byte.toString(16).padStart(2, "0")).join("");
}
function decodeSpatialChunk(input) {
    const cursor = new Cursor(input);
    const values = new Map();
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (wire === 0)
            values.set(field, cursor.varint());
        else
            cursor.skip(wire);
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
function decodeMaterialSurfaceDelta(input, schemaVersion) {
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
function decodeMaterialSurfaceGateDelta(input) {
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
function decodeMaterialSurfaceThermalDelta(input) {
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
function decodeThermalFieldDelta(input, schemaVersion) {
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
function decodeStreamHeader(input) {
    const cursor = new Cursor(input);
    let streamId;
    let schemaVersion;
    let sequenceNumber;
    let simulationTime;
    let physicalDigest = new Uint8Array();
    let historyDigest = new Uint8Array();
    let isSnapshot = false;
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            streamId = cursor.varint();
        else if (field === 2 && wire === 0)
            schemaVersion = Number(cursor.varint());
        else if (field === 3 && wire === 0)
            sequenceNumber = cursor.varint();
        else if (field === 4 && wire === 2)
            simulationTime = decodeSimulationTime(cursor.bytes());
        else if (field === 5 && wire === 2)
            physicalDigest = decodeDigest(cursor.bytes());
        else if (field === 6 && wire === 2)
            historyDigest = decodeDigest(cursor.bytes());
        else if (field === 7 && wire === 0)
            isSnapshot = cursor.varint() !== 0n;
        else
            cursor.skip(wire);
    }
    if (streamId === undefined ||
        schemaVersion === undefined ||
        sequenceNumber === undefined ||
        simulationTime === undefined ||
        physicalDigest.length !== 32 ||
        historyDigest.length !== 32) {
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
function decodeExplanationFrame(input) {
    const cursor = new Cursor(input);
    let checkpointTicks;
    let overallAssessment;
    const claims = [];
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            checkpointTicks = cursor.varint();
        else if (field === 2 && wire === 2)
            claims.push(decodeExplanationClaim(cursor.bytes()));
        else if (field === 3 && wire === 0)
            overallAssessment = Number(cursor.varint());
        else
            cursor.skip(wire);
    }
    if (checkpointTicks === undefined || overallAssessment === undefined) {
        throw new Error("incomplete ExplanationFrame");
    }
    return { checkpointTicks, claims, overallAssessment };
}
function decodeExplanationClaim(input) {
    const cursor = new Cursor(input);
    let schemaId;
    let value;
    let confidence;
    let comparison;
    let evidenceState;
    const evidenceTraceIds = [];
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            schemaId = cursor.varint();
        else if (field === 2 && wire === 2)
            value = decodeNumericClaimValue(cursor.bytes());
        else if (field === 3 && wire === 1)
            confidence = cursor.float64();
        else if (field === 4 && wire === 0)
            evidenceTraceIds.push(cursor.varint());
        else if (field === 5 && wire === 2)
            comparison = decodeComparison(cursor.bytes());
        else if (field === 6 && wire === 0)
            evidenceState = Number(cursor.varint());
        else
            cursor.skip(wire);
    }
    if (schemaId === undefined ||
        value === undefined ||
        confidence === undefined ||
        comparison === undefined ||
        evidenceState === undefined) {
        throw new Error("incomplete ExplanationClaim");
    }
    return { schemaId, value, confidence, evidenceTraceIds, comparison, evidenceState };
}
function decodeNumericClaimValue(input) {
    const cursor = new Cursor(input);
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 1 && wire === 0)
            return { kind: "scalar", value: zigzagDecode(cursor.varint()) };
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
function decodeComparison(input) {
    const values = decodeVarintFields(input);
    const kind = Number(values.get(1) ?? 0n);
    const cohortId = values.get(2);
    return cohortId === undefined ? { kind } : { kind, cohortId };
}
function decodeSimulationTime(input) {
    const values = decodeVarintFields(input);
    const value = values.get(1);
    if (value === undefined)
        throw new Error("missing SimulationTime field 1");
    return value;
}
function decodeDigest(input) {
    const cursor = new Cursor(input);
    let value = new Uint8Array();
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (field === 2 && wire === 2)
            value = cursor.bytes();
        else
            cursor.skip(wire);
    }
    return value;
}
function decodeChunkScope(input) {
    const values = decodeVarintFields(input);
    return values.get(2);
}
function decodeVarintFields(input) {
    const cursor = new Cursor(input);
    const values = new Map();
    while (!cursor.empty) {
        const [field, wire] = cursor.key();
        if (wire === 0)
            values.set(field, cursor.varint());
        else
            cursor.skip(wire);
    }
    return values;
}
/**
 * Narrow to 32 bits with the same bound Rust's `to_u32` applies.
 *
 * `Number()` on a bigint widens silently and lossily; every field the Rust
 * decoder narrows must be checked here or the two disagree on validity.
 */
function requireU32(value, field) {
    if (value > 0xffffffffn) {
        throw new Error(`observer field ${field} overflows its 32-bit range`);
    }
    return Number(value);
}
function requiredValue(values, name) {
    return (field) => {
        const found = values.get(field);
        if (found === undefined)
            throw new Error(`missing ${name} field ${field}`);
        return found;
    };
}
function zigzagDecode(value) {
    return (value >> 1n) ^ -(value & 1n);
}
function zigzagEncode(value) {
    return BigInt.asUintN(64, (value << 1n) ^ (value >> 63n));
}
function decodeZigzagI128(input) {
    let encoded = 0n;
    const maximum = (1n << 128n) - 1n;
    for (let index = 0; index < input.length; index += 1) {
        const byte = input[index];
        if (byte === undefined)
            throw new Error("truncated i128 zigzag varint");
        const shift = BigInt(index) * 7n;
        if (shift >= 128n)
            throw new Error("i128 zigzag varint overflows");
        const part = BigInt(byte & 0x7f);
        if (part > (maximum >> shift))
            throw new Error("i128 zigzag varint overflows");
        encoded |= part << shift;
        if ((byte & 0x80) === 0) {
            if (index + 1 !== input.length)
                throw new Error("invalid i128 zigzag varint");
            return zigzagDecode(encoded);
        }
    }
    throw new Error("invalid i128 zigzag varint");
}
function fieldVarint(output, field, value) {
    writeVarint(output, BigInt(field << 3));
    writeVarint(output, value);
}
function fieldBytes(output, field, value) {
    writeVarint(output, BigInt((field << 3) | 2));
    writeVarint(output, BigInt(value.length));
    output.push(...value);
}
function writeVarint(output, initial) {
    let value = initial;
    while (value >= 0x80n) {
        output.push(Number(value & 0x7fn) | 0x80);
        value >>= 7n;
    }
    output.push(Number(value));
}
class Cursor {
    constructor(input) {
        this.input = input;
        this.offset = 0;
    }
    get empty() {
        return this.offset === this.input.length;
    }
    varint() {
        let value = 0n;
        for (let shift = 0n; shift <= 63n; shift += 7n) {
            const byte = this.input[this.offset++];
            if (byte === undefined)
                throw new Error("truncated protobuf varint");
            // The tenth byte contributes only bit 63. A bigint cannot truncate, so
            // without this an over-wide varint would be accepted here and silently
            // truncated by the Rust decoder — the two disagreeing on validity rather
            // than on meaning. Kept identical to `Cursor::varint` in
            // `causafera-observer-wire`.
            if (shift === 63n && (byte & 0x7f) > 1) {
                throw new Error("protobuf varint exceeds 64 bits");
            }
            value |= BigInt(byte & 0x7f) << shift;
            if ((byte & 0x80) === 0)
                return value;
        }
        throw new Error("invalid protobuf varint");
    }
    key() {
        const key = this.varint();
        const rawField = key >> 3n;
        // Bounded before widening, matching Rust's `to_u32(key >> 3)`. `Number()` on
        // a bigint neither rejects nor saturates, so without this a field number
        // past 2^32 is accepted here and rejected there.
        if (rawField > 0xffffffffn)
            throw new Error("protobuf field number overflows");
        const field = Number(rawField);
        if (field === 0)
            throw new Error("invalid protobuf field number");
        return [field, Number(key & 7n)];
    }
    bytes() {
        const length = Number(this.varint());
        const end = this.offset + length;
        if (end > this.input.length)
            throw new Error("truncated protobuf bytes");
        const bytes = this.input.slice(this.offset, end);
        this.offset = end;
        return bytes;
    }
    float64() {
        const end = this.offset + 8;
        if (end > this.input.length)
            throw new Error("truncated protobuf fixed64");
        const value = new DataView(this.input.buffer, this.input.byteOffset + this.offset, 8).getFloat64(0, true);
        this.offset = end;
        return value;
    }
    skip(wire) {
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
    advance(count) {
        this.offset += count;
        if (this.offset > this.input.length)
            throw new Error("truncated protobuf field");
    }
}
