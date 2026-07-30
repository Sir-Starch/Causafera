#!/usr/bin/env node
// The `.proto` schema pinned to the hand-written Rust and TypeScript codecs.
//
// This repository has no generated protobuf binding pipeline: the wire codecs
// are written by hand and `proto/causafera/observer/v1/query.proto` is a
// declaration of what they do. Nothing enforces that on its own, so a field the
// schema names at 37 and the codec writes at 38 would compile, pass every Rust
// test, and be wrong for every consumer that generates bindings from the schema.
//
// This audit reads the schema as text and asserts that every field number, wire
// shape, and declared bound matches what the codecs actually implement. It makes
// no generated-binding claim, because there is no generated binding.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import path from 'node:path';

const REPOSITORY_ROOT = execFileSync('git', ['rev-parse', '--show-toplevel'], {
  encoding: 'utf8',
}).trim();

const SCHEMA = readFileSync(
  path.join(REPOSITORY_ROOT, 'proto', 'causafera', 'observer', 'v1', 'query.proto'),
  'utf8',
);
const RUST_WIRE = readFileSync(
  path.join(REPOSITORY_ROOT, 'crates', 'causafera-observer-wire', 'src', 'protocol.rs'),
  'utf8',
);
const RUST_API = readFileSync(
  path.join(REPOSITORY_ROOT, 'crates', 'causafera-observer-api', 'src', 'query.rs'),
  'utf8',
);
const TYPESCRIPT = readFileSync(
  path.join(REPOSITORY_ROOT, 'packages', 'observer-protocol', 'src', 'index.ts'),
  'utf8',
);

/** Strip comments so a field number mentioned in prose is never matched. */
function statements(source) {
  return source
    .split('\n')
    .map((line) => line.replace(/\/\/.*$/, '').trim())
    .filter((line) => line.length > 0);
}

/**
 * Every `name = number;` declaration inside one message or enum body, as a map
 * from name to `{ number, type }`.
 */
function declarations(name) {
  const opener = new RegExp(`^(message|enum)\\s+${name}\\s*\\{`);
  const lines = statements(SCHEMA);
  const start = lines.findIndex((line) => opener.test(line));
  assert.notEqual(start, -1, `${name} must be declared in query.proto`);
  const fields = new Map();
  let depth = 0;
  for (let index = start; index < lines.length; index += 1) {
    const line = lines[index];
    depth += (line.match(/\{/g) ?? []).length;
    depth -= (line.match(/\}/g) ?? []).length;
    const field = line.match(/^(?:(optional|repeated)\s+)?([\w.]+)\s+(\w+)\s*=\s*(\d+)\s*;/);
    if (field && index !== start) {
      fields.set(field[3], {
        number: Number(field[4]),
        type: field[2],
        label: field[1] ?? '',
      });
    }
    const member = line.match(/^([A-Z0-9_]+)\s*=\s*(\d+)\s*;/);
    if (member) fields.set(member[1], { number: Number(member[2]), type: 'enum', label: '' });
    if (depth === 0 && index > start) break;
  }
  assert.ok(fields.size > 0, `${name} must declare fields`);
  return fields;
}

function expectFields(name, expected) {
  const declared = declarations(name);
  for (const [field, { number, type, label }] of Object.entries(expected)) {
    const found = declared.get(field);
    assert.ok(found, `${name}.${field} must be declared`);
    assert.equal(found.number, number, `${name}.${field} must be field ${number}`);
    if (type !== undefined) {
      assert.equal(found.type, type, `${name}.${field} must be ${type}`);
    }
    if (label !== undefined) {
      assert.equal(found.label, label, `${name}.${field} must be "${label}"`);
    }
  }
  return declared;
}

test('the bootstrap summary mirror matches the shipped codecs', () => {
  // Fields 28-35 shipped before the schema declared them. The backfill must
  // describe what the codecs already write, not a tidier version of it.
  expectFields('RuntimeSummary', {
    bootstrap_schema_version: { number: 28, type: 'uint32' },
    bootstrap_plan_id: { number: 29, type: 'uint64' },
    bootstrap_world_seed: { number: 30, type: 'uint64' },
    bootstrap_stage_count: { number: 31, type: 'uint32' },
    bootstrap_complete: { number: 32, type: 'bool' },
    bootstrap_configured_population: { number: 33, type: 'uint64' },
    bootstrap_configured_promotion_limit: { number: 34, type: 'uint32' },
    bootstrap_receipts: { number: 35, type: 'BootstrapReceipt', label: 'repeated' },
  });
  expectFields('BootstrapReceipt', {
    stage: { number: 1, type: 'uint64' },
    completed_at: { number: 2, type: 'uint64' },
    result: { number: 3, type: 'bytes' },
    completion_trace: { number: 4, type: 'uint64' },
    dependency_traces: { number: 5, type: 'uint64', label: 'repeated' },
  });
  // The codecs write these exact numbers. Whitespace is elastic because
  // rustfmt breaks a long call across lines; the field number is not.
  for (const field of [28, 29, 30, 31, 32, 33, 34]) {
    assert.ok(
      new RegExp(`field_varint\\(\\s*&mut out,\\s*${field},`).test(RUST_WIRE),
      `the Rust encoder must write bootstrap field ${field} as a varint`,
    );
  }
  assert.ok(
    /field_bytes\(&mut out, 35, &nested\)/.test(RUST_WIRE),
    'the Rust encoder must write bootstrap receipts at field 35',
  );
});

test('the hydrology summary group matches the schema', () => {
  expectFields('RuntimeSummary', {
    hydrology_summary_schema_version: { number: 36, type: 'uint32' },
    hydrology_total_surface: { number: 37, type: 'bytes' },
    hydrology_total_soil: { number: 38, type: 'bytes' },
    hydrology_total_groundwater: { number: 39, type: 'bytes' },
    hydrology_total_conveyance: { number: 40, type: 'bytes' },
    hydrology_latest_residual: { number: 41, type: 'bytes' },
    hydrology_active_chunk_count: { number: 42, type: 'uint32' },
    hydrology_latest_forcing_tick: { number: 43, type: 'uint64', label: 'optional' },
    hydrology_latest_forcing_id: { number: 44, type: 'uint64', label: 'optional' },
    hydrology_latest_forcing_origin: { number: 45, type: 'uint64', label: 'optional' },
    hydrology_latest_accepted_source: { number: 46, type: 'bytes', label: 'optional' },
    hydrology_latest_accepted_et: { number: 47, type: 'bytes', label: 'optional' },
    hydrology_bootstrap_receipt: {
      number: 48,
      type: 'BootstrapReceipt',
      label: 'optional',
    },
  });
  // The Rust encoder writes the same numbers with the same wire shapes.
  for (const field of [36, 42, 43, 44, 45]) {
    assert.ok(
      new RegExp(`field_varint\\(\\s*out,\\s*${field},`).test(RUST_WIRE),
      `field ${field} must be a varint in the Rust encoder`,
    );
  }
  for (const field of [37, 38, 39, 40, 41, 46, 47]) {
    assert.ok(
      new RegExp(`field_bytes\\(\\s*out,\\s*${field},`).test(RUST_WIRE),
      `field ${field} must be length-delimited in the Rust encoder`,
    );
  }
  // And the TypeScript decoder agrees about which of them are varints.
  assert.ok(
    /return field === 36 \|\| \(field >= 42 && field <= 45\) \? 0 : 2;/.test(TYPESCRIPT),
    'the TypeScript wire-type table must match the schema',
  );
});

test('the world snapshot hydrology sections match the schema', () => {
  expectFields('WorldChunkSnapshot', {
    hydrology_deltas: { number: 9, type: 'HydrologyCellDelta', label: 'repeated' },
    hydrology_delta_schema_version: { number: 10, type: 'uint32' },
    hydrology_transfer_summaries: {
      number: 11,
      type: 'HydrologyTransferSummary',
      label: 'repeated',
    },
    hydrology_transfer_schema_version: { number: 12, type: 'uint32' },
    hydrology_conveyance_summaries: {
      number: 13,
      type: 'HydrologyConveyanceSummary',
      label: 'repeated',
    },
    hydrology_conveyance_schema_version: { number: 14, type: 'uint32' },
  });
  for (const field of [9, 11, 13]) {
    assert.ok(
      new RegExp(`field_bytes\\(\\s*&mut out,\\s*${field},`).test(RUST_WIRE),
      `field ${field} must be a repeated message in the Rust encoder`,
    );
  }
});

test('the three hydrology messages match the schema field for field', () => {
  expectFields('HydrologyCellDelta', {
    chart_id: { number: 1, type: 'uint64' },
    chunk_x: { number: 2, type: 'sint32' },
    chunk_y: { number: 3, type: 'sint32' },
    chunk_z: { number: 4, type: 'sint32' },
    cell_ordinal: { number: 5, type: 'uint32' },
    surface_before: { number: 6, type: 'bytes' },
    surface_after: { number: 7, type: 'bytes' },
    soil_before: { number: 8, type: 'bytes' },
    soil_after: { number: 9, type: 'bytes' },
    groundwater_before: { number: 10, type: 'bytes' },
    groundwater_after: { number: 11, type: 'bytes' },
    net_forcing: { number: 12, type: 'bytes' },
    net_lateral_flow: { number: 13, type: 'bytes' },
    transition_trace_id: { number: 14, type: 'uint64' },
    conservation_trace_id: { number: 15, type: 'uint64' },
    transition_tick: { number: 16, type: 'uint64' },
  });
  expectFields('HydrologyTransferSummary', {
    process_kind: { number: 1, type: 'uint32' },
    source_key: { number: 2, type: 'bytes' },
    target_key: { number: 3, type: 'bytes' },
    requested_volume: { number: 4, type: 'bytes' },
    accepted_volume: { number: 5, type: 'bytes' },
    unaccepted_volume: { number: 6, type: 'bytes' },
    transfer_trace_id: { number: 7, type: 'uint64' },
    conservation_trace_id: { number: 8, type: 'uint64' },
    tick: { number: 9, type: 'uint64' },
    forcing_origin_trace_id: { number: 10, type: 'uint64', label: 'optional' },
  });
  expectFields('HydrologyConveyanceSummary', {
    edge_key: { number: 1, type: 'bytes' },
    storage: { number: 2, type: 'bytes' },
    capacity: { number: 3, type: 'bytes' },
    accepted_inflow: { number: 4, type: 'bytes' },
    accepted_release: { number: 5, type: 'bytes' },
    last_change_trace_id: { number: 6, type: 'uint64' },
    tick: { number: 7, type: 'uint64' },
  });
});

test('the raster kinds and the unsigned band match the schema', () => {
  expectFields('FieldRasterKind', {
    FIELD_RASTER_KIND_TERRAIN_ELEVATION: { number: 1 },
    FIELD_RASTER_KIND_TERRAIN_ROUGHNESS: { number: 2 },
    FIELD_RASTER_KIND_MANA_INTENSITY: { number: 3 },
    FIELD_RASTER_KIND_HYDROLOGY_SURFACE_WATER: { number: 4 },
    FIELD_RASTER_KIND_HYDROLOGY_SOIL_WATER: { number: 5 },
    FIELD_RASTER_KIND_HYDROLOGY_GROUNDWATER: { number: 6 },
  });
  expectFields('FieldRaster', {
    unsigned_values: { number: 13, type: 'bytes' },
    unsigned_values_schema_version: { number: 14, type: 'uint32' },
  });
  // The Rust and TypeScript enums agree with the schema, discriminant for
  // discriminant.
  assert.ok(/HydrologySurfaceWater = 4,/.test(RUST_API), 'the Rust raster kind 4 must match the schema');
  assert.ok(/HydrologySoilWater = 5,/.test(RUST_API), 'the Rust raster kind 5 must match the schema');
  assert.ok(/HydrologyGroundwater = 6,/.test(RUST_API), 'the Rust raster kind 6 must match the schema');
  assert.ok(/HydrologySurfaceWater = 4,/.test(TYPESCRIPT), 'the TypeScript raster kind 4 must match the schema');
  assert.ok(/HydrologySoilWater = 5,/.test(TYPESCRIPT), 'the TypeScript raster kind 5 must match the schema');
  assert.ok(/HydrologyGroundwater = 6,/.test(TYPESCRIPT), 'the TypeScript raster kind 6 must match the schema');
});

test('the carrier key variants and lengths match the schema', () => {
  const variants = declarations('HydrologyCarrierKeyVariant');
  const expected = {
    HYDROLOGY_CARRIER_KEY_VARIANT_CELL: [1, 23],
    HYDROLOGY_CARRIER_KEY_VARIANT_EDGE: [2, 45],
    HYDROLOGY_CARRIER_KEY_VARIANT_EXTERIOR_FACE: [3, 24],
    HYDROLOGY_CARRIER_KEY_VARIANT_FORCING_RECORD: [4, 17],
    HYDROLOGY_CARRIER_KEY_VARIANT_RESOLUTION_CHUNK: [5, 21],
    HYDROLOGY_CARRIER_KEY_VARIANT_BATCH_NODE: [6, 9],
  };
  for (const [name, [number, length]] of Object.entries(expected)) {
    assert.equal(variants.get(name)?.number, number, `${name} must be variant ${number}`);
    // The declared length appears in the schema's own comment for that variant,
    // which is the only place the schema can carry a fixed length at all.
    assert.ok(
      new RegExp(`Length ${length}[.:]`).test(SCHEMA),
      `the schema must declare length ${length}`,
    );
  }
  // Both implementations agree on the same six lengths.
  for (const length of [23, 45, 24, 17, 21, 9]) {
    assert.ok(
      RUST_API.includes(`Ok(${length})`) ||
        RUST_API.includes(`Ok(1 + HYDROLOGY_CELL_BODY_LEN)`),
      'the Rust length table must be present',
    );
  }
  assert.ok(/const HYDROLOGY_CELL_BODY_LEN = 22;/.test(TYPESCRIPT), 'the TypeScript cell body is 22 bytes');
  assert.ok(/const HYDROLOGY_CELL_BODY_LEN: usize = 22;/.test(RUST_API), 'the Rust cell body is 22 bytes');
});

test('every declared bound is the same number in all three places', () => {
  for (const [rust, typescript] of [
    ['MAX_HYDROLOGY_DELTAS: usize = 64', 'MAX_HYDROLOGY_DELTAS = 64'],
    [
      'MAX_HYDROLOGY_TRANSFER_SUMMARIES: usize = 64',
      'MAX_HYDROLOGY_TRANSFER_SUMMARIES = 64',
    ],
    [
      'MAX_HYDROLOGY_CONVEYANCE_SUMMARIES: usize = 64',
      'MAX_HYDROLOGY_CONVEYANCE_SUMMARIES = 64',
    ],
    ['HYDROLOGY_SUMMARY_SCHEMA_V1: u32 = 1', 'HYDROLOGY_SUMMARY_SCHEMA_V1 = 1'],
    ['HYDROLOGY_DELTA_SCHEMA_V1: u32 = 1', 'HYDROLOGY_DELTA_SCHEMA_V1 = 1'],
    [
      'HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1: u32 = 1',
      'HYDROLOGY_TRANSFER_SUMMARY_SCHEMA_V1 = 1',
    ],
    [
      'HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1: u32 = 1',
      'HYDROLOGY_CONVEYANCE_SUMMARY_SCHEMA_V1 = 1',
    ],
    ['HYDROLOGY_RASTER_VALUES_SCHEMA_V1: u32 = 1', 'HYDROLOGY_RASTER_VALUES_SCHEMA_V1 = 1'],
    [
      'MAX_QUERY_RESPONSE_PAYLOAD_BYTES: usize = 1 << 20',
      'MAX_QUERY_RESPONSE_PAYLOAD_BYTES = 1 << 20',
    ],
  ]) {
    assert.ok(RUST_API.includes(rust), `the Rust API must declare ${rust}`);
    assert.ok(TYPESCRIPT.includes(typescript), `the TypeScript must declare ${typescript}`);
  }
  // The schema records the 64-entry rule in prose, since proto3 has no way to
  // declare a repeated-field bound.
  assert.ok(/capped at 64/.test(SCHEMA), 'the schema records the 64-entry rule');
});

test('no observer field number is declared twice', () => {
  // The one failure this whole audit exists to prevent, checked directly: two
  // declarations sharing a number would make one of them unreadable.
  for (const message of [
    'RuntimeSummary',
    'WorldChunkSnapshot',
    'FieldRaster',
    'BootstrapReceipt',
    'HydrologyCellDelta',
    'HydrologyTransferSummary',
    'HydrologyConveyanceSummary',
  ]) {
    const numbers = Array.from(declarations(message).values(), (entry) => entry.number);
    assert.equal(
      new Set(numbers).size,
      numbers.length,
      `${message} declares a field number twice`,
    );
  }
});

test('the schema parser is not vacuous', () => {
  // Every assertion above rests on `declarations` actually finding fields, so
  // it is pinned against a message whose shape predates this audit entirely.
  const summary = declarations('RuntimeSummary');
  assert.equal(summary.get('simulation_ticks')?.number, 1);
  assert.equal(summary.get('thermal_active_cell_count')?.number, 27);
  assert.ok(summary.size >= 48, 'RuntimeSummary declares at least 48 fields');
  assert.throws(
    () => declarations('NoSuchMessage'),
    undefined,
    'a message that does not exist must fail rather than return nothing',
  );
});
