#!/usr/bin/env node
// The live TypeScript decoder against real hydrology payloads.
//
// The Rust side of the wire contract is tested in
// `crates/causafera-observer-wire/tests/protocol.rs`. That says nothing about
// the TypeScript decoder, which is a second independent implementation of the
// same specification — and the failure mode the observer protocol cannot afford
// is the two disagreeing about whether a payload is valid, rather than about
// what it means.
//
// Every payload here is built from bytes, not from either decoder's encoder, so
// a shared mistake in an encoder cannot make a decoder look correct.
//
// Covers `plans/hydrology.md` §12 and V28.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test from 'node:test';

const REPOSITORY_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '..',
  '..',
);
const PACKAGE_ROOT = path.join(REPOSITORY_ROOT, 'packages', 'observer-protocol');

function compileLiveProtocolModule() {
  const outDir = mkdtempSync(path.join(tmpdir(), 'causafera-observer-protocol-hydrology-'));
  const compiler = path.join(PACKAGE_ROOT, 'node_modules', '.bin', 'tsc');
  const compiled = spawnSync(
    compiler,
    [
      'src/index.ts',
      '--outDir',
      outDir,
      '--target',
      'ES2020',
      '--module',
      'ESNext',
      '--moduleResolution',
      'bundler',
    ],
    { cwd: PACKAGE_ROOT, encoding: 'utf8' },
  );
  if (compiled.status !== 0) {
    rmSync(outDir, { recursive: true, force: true });
    throw new Error(
      `observer-protocol did not compile: ${compiled.stdout ?? ''}${compiled.stderr ?? ''}`,
    );
  }
  return { outDir, entry: pathToFileURL(path.join(outDir, 'index.js')).href };
}

const { outDir, entry } = compileLiveProtocolModule();
const live = await import(entry);
process.on('exit', () => rmSync(outDir, { recursive: true, force: true }));

// ---------------------------------------------------------------------------
// Byte-level encoder
// ---------------------------------------------------------------------------

function varint(value) {
  const out = [];
  let remaining = BigInt(value);
  while (remaining >= 0x80n) {
    out.push(Number((remaining & 0x7fn) | 0x80n));
    remaining >>= 7n;
  }
  out.push(Number(remaining));
  return out;
}

const scalar = (field, value) => [...varint(BigInt(field) << 3n), ...varint(value)];
const delimited = (field, bytes) => [
  ...varint((BigInt(field) << 3n) | 2n),
  ...varint(bytes.length),
  ...bytes,
];
const zigzag = (value) => BigInt.asUintN(64, (BigInt(value) << 1n) ^ (BigInt(value) >> 63n));

function varint128(value) {
  const out = [];
  let remaining = BigInt.asUintN(128, BigInt(value));
  while (remaining >= 0x80n) {
    out.push(Number((remaining & 0x7fn) | 0x80n));
    remaining >>= 7n;
  }
  out.push(Number(remaining));
  return out;
}

const zigzagBytes = (value) => {
  const wide = BigInt(value);
  const encoded = wide < 0n ? ((~wide << 1n) | 1n) : wide << 1n;
  return varint128(encoded);
};

/** The 22-byte cell body every cell-shaped carrier key embeds. */
function cellBody(chart, x, y, z, ordinal) {
  const bytes = new Uint8Array(22);
  const view = new DataView(bytes.buffer);
  view.setBigUint64(0, BigInt(chart), false);
  view.setInt32(8, x, false);
  view.setInt32(12, y, false);
  view.setInt32(16, z, false);
  view.setUint16(20, ordinal, false);
  return Array.from(bytes);
}

const cellKey = (ordinal) => [0x01, ...cellBody(1, 0, 0, 0, ordinal)];
const edgeKey = (low, high) => [
  0x02,
  ...cellBody(1, 0, 0, 0, low),
  ...cellBody(1, 0, 0, 0, high),
];
const faceKey = (ordinal, direction) => [0x03, ...cellBody(1, 0, 0, 0, ordinal), direction];
const forcingKey = (tick, id) => {
  const bytes = new Uint8Array(16);
  const view = new DataView(bytes.buffer);
  view.setBigUint64(0, BigInt(tick), false);
  view.setBigUint64(8, BigInt(id), false);
  return [0x04, ...bytes];
};

/** A runtime summary carrying every field the decoder requires. */
function baseSummary() {
  const out = [];
  out.push(...scalar(1, 6));
  out.push(...scalar(2, 8));
  out.push(...delimited(3, new Array(32).fill(0x2a)));
  out.push(...delimited(4, new Array(32).fill(0xfe)));
  for (const field of [5, 6, 7, 8, 9, 10, 11, 12]) out.push(...scalar(field, 3));
  for (let field = 13; field <= 23; field += 1) out.push(...scalar(field, field));
  out.push(...delimited(24, zigzagBytes(-17)));
  out.push(...delimited(25, zigzagBytes(4096)));
  out.push(...scalar(26, 3));
  out.push(...scalar(27, 81));
  return out;
}

function receiptBody(stage, dependencies = []) {
  const out = [];
  out.push(...scalar(1, stage));
  out.push(...scalar(2, stage * 10));
  out.push(...delimited(3, new Array(32).fill(stage)));
  out.push(...scalar(4, 100 + stage));
  for (const dependency of dependencies) out.push(...scalar(5, dependency));
  return out;
}

function bootstrapGroup() {
  const out = [];
  out.push(...scalar(28, 1));
  out.push(...scalar(29, 0x0123456789abcdefn));
  out.push(...scalar(30, 20260730));
  out.push(...scalar(31, 6));
  out.push(...scalar(32, 1));
  out.push(...scalar(33, 64));
  out.push(...scalar(34, 8));
  for (let stage = 1; stage <= 6; stage += 1) {
    out.push(...delimited(35, receiptBody(stage, stage === 1 ? [] : [100 + stage - 1])));
  }
  return out;
}

/** Totals deliberately above `u64::MAX`, which is what a narrowing bug loses. */
const TOTAL_SURFACE = (1n << 64n) + 5n;
const ACCEPTED_SOURCE = (1n << 64n) + 9n;

function hydrologyGroup({ forcing = true } = {}) {
  const out = [];
  out.push(...scalar(36, 1));
  out.push(...delimited(37, varint128(TOTAL_SURFACE)));
  out.push(...delimited(38, varint128(2000n)));
  out.push(...delimited(39, varint128(3000n)));
  out.push(...delimited(40, varint128(4000n)));
  out.push(...delimited(41, zigzagBytes(0)));
  out.push(...scalar(42, 3));
  if (forcing) {
    out.push(...scalar(43, 7));
    out.push(...scalar(44, 11));
    out.push(...scalar(45, 4242));
    out.push(...delimited(46, varint128(ACCEPTED_SOURCE)));
    out.push(...delimited(47, varint(0xffffffffffffffffn)));
  }
  return out;
}

const summaryWithHydrology = (options) =>
  Uint8Array.from([...baseSummary(), ...bootstrapGroup(), ...hydrologyGroup(options)]);

function cellDelta(ordinal, tick) {
  const out = [];
  out.push(...scalar(1, 1));
  out.push(...scalar(2, zigzag(-2)));
  out.push(...scalar(3, zigzag(3)));
  out.push(...scalar(4, zigzag(-4)));
  out.push(...scalar(5, ordinal));
  out.push(...delimited(6, varint(0xffffffffffffffffn)));
  out.push(...delimited(7, varint(0xfffffffffffffffan)));
  out.push(...delimited(8, varint(100)));
  out.push(...delimited(9, varint(105)));
  out.push(...delimited(10, varint(7)));
  out.push(...delimited(11, varint(7)));
  out.push(...delimited(12, zigzagBytes(-9)));
  out.push(...delimited(13, zigzagBytes(9)));
  out.push(...scalar(14, 50 + ordinal));
  out.push(...scalar(15, 900));
  out.push(...scalar(16, tick));
  return out;
}

function transferSummary({
  source = forcingKey(7, 11),
  target = cellKey(0),
  trace = 601,
  tick = 12,
  requested = 1000,
  accepted = 600,
  unaccepted = 400,
} = {}) {
  const out = [];
  out.push(...scalar(1, 7));
  out.push(...delimited(2, source));
  out.push(...delimited(3, target));
  out.push(...delimited(4, varint(requested)));
  out.push(...delimited(5, varint(accepted)));
  out.push(...delimited(6, varint(unaccepted)));
  out.push(...scalar(7, trace));
  out.push(...scalar(8, 900));
  out.push(...scalar(9, tick));
  out.push(...scalar(10, 4242));
  return out;
}

function conveyanceSummary(low = 0, high = 1, tick = 12) {
  const out = [];
  out.push(...delimited(1, edgeKey(low, high)));
  out.push(...delimited(2, varint(500)));
  out.push(...delimited(3, varint(1000)));
  out.push(...delimited(4, varint(40)));
  out.push(...delimited(5, varint(30)));
  out.push(...scalar(6, 901));
  out.push(...scalar(7, tick));
  return out;
}

function worldSnapshot(overrides = {}) {
  const {
    deltas = [cellDelta(0, 12), cellDelta(1, 12)],
    transfers = [
      transferSummary(),
      // A vertical process: one cell is legitimately both endpoints.
      transferSummary({ source: cellKey(0), target: cellKey(0), trace: 602 }),
      transferSummary({ source: cellKey(0), target: faceKey(0, 3), trace: 603 }),
    ],
    conveyance = [conveyanceSummary()],
    deltaSchema = 1,
    transferSchema = 1,
    conveyanceSchema = 1,
  } = overrides;
  const out = [];
  out.push(...scalar(1, 12));
  for (const delta of deltas) out.push(...delimited(9, delta));
  if (deltaSchema !== null) out.push(...scalar(10, deltaSchema));
  for (const summary of transfers) out.push(...delimited(11, summary));
  if (transferSchema !== null) out.push(...scalar(12, transferSchema));
  for (const summary of conveyance) out.push(...delimited(13, summary));
  if (conveyanceSchema !== null) out.push(...scalar(14, conveyanceSchema));
  return Uint8Array.from(out);
}

function hydrologyRaster(values = [0n, 0xffffffffffffffffn, 1n << 63n, 42n], overrides = {}) {
  const { field = 4, schema = 1, signed = null } = overrides;
  const out = [];
  out.push(...scalar(1, 1));
  out.push(...scalar(2, zigzag(-2)));
  out.push(...scalar(3, zigzag(3)));
  out.push(...scalar(4, zigzag(-4)));
  out.push(...scalar(5, field));
  out.push(...scalar(6, 0));
  out.push(...scalar(7, 2));
  out.push(...scalar(8, 1));
  if (signed !== null) {
    const band = [];
    let previous = 0n;
    for (const value of signed) {
      band.push(...varint(zigzag(BigInt(value) - previous)));
      previous = BigInt(value);
    }
    out.push(...delimited(9, band));
  }
  out.push(...scalar(12, 77));
  const packed = [];
  for (const value of values) packed.push(...varint(value));
  out.push(...delimited(13, packed));
  if (schema !== null) out.push(...scalar(14, schema));
  return Uint8Array.from(out);
}

// ---------------------------------------------------------------------------

test('the constants match the Rust contract', () => {
  assert.equal(live.HYDROLOGY_SUMMARY_SCHEMA_ABSENT, 0);
  assert.equal(live.HYDROLOGY_SUMMARY_SCHEMA_V1, 1);
  assert.equal(live.HYDROLOGY_DELTA_SCHEMA_V1, 1);
  assert.equal(live.HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1, 1);
  assert.equal(live.HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1, 1);
  assert.equal(live.HYDROLOGY_RASTER_VALUES_SCHEMA_V1, 1);
  assert.equal(live.MAX_HYDROLOGY_DELTAS, 64);
  assert.equal(live.MAX_HYDROLOGY_TRANSFER_SUMMARIES, 64);
  assert.equal(live.MAX_HYDROLOGY_CONVEYANCE_SUMMARIES, 64);
  assert.equal(live.MAX_QUERY_RESPONSE_PAYLOAD_BYTES, 1 << 20);
});

test('a full hydrology summary decodes with every total exact', () => {
  const decoded = live.decodeRuntimeSummary(summaryWithHydrology());

  assert.equal(decoded.hydrology.schemaVersion, 1);
  // The value a `number` would round and a `u64` would wrap.
  assert.equal(decoded.hydrology.totalSurface, TOTAL_SURFACE);
  assert.equal(decoded.hydrology.totalSoil, 2000n);
  assert.equal(decoded.hydrology.latestResidual, 0n);
  assert.equal(decoded.hydrology.activeChunkCount, 3);
  assert.equal(decoded.hydrology.latestForcing.tick, 7n);
  assert.equal(decoded.hydrology.latestForcing.forcingId, 11n);
  assert.equal(decoded.hydrology.latestForcing.originTrace, 4242n);
  assert.equal(decoded.hydrology.latestForcing.acceptedSource, ACCEPTED_SOURCE);
  assert.equal(decoded.hydrology.latestForcing.acceptedEvapotranspiration, 0xffffffffffffffffn);
  // The older fields keep their meanings beside it.
  assert.equal(decoded.simulationTicks, 6n);
  assert.equal(decoded.bootstrap.receipts.length, 6);
});

test('a payload written before hydrology existed decodes as absent', () => {
  const legacy = Uint8Array.from([...baseSummary(), ...bootstrapGroup()]);
  const decoded = live.decodeRuntimeSummary(legacy);

  assert.equal(decoded.hydrology.schemaVersion, live.HYDROLOGY_SUMMARY_SCHEMA_ABSENT);
  assert.equal(decoded.hydrology.totalSurface, 0n);
  assert.equal(decoded.hydrology.latestForcing, null);
  assert.equal(decoded.bootstrap.stageCount, 6);
});

test('a summary with no applied forcing record omits the whole group', () => {
  const decoded = live.decodeRuntimeSummary(summaryWithHydrology({ forcing: false }));
  assert.equal(decoded.hydrology.schemaVersion, 1);
  assert.equal(decoded.hydrology.latestForcing, null);
});

test('a partially present hydrology group is rejected', () => {
  const complete = [...baseSummary(), ...bootstrapGroup(), ...hydrologyGroup()];
  for (const dropped of [36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47]) {
    const partial = [];
    let index = 0;
    const parts = [
      [36, scalar(36, 1)],
      [37, delimited(37, varint128(TOTAL_SURFACE))],
      [38, delimited(38, varint128(2000n))],
      [39, delimited(39, varint128(3000n))],
      [40, delimited(40, varint128(4000n))],
      [41, delimited(41, zigzagBytes(0))],
      [42, scalar(42, 3)],
      [43, scalar(43, 7)],
      [44, scalar(44, 11)],
      [45, scalar(45, 4242)],
      [46, delimited(46, varint128(ACCEPTED_SOURCE))],
      [47, delimited(47, varint(1))],
    ];
    partial.push(...baseSummary(), ...bootstrapGroup());
    for (const [field, bytes] of parts) {
      if (field !== dropped) partial.push(...bytes);
      index += 1;
    }
    assert.equal(index, parts.length);
    assert.throws(
      () => live.decodeRuntimeSummary(Uint8Array.from(partial)),
      undefined,
      `a summary missing field ${dropped} must be rejected`,
    );
  }
  // The complete group is accepted, so the loop above is not vacuous.
  assert.ok(live.decodeRuntimeSummary(Uint8Array.from(complete)));
});

test('a forcing group with no summary to attribute it to is rejected', () => {
  const orphaned = Uint8Array.from([
    ...baseSummary(),
    ...bootstrapGroup(),
    ...scalar(43, 7),
    ...scalar(44, 11),
    ...scalar(45, 4242),
    ...delimited(46, varint128(1n)),
    ...delimited(47, varint(1)),
  ]);
  assert.throws(() => live.decodeRuntimeSummary(orphaned));
});

test('an unknown hydrology schema, duplicate, or wire type is rejected', () => {
  for (const version of [0, 2, 99]) {
    const tampered = Uint8Array.from([
      ...baseSummary(),
      ...bootstrapGroup(),
      ...scalar(36, version),
      ...hydrologyGroup().slice(scalar(36, 1).length),
    ]);
    assert.throws(
      () => live.decodeRuntimeSummary(tampered),
      undefined,
      `hydrology schema ${version} must be rejected`,
    );
  }
  const duplicated = Uint8Array.from([...summaryWithHydrology(), ...scalar(42, 4)]);
  assert.throws(() => live.decodeRuntimeSummary(duplicated), /duplicate/);

  // A scalar arriving length-delimited and a byte integer arriving as a varint.
  const mistyped = Uint8Array.from([
    ...baseSummary(),
    ...bootstrapGroup(),
    ...delimited(36, varint(1)),
  ]);
  assert.throws(() => live.decodeRuntimeSummary(mistyped), /wrong wire type/);
  const mistypedBytes = Uint8Array.from([
    ...baseSummary(),
    ...bootstrapGroup(),
    ...scalar(37, 1),
  ]);
  assert.throws(() => live.decodeRuntimeSummary(mistypedBytes), /wrong wire type/);
});

test('a noncanonical byte integer is rejected', () => {
  // `[0x80, 0x00]` is zero written in two bytes: a second encoding of one value,
  // which would give one projection two byte strings.
  for (const field of [37, 41, 47]) {
    const parts = hydrologyGroup();
    const tampered = [...baseSummary(), ...bootstrapGroup()];
    for (const chunk of [
      scalar(36, 1),
      field === 37 ? delimited(37, [0x80, 0x00]) : delimited(37, varint128(1n)),
      delimited(38, varint128(2n)),
      delimited(39, varint128(3n)),
      delimited(40, varint128(4n)),
      field === 41 ? delimited(41, [0x80, 0x00]) : delimited(41, zigzagBytes(0)),
      scalar(42, 1),
      scalar(43, 7),
      scalar(44, 11),
      scalar(45, 4242),
      delimited(46, varint128(1n)),
      field === 47 ? delimited(47, [0x80, 0x00]) : delimited(47, varint(1)),
    ]) {
      tampered.push(...chunk);
    }
    assert.ok(parts.length > 0);
    assert.throws(
      () => live.decodeRuntimeSummary(Uint8Array.from(tampered)),
      /shortest canonical form/,
      `a noncanonical zero in field ${field} must be rejected`,
    );
  }
});

test('a byte integer outside its declared domain is rejected', () => {
  // Field 47 is a `u64`; ten payload bytes describe a wider value.
  const tooWide = Uint8Array.from([
    ...baseSummary(),
    ...bootstrapGroup(),
    ...hydrologyGroup({ forcing: false }),
    ...scalar(43, 7),
    ...scalar(44, 11),
    ...scalar(45, 4242),
    ...delimited(46, varint128(1n)),
    ...delimited(47, [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
  ]);
  assert.throws(() => live.decodeRuntimeSummary(tooWide), /overflows its domain/);
});

test('the world projection decodes every hydrology section', () => {
  const decoded = live.decodeWorldChunkSnapshot(worldSnapshot());

  assert.equal(decoded.hydrologyDeltaSchemaVersion, 1);
  assert.equal(decoded.hydrologyDeltas.length, 2);
  assert.equal(decoded.hydrologyDeltas[0].surfaceBefore, 0xffffffffffffffffn);
  assert.equal(decoded.hydrologyDeltas[0].chunkX, -2);
  assert.equal(decoded.hydrologyDeltas[0].netForcing, -9n);
  assert.equal(decoded.hydrologyTransferSummaries.length, 3);
  assert.equal(decoded.hydrologyTransferSummaries[0].acceptedVolume, 600n);
  assert.equal(decoded.hydrologyTransferSummaries[0].forcingOriginTraceId, 4242n);
  assert.equal(decoded.hydrologyConveyanceSummaries.length, 1);
  assert.equal(decoded.hydrologyConveyanceSummaries[0].storage, 500n);
});

test('entries without a schema to interpret them are rejected', () => {
  assert.throws(() => live.decodeWorldChunkSnapshot(worldSnapshot({ deltaSchema: null })));
  assert.throws(() => live.decodeWorldChunkSnapshot(worldSnapshot({ transferSchema: null })));
  assert.throws(() => live.decodeWorldChunkSnapshot(worldSnapshot({ conveyanceSchema: null })));
});

test('every hydrology projection bound rejects one past its limit', () => {
  const deltas = Array.from({ length: 64 }, (_, index) => cellDelta(index, 12));
  assert.ok(live.decodeWorldChunkSnapshot(worldSnapshot({ deltas })));
  assert.throws(
    () => live.decodeWorldChunkSnapshot(worldSnapshot({ deltas: [...deltas, cellDelta(64, 12)] })),
    /exceed their bound/,
  );

  const transfers = Array.from({ length: 64 }, (_, index) =>
    transferSummary({ source: cellKey(0), target: cellKey(1), trace: 700 + index }),
  );
  assert.ok(live.decodeWorldChunkSnapshot(worldSnapshot({ transfers })));
  assert.throws(
    () =>
      live.decodeWorldChunkSnapshot(
        worldSnapshot({ transfers: [...transfers, transferSummary({ trace: 999 })] }),
      ),
    /exceed their bound/,
  );

  const conveyance = Array.from({ length: 64 }, (_, index) =>
    conveyanceSummary(index, index + 100),
  );
  assert.ok(live.decodeWorldChunkSnapshot(worldSnapshot({ conveyance })));
  assert.throws(
    () =>
      live.decodeWorldChunkSnapshot(
        worldSnapshot({ conveyance: [...conveyance, conveyanceSummary(200, 201)] }),
      ),
    /exceed their bound/,
  );
});

test('a duplicated hydrology key is rejected', () => {
  assert.throws(
    () => live.decodeWorldChunkSnapshot(worldSnapshot({ deltas: [cellDelta(0, 12), cellDelta(0, 12)] })),
    /duplicate hydrology cell delta key/,
  );
  assert.throws(
    () =>
      live.decodeWorldChunkSnapshot(
        worldSnapshot({
          transfers: [
            transferSummary({ source: cellKey(0), target: cellKey(1), trace: 601 }),
            transferSummary({ source: cellKey(0), target: cellKey(1), trace: 601 }),
          ],
        }),
      ),
    /duplicate hydrology transfer key/,
  );
  assert.throws(
    () =>
      live.decodeWorldChunkSnapshot(
        worldSnapshot({ conveyance: [conveyanceSummary(), conveyanceSummary()] }),
      ),
    /duplicate hydrology conveyance key/,
  );
  // The same cell at a different tick is a different row, not a duplicate.
  assert.ok(
    live.decodeWorldChunkSnapshot(worldSnapshot({ deltas: [cellDelta(0, 12), cellDelta(0, 11)] })),
  );
});

test('a malformed carrier key is rejected', () => {
  const cases = [
    ['an unknown variant', new Array(23).fill(0x09)],
    ['a cell key one byte short', cellKey(0).slice(0, 22)],
    ['a cell key one byte long', [...cellKey(0), 0]],
    ['an unknown face direction', faceKey(0, 4)],
    ['a reversed edge', edgeKey(3, 1)],
    ['an edge from a cell to itself', edgeKey(2, 2)],
  ];
  for (const [what, key] of cases) {
    assert.throws(
      () =>
        live.decodeWorldChunkSnapshot(
          worldSnapshot({ transfers: [transferSummary({ source: key })] }),
        ),
      undefined,
      `${what} must be rejected`,
    );
  }
});

test('a transfer between one non-cell carrier and itself is rejected', () => {
  // A cell may legitimately be both endpoints; every other carrier may not.
  assert.ok(
    live.decodeWorldChunkSnapshot(
      worldSnapshot({ transfers: [transferSummary({ source: cellKey(0), target: cellKey(0) })] }),
    ),
  );
  for (const key of [edgeKey(0, 1), faceKey(0, 2), forcingKey(3, 4)]) {
    assert.throws(
      () =>
        live.decodeWorldChunkSnapshot(
          worldSnapshot({ transfers: [transferSummary({ source: key, target: key })] }),
        ),
      /same carrier as source and target/,
    );
  }
});

test('a transfer whose volumes do not close is rejected', () => {
  assert.throws(
    () =>
      live.decodeWorldChunkSnapshot(
        worldSnapshot({
          transfers: [transferSummary({ requested: 100, accepted: 101, unaccepted: 0 })],
        }),
      ),
    /do not close/,
  );
  assert.throws(
    () =>
      live.decodeWorldChunkSnapshot(
        worldSnapshot({
          transfers: [transferSummary({ requested: 100, accepted: 40, unaccepted: 50 })],
        }),
      ),
    /do not close/,
  );
});

test('a conveyance summary that does not name an edge is rejected', () => {
  const wrongCarrier = [
    ...delimited(1, cellKey(0)),
    ...delimited(2, varint(1)),
    ...delimited(3, varint(2)),
    ...delimited(4, varint(3)),
    ...delimited(5, varint(4)),
    ...scalar(6, 901),
    ...scalar(7, 12),
  ];
  assert.throws(
    () => live.decodeWorldChunkSnapshot(worldSnapshot({ conveyance: [wrongCarrier] })),
    /must name an edge/,
  );
});

test('an unsigned raster band round-trips values above the signed ceiling', () => {
  const decoded = live.decodeFieldRaster(hydrologyRaster());

  assert.equal(decoded.unsignedValuesSchemaVersion, 1);
  assert.ok(decoded.unsignedValues instanceof BigUint64Array);
  assert.equal(decoded.unsignedValues[1], 0xffffffffffffffffn);
  assert.equal(decoded.unsignedValues[2], 1n << 63n);
  assert.equal(decoded.values.length, 0);
  assert.equal(decoded.field, live.FieldRasterKind.HydrologySurfaceWater);
});

test('the two raster bands are mutually exclusive', () => {
  // A hydrology raster carrying the signed band would have rounded every volume
  // past 2^53 on the way in.
  assert.throws(
    () => live.decodeFieldRaster(hydrologyRaster(undefined, { signed: [1, 2, 3, 4] })),
    /must not carry the signed bands/,
  );
  // A terrain raster carrying the unsigned band would have lost every elevation
  // below sea level.
  assert.throws(
    () => live.decodeFieldRaster(hydrologyRaster(undefined, { field: 1, signed: [-1, 2, -3, 4] })),
    /must not carry the unsigned band/,
  );
});

test('a hydrology raster without its schema marker or with a short band is rejected', () => {
  assert.throws(() => live.decodeFieldRaster(hydrologyRaster(undefined, { schema: null })), /field 14/);
  assert.throws(
    () => live.decodeFieldRaster(hydrologyRaster(undefined, { schema: 2 })),
    /unsupported hydrology raster values schema/,
  );
  assert.throws(() => live.decodeFieldRaster(hydrologyRaster([1n, 2n, 3n])), /declares 4 cells/);
  assert.throws(
    () => live.decodeFieldRaster(hydrologyRaster([1n, 2n, 3n, 4n, 5n])),
    /declares 4 cells/,
  );
});

test('a noncanonical value in the unsigned band is rejected', () => {
  const out = [];
  out.push(...scalar(1, 1));
  out.push(...scalar(5, 4));
  out.push(...scalar(7, 2));
  out.push(...scalar(8, 1));
  // Four values, the last of them zero written in two bytes.
  out.push(...delimited(13, [1, 2, 3, 0x80, 0x00]));
  out.push(...scalar(14, 1));
  assert.throws(
    () => live.decodeFieldRaster(Uint8Array.from(out)),
    /shortest canonical form/,
  );
});

test('a response payload past the cap is refused before it is allocated', () => {
  const oversized = new Array(live.MAX_QUERY_RESPONSE_PAYLOAD_BYTES + 1).fill(0);
  const forged = Uint8Array.from([
    ...scalar(1, 1),
    ...scalar(2, 1),
    ...scalar(3, 1),
    ...delimited(4, oversized),
  ]);
  assert.throws(() => live.decodeQueryResponse(forged), /exceeds its cap/);

  const atCap = Uint8Array.from([
    ...scalar(1, 1),
    ...scalar(2, 1),
    ...scalar(3, 1),
    ...delimited(4, new Array(live.MAX_QUERY_RESPONSE_PAYLOAD_BYTES).fill(0)),
  ]);
  assert.equal(
    live.decodeQueryResponse(atCap).payload.length,
    live.MAX_QUERY_RESPONSE_PAYLOAD_BYTES,
  );
});

test('the carrier key validator agrees with its own length table', () => {
  assert.equal(live.validateHydrologyCarrierKey(Uint8Array.from(cellKey(0))), 1);
  assert.equal(live.validateHydrologyCarrierKey(Uint8Array.from(edgeKey(0, 1))), 2);
  assert.equal(live.validateHydrologyCarrierKey(Uint8Array.from(faceKey(0, 3))), 3);
  assert.equal(live.validateHydrologyCarrierKey(Uint8Array.from(forcingKey(1, 2))), 4);
  assert.throws(() => live.validateHydrologyCarrierKey(new Uint8Array()), /empty/);
  assert.throws(() => live.validateHydrologyCarrierKey(Uint8Array.from([0x07])), /unknown/);

  // The endpoint comparison decodes the coordinates rather than comparing bytes:
  // a two's-complement negative coordinate's leading byte sorts above a positive
  // one's, so a byte-wise check would call this pair reversed.
  const negativeFirst = Uint8Array.from([
    0x02,
    ...cellBody(1, -1, 0, 0, 0),
    ...cellBody(1, 1, 0, 0, 0),
  ]);
  assert.equal(live.validateHydrologyCarrierKey(negativeFirst), 2);
  const positiveFirst = Uint8Array.from([
    0x02,
    ...cellBody(1, 1, 0, 0, 0),
    ...cellBody(1, -1, 0, 0, 0),
  ]);
  assert.throws(() => live.validateHydrologyCarrierKey(positiveFirst), /canonical endpoint order/);
});

// ---------------------------------------------------------------------------
// A real engine payload
// ---------------------------------------------------------------------------

// Every payload above was built from bytes by this file, which proves the
// decoder consistent with an audit rather than with the producer. These bytes
// came out of a production-bootstrapped runtime after three committed hydrology
// ticks. A Rust test regenerates and compares them, so the capture cannot
// quietly drift away from what the engine emits.
const CAPTURE = JSON.parse(
  readFileSync(
    path.join(REPOSITORY_ROOT, 'tools', 'audit', 'fixtures', 'observer-hydrology-engine-payload.json'),
    'utf8',
  ),
);

function fromHex(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}

test('the TypeScript decoder reads a real engine summary', () => {
  const decoded = live.decodeRuntimeSummary(fromHex(CAPTURE.summary_hex));

  assert.equal(decoded.hydrology.schemaVersion, live.HYDROLOGY_SUMMARY_SCHEMA_V1);
  assert.equal(decoded.hydrology.totalSurface.toString(), CAPTURE.hydrologyTotalSurface);
  // A committed batch closes exactly, and the engine says so.
  assert.equal(decoded.hydrology.latestResidual, 0n);
  assert.ok(decoded.hydrology.activeChunkCount > 0);
  // The bootstrap projection beside it is the frozen six-stage one.
  assert.equal(decoded.bootstrap.stageCount, 6);
  assert.equal(decoded.bootstrap.receipts.length, 6);
  assert.notEqual(decoded.bootstrap.stageSeven, null);
  assert.equal(decoded.bootstrap.stageSeven.stage, 7n);
});

test('the TypeScript decoder reads a real engine world projection', () => {
  const decoded = live.decodeWorldChunkSnapshot(fromHex(CAPTURE.world_hex));

  assert.equal(decoded.hydrologyDeltas.length, CAPTURE.hydrologyDeltaCount);
  assert.equal(decoded.hydrologyTransferSummaries.length, CAPTURE.hydrologyTransferCount);
  assert.equal(decoded.hydrologyConveyanceSummaries.length, CAPTURE.hydrologyConveyanceCount);
  assert.equal(decoded.hydrologyDeltaSchemaVersion, 1);

  for (const summary of decoded.hydrologyTransferSummaries) {
    // The producer's keys pass this decoder's own independent validator, which
    // is what ties the two implementations together on real bytes.
    live.validateHydrologyCarrierKey(summary.sourceKey);
    live.validateHydrologyCarrierKey(summary.targetKey);
    assert.equal(
      summary.requestedVolume - summary.acceptedVolume,
      summary.unacceptedVolume,
    );
    assert.notEqual(summary.transferTraceId, 0n);
  }
  for (const delta of decoded.hydrologyDeltas) {
    assert.notEqual(delta.transitionTraceId, 0n);
    assert.notEqual(delta.conservationTraceId, 0n);
  }
  for (const summary of decoded.hydrologyConveyanceSummaries) {
    assert.equal(live.validateHydrologyCarrierKey(summary.edgeKey), live.HYDROLOGY_CARRIER_EDGE);
    assert.ok(summary.storage <= summary.capacity);
  }
});

test('the TypeScript decoder reads a real engine water raster', () => {
  const decoded = live.decodeFieldRaster(fromHex(CAPTURE.raster_hex));

  assert.equal(decoded.field, live.FieldRasterKind.HydrologySurfaceWater);
  assert.equal(decoded.unsignedValuesSchemaVersion, live.HYDROLOGY_RASTER_VALUES_SCHEMA_V1);
  assert.ok(decoded.unsignedValues instanceof BigUint64Array);
  assert.equal(decoded.unsignedValues.length, decoded.edge * decoded.edge * decoded.depth);
  assert.equal(decoded.values.length, 0, 'a water lattice carries no signed band');
  assert.equal(decoded.cellTraces.length, decoded.unsignedValues.length);
  // The lattice holds real water, so a decoder that returned an empty band
  // could not have passed the length check above by accident.
  assert.ok(
    Array.from(decoded.unsignedValues).some((value) => value > 0n),
    'the captured chunk holds water',
  );
});
