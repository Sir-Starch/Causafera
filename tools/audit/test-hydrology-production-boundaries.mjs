#!/usr/bin/env node
// Source audit for the hydrology production boundaries `plans/hydrology.md`
// names: no fixture or demo constructor in a production bootstrap or runtime
// path, no semantic water-body vocabulary in authoritative state, and no use of
// `chunk_extent` as a hydrology metric scale.
//
// A source audit rather than a test, because what it protects is the *absence*
// of something. A test can only observe what a constructor does; only reading the
// source can say that no production path calls one that does not exist yet.

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";

const repo = execFileSync("git", ["rev-parse", "--show-toplevel"], {
  encoding: "utf8",
}).trim();

/** Files that carry authoritative hydrology behaviour. */
const PRODUCTION = [
  "crates/causafera-geography/src/hydrology/mod.rs",
  "crates/causafera-geography/src/hydrology/metric.rs",
  "crates/causafera-geography/src/hydrology/substrate.rs",
  "crates/causafera-geography/src/hydrology/forcing.rs",
  "crates/causafera-geography/src/hydrology/state.rs",
  "crates/causafera-domains/src/hydrology/mod.rs",
  "crates/causafera-domains/src/hydrology/parameters.rs",
  "crates/causafera-domains/src/hydrology/evolution.rs",
  "crates/causafera-domains/src/hydrology/proposal.rs",
  "crates/causafera-domains/src/hydrology/records.rs",
  "crates/causafera-domains/src/hydrology/receipts.rs",
  "crates/causafera-domains/src/hydrology/resolution.rs",
  "crates/causafera-runtime/src/hydrology.rs",
  "crates/causafera-runtime/src/hydrology_config.rs",
  "crates/causafera-runtime/src/hydrology_events.rs",
  "crates/causafera-runtime/src/hydrology_validation.rs",
  "crates/causafera-runtime/src/tick_transaction.rs",
];

/** Read a production file, stripped of its `#[cfg(test)]` module and comments. */
function productionSource(relative) {
  const text = readFileSync(`${repo}/${relative}`, "utf8");
  const testModule = text.indexOf("#[cfg(test)]");
  const body = testModule === -1 ? text : text.slice(0, testModule);
  return body
    .split("\n")
    .filter((line) => {
      const trimmed = line.trim();
      return !trimmed.startsWith("//") && !trimmed.startsWith("///");
    })
    .join("\n");
}

test("no hydrology production path constructs a fixture or a demo", () => {
  // `plans/hydrology.md`: "Fixture and demo constructors must not appear in
  // production bootstrap or runtime sessions. Production state requires causal
  // initialization." The audit looks for the constructor *names*, because a
  // production path that called one would have to name it.
  const forbidden = [
    "fixture",
    "demo",
    "sample_world",
    "example_basin",
    "test_world",
    "for_testing",
  ];
  for (const relative of PRODUCTION) {
    const source = productionSource(relative).toLowerCase();
    for (const needle of forbidden) {
      assert.ok(
        !source.includes(needle),
        `${relative} names "${needle}" outside its test module`,
      );
    }
  }
});

test("no hydrology carrier is a semantic water body", () => {
  // The plan is explicit: "No authoritative carrier is a semantic `River`,
  // `Lake`, `Wetland`, `Flood`, `Season`, or `Watershed` enum." Those are
  // classifications an observer may compute; they are never simulation meaning.
  const forbidden = [
    "River",
    "Lake",
    "Wetland",
    "Flood",
    "Watershed",
    "Season",
    "Aquifer{",
    "SoilClass",
    "Biome",
  ];
  for (const relative of PRODUCTION) {
    const source = productionSource(relative);
    for (const needle of forbidden) {
      assert.ok(
        !source.includes(needle),
        `${relative} names the semantic type "${needle}"`,
      );
    }
  }
});

test("hydrology never reads a surface material identity", () => {
  // The plan is explicit: "Surface material identity has no hydraulic meaning in
  // this tranche and is not mapped to permeability or a soil class." A domain
  // counterfactual proves two worlds differing only in material behave
  // identically; only the source can prove no production path reads it at all,
  // including the bootstrap derivation the solver never sees.
  for (const relative of PRODUCTION) {
    const source = productionSource(relative);
    for (const needle of ["surface_material", "MaterialId"]) {
      assert.ok(
        !source.includes(needle),
        `${relative} reads ${needle}, which has no hydraulic meaning`,
      );
    }
  }
});

test("hydrology never reads chunk_extent as a metric scale", () => {
  // `chunk_extent` sizes the mana volume. Using it as a hydrology length would
  // make a chunk a metric unit, which it is not (INV-036, INV-037, INV-043).
  for (const relative of PRODUCTION) {
    const source = productionSource(relative);
    assert.ok(
      !source.includes("chunk_extent"),
      `${relative} reads chunk_extent, which is not a hydrology metric`,
    );
  }
});

test("hydrology process identities are numbers, never language strings", () => {
  // A receipt that said "infiltration" would be a developer analytical label
  // sitting inside authoritative state.
  const parameters = productionSource(
    "crates/causafera-domains/src/hydrology/parameters.rs",
  );
  const stringLiterals = parameters.match(/"[^"]*"/g) ?? [];
  assert.deepEqual(
    stringLiterals,
    [],
    "hydrology process and substage identities carry no string literals",
  );
});

test("the disabled hydrology configuration is the runtime default", () => {
  // No field, edge, boundary, or rainfall is defaulted into an existing session.
  const config = productionSource("crates/causafera-runtime/src/config.rs");
  assert.ok(
    config.includes("hydrology: HydrologyConfig::disabled()"),
    "RuntimeConfig::new must default hydrology to disabled",
  );
});
