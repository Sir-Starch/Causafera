#!/usr/bin/env node
// Adversarial coverage for the TypeScript observer decoder.
//
// The Rust and TypeScript decoders are one wire contract, and four consecutive
// audits of `plans/production-bootstrap-receipt-closure.md` each found a case
// where one accepted what the other refused. The Rust side has an adversarial
// suite in `crates/causafera-observer-wire/tests/protocol.rs`; this is its
// counterpart, so a divergence is caught by a test rather than by the next
// reader of both files.
//
// The package ships TypeScript sources with no build output, so the module is
// compiled here with the workspace's own `tsc` before being imported. That is
// why this lives under `tools/audit` — where a Node test runner already exists
// and is wired into `run-source-tests.mjs` — rather than as a package test.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
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

function compileProtocolModule() {
  const outDir = mkdtempSync(path.join(tmpdir(), 'causafera-observer-protocol-'));
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

const { outDir, entry } = compileProtocolModule();
const protocol = await import(entry);
process.on('exit', () => rmSync(outDir, { recursive: true, force: true }));

// ---------------------------------------------------------------------------
// Minimal encoder, so the vectors below are built from bytes rather than from
// the decoder's own inverse. An encoder shared with the decoder could agree
// with it about a payload both get wrong.
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

/** A runtime summary carrying every field the decoder requires. */
function baseSummary() {
  const out = [];
  out.push(...scalar(1, 3));
  out.push(...scalar(2, 7));
  out.push(...delimited(3, new Array(32).fill(1)));
  out.push(...delimited(4, new Array(32).fill(2)));
  for (const field of [5, 6, 7, 8, 9, 10, 11, 12]) out.push(...scalar(field, 1));
  for (let field = 13; field <= 23; field += 1) out.push(...scalar(field, 1));
  out.push(...scalar(26, 9));
  out.push(...scalar(27, 243));
  return out;
}

function receiptBody(stage, dependencies = []) {
  const out = [];
  out.push(...scalar(1, stage));
  out.push(...scalar(2, stage));
  out.push(...delimited(3, new Array(32).fill(stage)));
  out.push(...scalar(4, 100 + stage));
  for (const dependency of dependencies) out.push(...scalar(5, dependency));
  return out;
}

/** A complete, valid bootstrap summary group over `stages` receipts. */
function bootstrapGroup(stages) {
  const out = [];
  out.push(...scalar(28, 1));
  out.push(...scalar(29, 0x0123456789abcdefn));
  out.push(...scalar(30, 4242));
  out.push(...scalar(31, stages));
  out.push(...scalar(32, stages > 0 ? 1 : 0));
  out.push(...scalar(33, 512));
  out.push(...scalar(34, 8));
  for (let stage = 1; stage <= stages; stage += 1) {
    out.push(...delimited(35, receiptBody(stage, stage === 1 ? [] : [100 + stage - 1])));
  }
  return out;
}

const decode = (bytes) => protocol.decodeRuntimeSummary(Uint8Array.from(bytes));
const rejects = (bytes, what) =>
  assert.throws(() => decode(bytes), undefined, `${what} must be rejected`);

// ---------------------------------------------------------------------------

test('a valid six-receipt summary decodes, and is the control for every rejection below', () => {
  const summary = decode([...baseSummary(), ...bootstrapGroup(6)]);
  assert.equal(summary.bootstrap.schemaVersion, 1);
  assert.equal(summary.bootstrap.complete, true);
  assert.equal(summary.bootstrap.stageCount, 6);
  assert.equal(summary.bootstrap.receipts.length, 6);
  assert.equal(summary.bootstrap.configuredPopulation, 512n);
  assert.equal(summary.bootstrap.configuredPromotionLimit, 8);
  assert.equal(summary.bootstrap.receipts[1].dependencyTraces[0], 101n);
});

test('a payload written before the summary existed decodes as absent, not as empty', () => {
  const summary = decode(baseSummary());
  assert.equal(summary.bootstrap.schemaVersion, 0);
  assert.equal(summary.bootstrap.complete, false);
  assert.equal(summary.bootstrap.stageCount, 0);
  assert.deepEqual(summary.bootstrap.receipts, []);
});

test('a partially present summary group is rejected', () => {
  for (const omitted of [29, 30, 31, 32, 33, 34]) {
    const bytes = [...baseSummary(), ...scalar(28, 1)];
    for (const field of [29, 30, 31, 32, 33, 34]) {
      if (field !== omitted) bytes.push(...scalar(field, 1));
    }
    rejects(bytes, `a summary missing field ${omitted}`);
  }
});

test('receipts with no summary to interpret them are rejected', () => {
  rejects(
    [...baseSummary(), ...delimited(35, receiptBody(1))],
    'a receipt with no summary',
  );
});

test('an unknown summary schema version is rejected', () => {
  for (const version of [0, 2, 99]) {
    rejects(
      [...baseSummary(), ...bootstrapGroup(1).slice(scalar(28, 1).length), ...scalar(28, version)],
      `schema version ${version}`,
    );
  }
});

test('a summary that contradicts its own receipts is rejected', () => {
  const withStageCount = (stages, declared) => [
    ...baseSummary(),
    ...scalar(28, 1),
    ...scalar(29, 1),
    ...scalar(30, 1),
    ...scalar(31, declared),
    ...scalar(32, declared > 0 ? 1 : 0),
    ...scalar(33, 1),
    ...scalar(34, 1),
    ...Array.from({ length: stages }, (_, index) =>
      delimited(35, receiptBody(index + 1, index === 0 ? [] : [100 + index])),
    ).flat(),
  ];
  rejects(withStageCount(2, 6), 'a stage count the receipts do not match');
  rejects(withStageCount(2, 7), 'a stage count past the six-stage envelope');

  const completenessDisagrees = [
    ...baseSummary(),
    ...scalar(28, 1),
    ...scalar(29, 1),
    ...scalar(30, 1),
    ...scalar(31, 2),
    ...scalar(32, 0),
    ...scalar(33, 1),
    ...scalar(34, 1),
    ...delimited(35, receiptBody(1)),
    ...delimited(35, receiptBody(2, [101])),
  ];
  rejects(completenessDisagrees, 'completeness disagreeing with the stage count');

  const invalidBoolean = [
    ...baseSummary(),
    ...scalar(28, 1),
    ...scalar(29, 1),
    ...scalar(30, 1),
    ...scalar(31, 0),
    ...scalar(32, 2),
    ...scalar(33, 1),
    ...scalar(34, 1),
  ];
  rejects(invalidBoolean, 'a completeness flag that is neither zero nor one');
});

test('more receipts than the current bootstrap can produce are rejected', () => {
  const bytes = [...baseSummary(), ...bootstrapGroup(6)];
  bytes.push(...delimited(35, receiptBody(7, [106])));
  rejects(bytes, 'a seventh receipt');
});

test('an out-of-order receipt list is rejected', () => {
  const bytes = [
    ...baseSummary(),
    ...scalar(28, 1),
    ...scalar(29, 1),
    ...scalar(30, 1),
    ...scalar(31, 2),
    ...scalar(32, 1),
    ...scalar(33, 1),
    ...scalar(34, 1),
    ...delimited(35, receiptBody(2)),
    ...delimited(35, receiptBody(1)),
  ];
  rejects(bytes, 'descending receipt stages');
});

test('a duplicated summary scalar is rejected', () => {
  for (let field = 28; field <= 34; field += 1) {
    rejects(
      [...baseSummary(), ...bootstrapGroup(1), ...scalar(field, 1)],
      `a duplicated field ${field}`,
    );
  }
});

test('a mistyped bootstrap field is rejected rather than read as absent', () => {
  for (let field = 28; field <= 34; field += 1) {
    const bytes = [...baseSummary()];
    for (let other = 28; other <= 34; other += 1) {
      if (other === field) bytes.push(...delimited(other, [1]));
      else bytes.push(...scalar(other, 1));
    }
    rejects(bytes, `field ${field} on the wrong wire type`);
  }
  // Field 35 on a varint wire, which must not be stored as a scalar and dropped.
  rejects(
    [...baseSummary(), ...bootstrapGroup(1), ...scalar(35, 1)],
    'field 35 on a varint wire',
  );
});

test('a receipt result that is not thirty-two bytes is rejected', () => {
  const short = [
    ...scalar(1, 1),
    ...scalar(2, 1),
    ...delimited(3, new Array(31).fill(1)),
    ...scalar(4, 101),
  ];
  const bytes = [
    ...baseSummary(),
    ...scalar(28, 1),
    ...scalar(29, 1),
    ...scalar(30, 1),
    ...scalar(31, 1),
    ...scalar(32, 1),
    ...scalar(33, 1),
    ...scalar(34, 1),
    ...delimited(35, short),
  ];
  rejects(bytes, 'a thirty-one byte fingerprint');
});

test('a contradictory nested receipt is rejected', () => {
  const wrap = (body) => [
    ...baseSummary(),
    ...scalar(28, 1),
    ...scalar(29, 1),
    ...scalar(30, 1),
    ...scalar(31, 1),
    ...scalar(32, 1),
    ...scalar(33, 1),
    ...scalar(34, 1),
    ...delimited(35, body),
  ];
  // Control: the wrapper itself is valid.
  assert.equal(decode(wrap(receiptBody(1))).bootstrap.receipts.length, 1);

  for (const field of [1, 2, 4]) {
    rejects(wrap([...receiptBody(1), ...scalar(field, 9)]), `a duplicated receipt field ${field}`);
  }
  rejects(
    wrap([...receiptBody(1), ...delimited(3, new Array(32).fill(9))]),
    'a duplicated receipt result',
  );
  rejects(wrap([...receiptBody(1), ...delimited(1, [1])]), 'receipt field 1 as bytes');
  rejects(wrap([...receiptBody(1), ...delimited(5, [1])]), 'receipt field 5 as bytes');
  rejects(
    wrap([...receiptBody(1, [3, 2])]),
    'descending dependency traces',
  );
  rejects(
    wrap([...receiptBody(1, [1, 2, 3, 4, 5, 6, 7, 8, 9])]),
    'nine dependency traces',
  );
});

test('numeric bounds match the Rust decoder', () => {
  // Every field the Rust decoder narrows with `to_u32` must reject past 2^32-1
  // here too, or one decoder accepts what the other refuses. These fields are
  // written twice in the payload, and the decoder takes the later value; the
  // control below proves an in-range second value still decodes, so the
  // rejection is the bound and not the repetition.
  assert.equal(decode([...baseSummary(), ...scalar(2, 7)]).digestSchemaVersion, 7);
  for (const field of [2, 7, 9, 11, 26, 27]) {
    rejects(
      [...baseSummary(), ...scalar(field, 0x1_0000_0000n)],
      `field ${field} past its 32-bit range`,
    );
  }
  rejects(
    [
      ...baseSummary(),
      ...scalar(28, 1),
      ...scalar(29, 1),
      ...scalar(30, 1),
      ...scalar(31, 0),
      ...scalar(32, 0),
      ...scalar(33, 1),
      ...scalar(34, 0x1_0000_0000n),
    ],
    'a promotion limit past its 32-bit range',
  );
});

test('an over-wide varint is rejected, as the Rust cursor rejects it', () => {
  // Ten continuation bytes with a tenth payload byte above one describes a
  // value wider than 64 bits. Rust truncates it if unchecked; a bigint cannot,
  // so without the bound the two disagree on validity.
  const overWide = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02];

  // The varint has to sit inside an otherwise COMPLETE group. Appending field 29
  // to a summary carrying no group is rejected by the partial-group check
  // whatever the bound does, which is how an earlier revision of this test
  // passed with the bound removed — it asserted a rejection it would have got
  // for the wrong reason.
  const group = [
    ...scalar(28, 1),
    ...varint(29n << 3n),
    ...overWide,
    ...scalar(30, 4242),
    ...scalar(31, 0),
    ...scalar(32, 0),
    ...scalar(33, 512),
    ...scalar(34, 8),
  ];
  rejects([...baseSummary(), ...group], 'an over-wide varint inside a complete group');

  // Control: the same group with a representable field 29 decodes, so the
  // rejection above is the bound and not the group's shape.
  const control = decode([...baseSummary(), ...bootstrapGroup(0)]);
  assert.equal(control.bootstrap.schemaVersion, 1);
  assert.equal(control.bootstrap.planId, 0x0123456789abcdefn);

  // And the VALUE, not merely the absence of a rejection. Rejection alone
  // cannot tell a correct bound from one applied at the wrong shift or one that
  // drops bit 63 — both read the varint, assemble a different number, and are
  // still rejected on the vector above. The Rust test pins these same two
  // values; without this the TypeScript decoder could silently misplace bit 63
  // and diverge with the whole suite green.
  const representable = (tenth) => {
    const group = [
      ...scalar(28, 1),
      ...varint(29n << 3n),
      ...[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, tenth],
      ...scalar(30, 4242),
      ...scalar(31, 0),
      ...scalar(32, 0),
      ...scalar(33, 512),
      ...scalar(34, 8),
    ];
    return decode([...baseSummary(), ...group]).bootstrap.planId;
  };
  // Nine payload bytes of ones fill bits 0 to 62; the tenth byte carries bit 63.
  assert.equal(representable(0x00), (1n << 63n) - 1n);
  assert.equal(representable(0x01), (1n << 64n) - 1n);
});

test('an unsupported wire type is rejected, as the Rust cursor rejects it', () => {
  // Wire type 5 (fixed32) is legal protobuf that no encoder here emits and the
  // Rust cursor refuses; accepting it would walk past a field Rust rejects the
  // whole message over.
  rejects(
    [...baseSummary(), ...varint((200n << 3n) | 5n), 1, 2, 3, 4],
    'an unknown field on wire type 5',
  );
});

test('an unknown field number past its 32-bit range is rejected', () => {
  rejects(
    [...baseSummary(), ...varint((0x1_0000_0000n << 3n) | 0n), 1],
    'a field number past 2^32',
  );
});

test('an unknown field on a supported wire type is skipped, not rejected', () => {
  const summary = decode([...baseSummary(), ...bootstrapGroup(1), ...scalar(200, 1)]);
  assert.equal(summary.bootstrap.stageCount, 1);
});

// ---------------------------------------------------------------------------
// Field raster. `cell_count` became a checked `Option` on the Rust side during
// this plan, so the lattice bound is a surface this branch changed; the Node
// suite covered none of it, and neither did the Rust suite until now.
// ---------------------------------------------------------------------------

const zigzag = (value) => (value < 0n ? -value * 2n - 1n : value * 2n);

/** Delta-encoded value band, built from bytes rather than from the decoder. */
function deltaBand(values) {
  const out = [];
  let previous = 0n;
  for (const value of values) {
    out.push(...varint(zigzag(BigInt(value) - previous)));
    previous = BigInt(value);
  }
  return out;
}

function raster({ edge, depth, values = [], auxiliary, cellTraces }) {
  const out = [];
  out.push(...scalar(1, 3));
  out.push(...scalar(2, 0));
  out.push(...scalar(3, 0));
  out.push(...scalar(4, 0));
  out.push(...scalar(5, 1));
  out.push(...scalar(6, 0));
  out.push(...scalar(7, edge));
  out.push(...scalar(8, depth));
  out.push(...delimited(9, deltaBand(values)));
  if (auxiliary !== undefined) out.push(...delimited(10, deltaBand(auxiliary)));
  if (cellTraces !== undefined) {
    const packed = [];
    for (const trace of cellTraces) packed.push(...varint(BigInt(trace)));
    out.push(...delimited(11, packed));
  }
  return out;
}

const decodeRaster = (bytes) => protocol.decodeFieldRaster(Uint8Array.from(bytes));
const rasterRejects = (bytes, what) =>
  assert.throws(() => decodeRaster(bytes), undefined, `${what} must be rejected`);

test('a field raster whose payload fills its lattice decodes, and is the control', () => {
  const decoded = decodeRaster(raster({ edge: 3, depth: 1, values: [0, 5, -5, 7, 0, 1, 2, 3, 4] }));
  assert.equal(decoded.edge, 3);
  assert.equal(decoded.depth, 1);
  assert.equal(decoded.values.length, 9);
  assert.equal(decoded.values[2], -5);
});

test('a field raster is refused when its payload does not fill the declared lattice', () => {
  rasterRejects(raster({ edge: 4, depth: 1, values: [0, 1, 2, 3, 4, 5, 6, 7, 8] }), 'nine cells in a sixteen-cell lattice');
  rasterRejects(
    raster({ edge: 2, depth: 1, values: [0, 1, 2, 3], auxiliary: [0, 1] }),
    'an auxiliary band shorter than the lattice',
  );
  rasterRejects(
    raster({ edge: 2, depth: 1, values: [0, 1, 2, 3], cellTraces: [1, 2, 3] }),
    'a trace band shorter than the lattice',
  );
});

test('a declared lattice larger than a representable cell count is refused', () => {
  // Rust computed this as an unchecked multiply before `cell_count` became an
  // `Option`: `edge² × depth` of exactly 2^64 wrapped to zero and a release
  // build accepted an empty value band. TypeScript accumulates into a float and
  // never wrapped, so this is the pair of inputs on which the two decoders used
  // to disagree outright.
  rasterRejects(raster({ edge: 1 << 24, depth: 1 << 16 }), 'a lattice of exactly 2^64 cells');
  rasterRejects(raster({ edge: 1 << 26, depth: 1 << 12 }), 'a lattice that overflows 64 bits');
  rasterRejects(raster({ edge: 0xffffffff, depth: 0xffffffff }), 'a lattice at the 32-bit maxima');
});

test('a field raster missing a required field is rejected rather than defaulted', () => {
  const complete = raster({ edge: 2, depth: 1, values: [0, 1, 2, 3] });
  for (const field of [1, 5, 7, 8]) {
    rasterRejects(stripRasterField(complete, field), `a raster with no field ${field}`);
  }
});

/** Drop one field from a raster payload, leaving the rest byte-identical. */
function stripRasterField(bytes, target) {
  const out = [];
  let at = 0;
  while (at < bytes.length) {
    const start = at;
    let key = 0n;
    let shift = 0n;
    while (bytes[at] & 0x80) key |= BigInt(bytes[at++] & 0x7f) << shift, (shift += 7n);
    key |= BigInt(bytes[at++]) << shift;
    const field = Number(key >> 3n);
    const wire = Number(key & 7n);
    if (wire === 0) {
      while (bytes[at] & 0x80) at += 1;
      at += 1;
    } else if (wire === 2) {
      let length = 0n;
      let lengthShift = 0n;
      while (bytes[at] & 0x80) length |= BigInt(bytes[at++] & 0x7f) << lengthShift, (lengthShift += 7n);
      length |= BigInt(bytes[at++]) << lengthShift;
      at += Number(length);
    } else {
      throw new Error(`unexpected wire type ${wire}`);
    }
    if (field !== target) out.push(...bytes.slice(start, at));
  }
  return out;
}
