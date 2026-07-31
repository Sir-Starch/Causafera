#!/usr/bin/env node
// The frozen pre-hydrology observer V1 decoder, exercised as an oracle.
//
// `plans/hydrology.md` keeps the observer protocol at V1 across the hydrology
// additions, which is a claim about clients that were built before those
// additions existed. The current TypeScript decoder cannot test that claim
// about itself, so Stage 1 freezes it as
// `tools/audit/fixtures/observer-protocol-v1-pre-hydrology.mjs` and this file
// drives that copy.
//
// At Stage 1 there is no hydrology payload yet, so the work here is to make the
// freeze trustworthy before it is relied on:
//
//   1. the frozen file is byte-identical to what was frozen (pinned digest), so
//      "the old decoder accepts it" can never quietly become "the new decoder
//      accepts it"; and
//   2. the frozen copy and the live decoder agree, field for field, on real
//      payloads and on malformed ones — the evidence that the copy is faithful
//      rather than merely plausible.
//
// Stage 7 adds the actual compatibility claim at the end of this file: the
// frozen decoder ignores hydrology fields 36-48 on a new-engine payload, reads
// the same summary it read without them, and still holds field 35 to six
// bootstrap receipts.

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
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
const FROZEN_ORACLE = path.join(
  REPOSITORY_ROOT,
  'tools',
  'audit',
  'fixtures',
  'observer-protocol-v1-pre-hydrology.mjs',
);

/**
 * The frozen oracle's own digest. Editing the oracle is a test failure, not a
 * re-freeze: an oracle that moves with the code it checks stops being one.
 */
const FROZEN_ORACLE_SHA256 =
  '830c3f2e8328b0f2834a84e936a2b14dc046e8900651ebb1baba7daf0b31f2d1';

/** The live decoder's source at the moment of the freeze, for provenance. */
const FROZEN_FROM_SOURCE_SHA256 =
  '79c351d9cdf363d300d460a46b3628a998308ab746da1853bb9ac1340322fa76';

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function compileLiveProtocolModule() {
  const outDir = mkdtempSync(path.join(tmpdir(), 'causafera-observer-protocol-live-'));
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
const frozen = await import(pathToFileURL(FROZEN_ORACLE).href);
process.on('exit', () => rmSync(outDir, { recursive: true, force: true }));

// ---------------------------------------------------------------------------
// Minimal encoder, built from bytes rather than from either decoder's inverse.
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

/** A runtime summary carrying every field the decoder requires. */
function baseSummary() {
  const out = [];
  out.push(...scalar(1, 6));
  out.push(...scalar(2, 7));
  out.push(...delimited(3, new Array(32).fill(0x2a)));
  out.push(...delimited(4, new Array(32).fill(0xfe)));
  for (const field of [5, 6, 7, 8, 9, 10, 11, 12]) out.push(...scalar(field, 3));
  for (let field = 13; field <= 23; field += 1) out.push(...scalar(field, field));
  out.push(...delimited(24, varint(zigzag(-17))));
  out.push(...delimited(25, varint(zigzag(4096))));
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

/** The complete six-stage bootstrap group the current engine produces. */
function bootstrapGroup(stages = 6) {
  const out = [];
  out.push(...scalar(28, 1));
  out.push(...scalar(29, 0x0123456789abcdefn));
  out.push(...scalar(30, 20260729));
  out.push(...scalar(31, stages));
  out.push(...scalar(32, stages > 0 ? 1 : 0));
  out.push(...scalar(33, 64));
  out.push(...scalar(34, 8));
  for (let stage = 1; stage <= stages; stage += 1) {
    out.push(...delimited(35, receiptBody(stage, stage === 1 ? [] : [100 + stage - 1])));
  }
  return out;
}

function chunkSummary(chunkX) {
  const out = [];
  out.push(...scalar(1, 1));
  out.push(...scalar(2, zigzag(chunkX)));
  out.push(...scalar(3, zigzag(0)));
  out.push(...scalar(4, zigzag(0)));
  out.push(...scalar(5, zigzag(-32800)));
  out.push(...scalar(6, zigzag(36900)));
  out.push(...scalar(7, 1617));
  out.push(...scalar(8, zigzag(2570)));
  out.push(...scalar(9, zigzag(11)));
  out.push(...scalar(10, 0));
  out.push(...scalar(11, 64));
  out.push(...scalar(12, 128));
  out.push(...scalar(13, 4096));
  return out;
}

function worldChunkSnapshot() {
  const out = [];
  out.push(...scalar(1, 6));
  for (const chunkX of [-1, 0, 1]) out.push(...delimited(2, chunkSummary(chunkX)));
  out.push(...scalar(4, 4));
  out.push(...scalar(7, 1));
  return out;
}

function deltaBand(values) {
  const out = [];
  let previous = 0n;
  for (const value of values) {
    out.push(...varint(zigzag(BigInt.asIntN(64, BigInt(value) - previous))));
    previous = BigInt(value);
  }
  return out;
}

function fieldRaster() {
  const out = [];
  out.push(...scalar(1, 1));
  out.push(...scalar(2, zigzag(0)));
  out.push(...scalar(3, zigzag(0)));
  out.push(...scalar(4, zigzag(0)));
  out.push(...scalar(5, 1));
  out.push(...scalar(6, 0));
  out.push(...scalar(7, 2));
  out.push(...scalar(8, 1));
  out.push(...delimited(9, deltaBand([-13500, -13700, 13100, 19500])));
  out.push(...delimited(10, deltaBand([1, 2, 3, 4])));
  out.push(...delimited(11, [...varint(7), ...varint(8), ...varint(9), ...varint(10)]));
  out.push(...scalar(12, 4242));
  return out;
}

/** Every payload shape the frozen oracle and the live decoder both parse. */
const VECTORS = [
  {
    what: 'a six-receipt runtime summary',
    decoder: 'decodeRuntimeSummary',
    bytes: [...baseSummary(), ...bootstrapGroup(6)],
  },
  {
    what: 'a runtime summary written before the bootstrap group existed',
    decoder: 'decodeRuntimeSummary',
    bytes: baseSummary(),
  },
  {
    what: 'a three-chunk world snapshot',
    decoder: 'decodeWorldChunkSnapshot',
    bytes: worldChunkSnapshot(),
  },
  {
    what: 'a two-by-two elevation raster with both bands',
    decoder: 'decodeFieldRaster',
    bytes: fieldRaster(),
  },
  {
    what: 'a query response envelope',
    decoder: 'decodeQueryResponse',
    bytes: [
      ...scalar(1, 9),
      ...scalar(2, 1),
      ...scalar(3, 1),
      ...delimited(4, baseSummary()),
    ],
  },
];

/** Payloads both decoders must refuse, for the same reasons. */
const REJECTIONS = [
  {
    what: 'a summary claiming a seventh bootstrap receipt',
    decoder: 'decodeRuntimeSummary',
    bytes: [...baseSummary(), ...bootstrapGroup(7)],
  },
  {
    what: 'a partially present bootstrap group',
    decoder: 'decodeRuntimeSummary',
    bytes: [...baseSummary(), ...scalar(28, 1), ...scalar(29, 1)],
  },
  {
    what: 'bootstrap receipts with no summary to interpret them',
    decoder: 'decodeRuntimeSummary',
    bytes: [...baseSummary(), ...delimited(35, receiptBody(1))],
  },
  {
    what: 'a bootstrap field on the wrong wire type',
    decoder: 'decodeRuntimeSummary',
    bytes: [...baseSummary(), ...delimited(31, varint(6))],
  },
  {
    what: 'a raster whose value band does not fill its declared lattice',
    decoder: 'decodeFieldRaster',
    bytes: [
      ...scalar(1, 1),
      ...scalar(5, 1),
      ...scalar(7, 4),
      ...scalar(8, 1),
      ...delimited(9, deltaBand([1, 2, 3])),
    ],
  },
  {
    what: 'a varint wider than 64 bits',
    decoder: 'decodeQueryResponse',
    bytes: [
      ...varint(1n << 3n),
      0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f,
    ],
  },
];

/** Structural comparison that survives typed arrays and bigints. */
function normalize(value) {
  if (typeof value === 'bigint') return `bigint:${value}`;
  if (value instanceof Uint8Array) return `u8:${Array.from(value).join(',')}`;
  if (value instanceof Float64Array) return `f64:${Array.from(value).join(',')}`;
  if (value instanceof BigUint64Array) {
    return `u64:${Array.from(value, (entry) => entry.toString()).join(',')}`;
  }
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, normalize(value[key])]),
    );
  }
  return value;
}

// ---------------------------------------------------------------------------

test('the frozen oracle is byte-identical to what was frozen', () => {
  const bytes = readFileSync(FROZEN_ORACLE);
  assert.equal(
    sha256(bytes),
    FROZEN_ORACLE_SHA256,
    'the pre-hydrology decoder oracle was edited; it must be replaced only by a deliberate re-freeze that also moves this digest',
  );
});

test('the freeze records the live source it was taken from', () => {
  const header = readFileSync(FROZEN_ORACLE, 'utf8').slice(0, 2048);
  assert.match(header, /FROZEN ARTEFACT/);
  assert.ok(
    header.includes(FROZEN_FROM_SOURCE_SHA256),
    'the oracle must state which packages/observer-protocol/src/index.ts it was emitted from',
  );
});

test('the frozen oracle carries the pre-hydrology V1 bounds', () => {
  // These are exactly the bounds `plans/hydrology.md` promises not to widen:
  // the seventh bootstrap receipt goes in its own optional field instead.
  assert.equal(frozen.OBSERVER_PROTOCOL_V1, 1);
  assert.equal(frozen.MAX_BOOTSTRAP_RECEIPT_SUMMARIES, 6);
  assert.equal(frozen.MAX_BOOTSTRAP_RECEIPT_DEPENDENCIES, 8);
  assert.equal(frozen.MAX_MATERIAL_SURFACE_DELTAS, 64);
  assert.equal(frozen.MAX_THERMAL_DELTAS, 64);
  // The frozen copy knows nothing about hydrology, which is the whole point.
  assert.equal(frozen.HYDROLOGY_SUMMARY_SCHEMA_V1, undefined);
  assert.equal(frozen.FieldRasterKind.ManaIntensity, 3);
  assert.equal(Object.keys(frozen.FieldRasterKind).length, 6);
});

test('the frozen oracle and the live decoder agree on every valid payload', () => {
  assert.ok(VECTORS.length > 0);
  for (const { what, decoder, bytes } of VECTORS) {
    const input = Uint8Array.from(bytes);
    const frozenValue = frozen[decoder](input);
    const liveValue = live[decoder](input);
    assert.deepEqual(
      normalize(frozenValue),
      normalize(withoutAdditions(liveValue)),
      `${what} must decode identically in the frozen and live decoders`,
    );
  }
});

/**
 * The live decoder's output with the fields added after the freeze removed.
 *
 * There are exactly two: the summary's `stageSeven`, which reports the appended
 * field-48 bootstrap stage, and its `hydrology` group. The frozen decoder cannot
 * have a key for a field that did not exist when it was frozen, so comparing the
 * two without removing them would report the *presence* of an additive field as
 * a divergence, which is the opposite of what this audit is for. The removal is
 * deliberately narrow: a field the live decoder added anywhere else still fails
 * the comparison.
 */
function withoutAdditions(value) {
  if (value === null || typeof value !== 'object') return value;
  let stripped = value;
  if ('hydrology' in stripped) {
    const { hydrology, ...rest } = stripped;
    assert.ok(
      hydrology !== null && typeof hydrology === 'object',
      'hydrology is a summary object',
    );
    stripped = rest;
  }
  if ('unsignedValues' in stripped) {
    const { unsignedValues, unsignedValuesSchemaVersion, ...rest } = stripped;
    assert.ok(unsignedValues instanceof BigUint64Array, 'the unsigned band is a BigUint64Array');
    assert.equal(typeof unsignedValuesSchemaVersion, 'number');
    stripped = rest;
  }
  for (const key of [
    'hydrologyDeltas',
    'hydrologyDeltaSchemaVersion',
    'hydrologyTransferSummaries',
    'hydrologyTransferSchemaVersion',
    'hydrologyConveyanceSummaries',
    'hydrologyConveyanceSchemaVersion',
  ]) {
    if (key in stripped) {
      const { [key]: _removed, ...rest } = stripped;
      stripped = rest;
    }
  }
  if (!('bootstrap' in stripped)) return stripped;
  const { stageSeven, ...bootstrap } = stripped.bootstrap;
  assert.ok(
    stageSeven === null || typeof stageSeven === 'object',
    'stageSeven is absent, null, or a receipt',
  );
  return { ...stripped, bootstrap };
}

test('the frozen oracle and the live decoder reject the same payloads', () => {
  assert.ok(REJECTIONS.length > 0);
  for (const { what, decoder, bytes } of REJECTIONS) {
    const input = Uint8Array.from(bytes);
    assert.throws(() => frozen[decoder](input), undefined, `frozen decoder must reject ${what}`);
    assert.throws(() => live[decoder](input), undefined, `live decoder must reject ${what}`);
  }
});

test('the valid vectors are not vacuous', () => {
  // A decoder that returned an empty object for everything would pass the
  // agreement test above. These pin enough real structure that it cannot.
  const summary = frozen.decodeRuntimeSummary(
    Uint8Array.from([...baseSummary(), ...bootstrapGroup(6)]),
  );
  assert.equal(summary.simulationTicks, 6n);
  assert.equal(summary.digestSchemaVersion, 7);
  assert.equal(summary.bootstrap.stageCount, 6);
  assert.equal(summary.bootstrap.receipts.length, 6);
  assert.equal(summary.bootstrap.complete, true);
  assert.equal(summary.thermalTotalCellEnergy, -17n);
  assert.equal(summary.thermalTotalReservoirBudget, 4096n);

  const chunks = frozen.decodeWorldChunkSnapshot(Uint8Array.from(worldChunkSnapshot()));
  assert.equal(chunks.chunks.length, 3);
  assert.equal(chunks.chunks[0].chunkX, -1);
  assert.equal(chunks.materialSurfaceDeltaSchemaVersion, 4);

  const raster = frozen.decodeFieldRaster(Uint8Array.from(fieldRaster()));
  assert.equal(raster.edge, 2);
  assert.equal(raster.depth, 1);
  assert.deepEqual(Array.from(raster.values), [-13500, -13700, 13100, 19500]);
  assert.equal(raster.generationTraceId, 4242n);
});

// ---------------------------------------------------------------------------
// Stage 6: the appended field-48 bootstrap stage
// ---------------------------------------------------------------------------

/** A runtime summary carrying six stages and, optionally, the appended seventh. */
function summaryWithAppendedStage(appended) {
  const out = [...baseSummary(), ...bootstrapGroup(6)];
  if (appended) {
    out.push(...delimited(48, receiptBody(7, [106])));
  }
  return Uint8Array.from(out);
}

test('the frozen V1 decoder skips field 48 and reads the same six-stage summary', () => {
  // The whole claim behind keeping the protocol at V1: a consumer built before
  // hydrology existed must not notice the appended stage, and must not read a
  // different six-stage summary because of it.
  const without = frozen.decodeRuntimeSummary(summaryWithAppendedStage(false));
  const withAppended = frozen.decodeRuntimeSummary(summaryWithAppendedStage(true));

  assert.equal(without.bootstrap.stageCount, 6);
  assert.equal(without.bootstrap.complete, true);
  assert.equal(without.bootstrap.receipts.length, 6);
  assert.deepEqual(
    withAppended.bootstrap,
    without.bootstrap,
    'field 48 must be invisible to a frozen V1 consumer',
  );
  assert.deepEqual(withAppended, without, 'and invisible to the rest of the summary too');
});

test('the live decoder reads the appended stage while keeping fields 31, 32, and 35', () => {
  const decoded = live.decodeRuntimeSummary(summaryWithAppendedStage(true));
  assert.equal(decoded.bootstrap.stageCount, 6, 'field 31 stays a projected six');
  assert.equal(decoded.bootstrap.complete, true, 'field 32 stays six-stage completion');
  assert.equal(decoded.bootstrap.receipts.length, 6, 'field 35 stays capped at six');
  assert.notEqual(decoded.bootstrap.stageSeven, null);
  assert.equal(decoded.bootstrap.stageSeven.stage, 7n);
  assert.deepEqual(decoded.bootstrap.stageSeven.dependencyTraces, [106n]);
});

test('the live decoder reports no appended stage when the field is absent', () => {
  const decoded = live.decodeRuntimeSummary(summaryWithAppendedStage(false));
  assert.equal(decoded.bootstrap.stageSeven, null);
});

test('two appended stages in one payload are refused by the live decoder', () => {
  // Exactly one appended stage exists, so a repeated field 48 describes two
  // seventh stages.
  const doubled = Uint8Array.from([
    ...baseSummary(),
    ...bootstrapGroup(6),
    ...delimited(48, receiptBody(7, [106])),
    ...delimited(48, receiptBody(7, [106])),
  ]);
  assert.throws(() => live.decodeRuntimeSummary(doubled), /repeats its appended stage/);
});

// ---------------------------------------------------------------------------
// Stage 7: the hydrology fields
// ---------------------------------------------------------------------------

/** Canonical unsigned LEB128 over 128 bits, as fields 37-40 and 46 carry. */
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

function zigzag128Bytes(value) {
  const wide = BigInt(value);
  return varint128(wide < 0n ? (~wide << 1n) | 1n : wide << 1n);
}

/** Fields 36-47 exactly as the current engine writes them. */
function hydrologyGroup() {
  const out = [];
  out.push(...scalar(36, 1));
  // Deliberately past `u64::MAX`: the value a narrowing decoder destroys.
  out.push(...delimited(37, varint128((1n << 64n) + 5n)));
  out.push(...delimited(38, varint128(2000n)));
  out.push(...delimited(39, varint128(3000n)));
  out.push(...delimited(40, varint128(4000n)));
  out.push(...delimited(41, zigzag128Bytes(0)));
  out.push(...scalar(42, 3));
  out.push(...scalar(43, 7));
  out.push(...scalar(44, 11));
  out.push(...scalar(45, 4242));
  out.push(...delimited(46, varint128((1n << 64n) + 9n)));
  out.push(...delimited(47, varint(0xffffffffffffffffn)));
  return out;
}

/** A world snapshot carrying the three hydrology sections at fields 9-14. */
function worldSnapshotWithHydrology() {
  const cellBody = (ordinal) => {
    const bytes = new Uint8Array(22);
    const view = new DataView(bytes.buffer);
    view.setBigUint64(0, 1n, false);
    view.setInt32(8, 0, false);
    view.setInt32(12, 0, false);
    view.setInt32(16, 0, false);
    view.setUint16(20, ordinal, false);
    return Array.from(bytes);
  };
  const delta = [
    ...scalar(1, 1),
    ...scalar(2, zigzag(0)),
    ...scalar(3, zigzag(0)),
    ...scalar(4, zigzag(0)),
    ...scalar(5, 0),
    ...delimited(6, varint(10)),
    ...delimited(7, varint(12)),
    ...delimited(8, varint(20)),
    ...delimited(9, varint(18)),
    ...delimited(10, varint(30)),
    ...delimited(11, varint(30)),
    ...delimited(12, zigzag128Bytes(2)),
    ...delimited(13, zigzag128Bytes(-2)),
    ...scalar(14, 51),
    ...scalar(15, 900),
    ...scalar(16, 6),
  ];
  const transfer = [
    ...scalar(1, 3),
    ...delimited(2, [0x01, ...cellBody(0)]),
    ...delimited(3, [0x01, ...cellBody(0)]),
    ...delimited(4, varint(100)),
    ...delimited(5, varint(80)),
    ...delimited(6, varint(20)),
    ...scalar(7, 51),
    ...scalar(8, 900),
    ...scalar(9, 6),
  ];
  const conveyance = [
    ...delimited(1, [0x02, ...cellBody(0), ...cellBody(1)]),
    ...delimited(2, varint(5)),
    ...delimited(3, varint(50)),
    ...delimited(4, varint(1)),
    ...delimited(5, varint(1)),
    ...scalar(6, 901),
    ...scalar(7, 6),
  ];
  return [
    ...worldChunkSnapshot(),
    ...delimited(9, delta),
    ...scalar(10, 1),
    ...delimited(11, transfer),
    ...scalar(12, 1),
    ...delimited(13, conveyance),
    ...scalar(14, 1),
  ];
}

test('the frozen V1 decoder ignores hydrology fields 36-48 entirely', () => {
  // The compatibility claim the whole plan rests on: protocol V1 stays V1
  // because a client built before hydrology existed reads a new-engine payload
  // and gets exactly what it always got.
  const without = Uint8Array.from([...baseSummary(), ...bootstrapGroup(6)]);
  const withHydrology = Uint8Array.from([
    ...baseSummary(),
    ...bootstrapGroup(6),
    ...hydrologyGroup(),
    ...delimited(48, receiptBody(7, [106])),
  ]);

  const before = frozen.decodeRuntimeSummary(without);
  const after = frozen.decodeRuntimeSummary(withHydrology);

  assert.deepEqual(
    normalize(after),
    normalize(before),
    'a frozen V1 consumer must not see hydrology at all',
  );
  assert.equal(after.bootstrap.stageCount, 6, 'field 31 stays a projected six');
  assert.equal(after.bootstrap.complete, true, 'field 32 stays six-stage completion');
  assert.equal(after.bootstrap.receipts.length, 6, 'field 35 stays capped at six');
});

test('the live decoder reads the hydrology group the frozen one skipped', () => {
  // The other half: the additive fields are not merely tolerated, they carry
  // real values that the frozen decoder had no way to represent.
  const decoded = live.decodeRuntimeSummary(
    Uint8Array.from([...baseSummary(), ...bootstrapGroup(6), ...hydrologyGroup()]),
  );

  assert.equal(decoded.hydrology.schemaVersion, 1);
  assert.equal(decoded.hydrology.totalSurface, (1n << 64n) + 5n);
  assert.equal(decoded.hydrology.latestForcing.acceptedSource, (1n << 64n) + 9n);
  assert.equal(decoded.hydrology.latestForcing.acceptedEvapotranspiration, 0xffffffffffffffffn);
  // And the six-stage projection is untouched beside it.
  assert.equal(decoded.bootstrap.stageCount, 6);
  assert.equal(decoded.bootstrap.receipts.length, 6);
});

test('the frozen V1 decoder ignores world-snapshot fields 9-14', () => {
  const without = Uint8Array.from(worldChunkSnapshot());
  const withHydrology = Uint8Array.from(worldSnapshotWithHydrology());

  assert.deepEqual(
    normalize(frozen.decodeWorldChunkSnapshot(withHydrology)),
    normalize(frozen.decodeWorldChunkSnapshot(without)),
    'a frozen V1 consumer must not see the hydrology sections',
  );

  // The live decoder does see them, so the comparison above is about an
  // invisible addition rather than an absent one.
  const decoded = live.decodeWorldChunkSnapshot(withHydrology);
  assert.equal(decoded.hydrologyDeltas.length, 1);
  assert.equal(decoded.hydrologyTransferSummaries.length, 1);
  assert.equal(decoded.hydrologyConveyanceSummaries.length, 1);
});

test('the frozen V1 decoder refuses a hydrology raster it cannot represent', () => {
  // A water lattice is not something the old decoder can read wrongly and
  // quietly: it has no unsigned band, so the raster's signed band is empty and
  // the declared lattice does not match. Failing closed is the correct legacy
  // behaviour — a client that cannot carry `u64` volumes must not draw them.
  const raster = Uint8Array.from([
    ...scalar(1, 1),
    ...scalar(2, zigzag(0)),
    ...scalar(3, zigzag(0)),
    ...scalar(4, zigzag(0)),
    ...scalar(5, 4),
    ...scalar(7, 2),
    ...scalar(8, 1),
    ...delimited(13, [...varint(1), ...varint(2), ...varint(3), ...varint(4)]),
    ...scalar(14, 1),
  ]);

  assert.throws(() => frozen.decodeFieldRaster(raster));
  const decoded = live.decodeFieldRaster(raster);
  assert.deepEqual(Array.from(decoded.unsignedValues), [1n, 2n, 3n, 4n]);
});
