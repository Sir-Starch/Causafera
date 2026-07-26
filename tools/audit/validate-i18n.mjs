#!/usr/bin/env node
/**
 * Localization audit.
 *
 * TypeScript already guarantees that every dictionary has the same keys as the English
 * baseline — `Copy` is derived from `en.ts`, so a missing key is a compile error. This tool
 * checks the things the compiler cannot see:
 *
 * 1. Placeholder parity. `{n}` present in English but dropped in German is a well-typed string
 *    that renders a hole. The compiler is happy; the reader is not.
 * 2. Untranslated leakage. A value byte-identical to English in another dictionary is either a
 *    forgotten translation or a deliberate identity (a product name, a unit symbol). The
 *    deliberate ones are listed by key; anything else fails.
 * 3. Empty or whitespace-only values, which render as an invisible label.
 * 4. Locale-set agreement across the places that enumerate locales independently: the
 *    `ObserverLocale` union, the switcher tables in `src/i18n/index.ts`, the locale-keyed data
 *    tables outside the dictionaries, and the Rust renderer's own tables.
 * 5. Rust template parity. `render.rs` holds five-element tables of format strings; the array
 *    length is checked by rustc, but nothing checks that `{name}` survives into all five.
 *
 * Exits non-zero on the first category that fails, listing every failure in it.
 */

import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import ts from "typescript";

const here = path.dirname(url.fileURLToPath(import.meta.url));
const projectRoot = path.resolve(here, "../../");
const i18nDir = path.join(projectRoot, "apps/observer/src/i18n");

/** The canonical locale set. Every other list in the repository is checked against this one. */
const LOCALE_TAGS = ["en-US", "ru-RU", "zh-Hans", "de-DE", "es-ES"];
const BASELINE = "en-US";

/** Dictionary module per tag, and the export each one provides. */
const DICTIONARY_MODULES = {
  "en-US": { file: "en.ts", binding: "en" },
  "ru-RU": { file: "ru.ts", binding: "ru" },
  "zh-Hans": { file: "zh-Hans.ts", binding: "zhHans" },
  "de-DE": { file: "de.ts", binding: "de" },
  "es-ES": { file: "es.ts", binding: "es" },
};

/**
 * Keys whose value is allowed to match English in another language, with the reason.
 *
 * A key earns a place here only when the string is an identity rather than a word: a product
 * name, a protocol noun, a unit symbol, or a typographic mark. Adding a key here to silence a
 * genuinely missing translation defeats the check.
 */
const IDENTICAL_ALLOWED = {
  product: "the product name is not translated",
  "areas.assay.note": "Explanation IR is a protocol noun",
  "assay.eyebrow": "Explanation IR is a protocol noun",
  "transport.seed": "seed is the API parameter name",
  "common.none": "an em dash is typographic, not linguistic",
  "chart.metres": "SI symbol",
  "chart.millimetres": "SI symbol",
  "chart.north": "compass letter shared by these languages",
  "chart.mana": "Mana is spelled identically in these languages",
  "chart.chunk": "German borrows Chunk unchanged; Russian transliterates and the others translate",
  "chart.chunksPlural": "German borrows Chunk unchanged, and its plural is identical",
  "instrument.detail": "spelled identically in these languages",
  "common.total": "spelled identically in these languages",
  "flux.total": "spelled identically in these languages",
  "assay.experiment": "spelled identically in these languages",
  "meridian.inspector": "spelled identically in these languages",
  "instrument.channel": "spelled identically in these languages",
  "instrument.command": "spelled identically in these languages",
  "flux.rate": "spelled identically in these languages",
  "flux.contact": "spelled identically in these languages",
  "instrument.title": "spelled identically in these languages",
  "instrument.capabilities": "spelled identically in these languages",
  "assay.claims": "spelled identically in these languages",
  "assay.claim": "spelled identically in these languages",
  "assay.confidence": "spelled identically in these languages",
  "assay.value": "spelled identically in these languages",
  "assay.checkpoint": "spelled identically in these languages",
  "assay.schema": "spelled identically in these languages",
  "assay.partial": "spelled identically in these languages",
  "areas.instrument.name": "spelled identically in these languages",
  "marginalia.protocol": "spelled identically in these languages",
  "chart.legend": "spelled identically in these languages",
  "chart.selection": "spelled identically in these languages",
  "chart.resolution": "spelled identically in these languages",
  "chart.population": "spelled identically in these languages",
  "chart.transitions": "spelled identically in these languages",
  "station.population": "spelled identically in these languages",
  "station.actors": "spelled identically in these languages",
  "station.resolutionRelevance": "spelled identically in these languages",
  "palette.actions": "spelled identically in these languages",
  "palette.areas": "spelled identically in these languages",
};

const failures = [];

function fail(category, message) {
  failures.push({ category, message });
}

/* ------------------------------------------------------------ loading -- */

/**
 * Load a dictionary by stripping its types and evaluating it.
 *
 * The dictionaries import nothing at runtime — `Copy` arrives through `import type`, which
 * transpiles away — so an empty `require` is enough, and a real one appearing later should be
 * loud rather than silent.
 */
function loadDictionary(file, binding) {
  const source = fs.readFileSync(path.join(i18nDir, file), "utf-8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText;

  const exports = {};
  const require = (id) => {
    throw new Error(`${file} gained a runtime import of ${id}; the audit cannot evaluate it`);
  };
  new Function("exports", "require", transpiled)(exports, require);

  const dictionary = exports[binding];
  if (dictionary === undefined || typeof dictionary !== "object") {
    throw new Error(`${file} does not export ${binding}`);
  }
  return dictionary;
}

/* ------------------------------------------------------------- walking -- */

/** Flatten to `path -> string`, so comparisons are between leaves rather than shapes. */
function flatten(value, prefix = "", into = new Map()) {
  for (const [key, entry] of Object.entries(value)) {
    const at = prefix === "" ? key : `${prefix}.${key}`;
    if (typeof entry === "string") {
      into.set(at, entry);
    } else if (entry !== null && typeof entry === "object") {
      flatten(entry, at, into);
    } else {
      fail("shape", `${at} is ${typeof entry}; dictionaries hold strings and objects only`);
    }
  }
  return into;
}

function placeholders(text) {
  return new Set(text.match(/\{[a-zA-Z][a-zA-Z0-9_]*\}/g) ?? []);
}

function sameSet(left, right) {
  if (left.size !== right.size) return false;
  for (const entry of left) if (!right.has(entry)) return false;
  return true;
}

/* ------------------------------------------------- dictionary checks -- */

const dictionaries = new Map();
for (const tag of LOCALE_TAGS) {
  const { file, binding } = DICTIONARY_MODULES[tag];
  dictionaries.set(tag, flatten(loadDictionary(file, binding)));
}

const baseline = dictionaries.get(BASELINE);

for (const tag of LOCALE_TAGS) {
  const dictionary = dictionaries.get(tag);

  for (const [key, english] of baseline) {
    const value = dictionary.get(key);

    if (value === undefined) {
      fail("parity", `${tag} is missing ${key}`);
      continue;
    }
    if (value.trim() === "") {
      fail("empty", `${tag} has an empty value at ${key}`);
    }
    if (!sameSet(placeholders(english), placeholders(value))) {
      fail(
        "placeholder",
        `${tag} at ${key} has placeholders ${[...placeholders(value)].join(", ") || "(none)"}, ` +
          `English has ${[...placeholders(english)].join(", ") || "(none)"}`,
      );
    }
    if (tag !== BASELINE && value === english && IDENTICAL_ALLOWED[key] === undefined) {
      fail("untranslated", `${tag} at ${key} is identical to English: ${JSON.stringify(value)}`);
    }
  }

  for (const key of dictionary.keys()) {
    if (!baseline.has(key)) fail("parity", `${tag} has ${key}, which English does not`);
  }
}

/* ------------------------------------------------- locale set agreement -- */

function readSource(relative) {
  return fs.readFileSync(path.join(projectRoot, relative), "utf-8");
}

/** The `ObserverLocale` union is the source of truth for which tags exist. */
const formatSource = readSource("apps/observer/src/observer/format.ts");
const unionMatch = formatSource.match(/export type ObserverLocale =([^;]+);/);
if (unionMatch === null) {
  fail("locale-set", "could not find the ObserverLocale union in observer/format.ts");
} else {
  const declared = [...unionMatch[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  const missing = LOCALE_TAGS.filter((tag) => !declared.includes(tag));
  const extra = declared.filter((tag) => !LOCALE_TAGS.includes(tag));
  if (missing.length > 0) fail("locale-set", `ObserverLocale is missing ${missing.join(", ")}`);
  if (extra.length > 0) {
    fail("locale-set", `ObserverLocale declares ${extra.join(", ")}, which this audit does not know`);
  }
}

/** Each switcher table must name every locale exactly once. */
const indexSource = readSource("apps/observer/src/i18n/index.ts");
for (const table of ["LOCALES", "LOCALE_NAMES", "LOCALE_MARKS", "dictionaries"]) {
  const block = indexSource.match(new RegExp(`${table}[^=]*=\\s*([\\s\\S]*?)\\n\\};?|${table}[^=]*=\\s*\\[([\\s\\S]*?)\\];`));
  if (block === null) {
    fail("locale-set", `could not find ${table} in i18n/index.ts`);
    continue;
  }
  const body = block[1] ?? block[2] ?? "";
  for (const tag of LOCALE_TAGS) {
    if (!body.includes(`"${tag}"`)) fail("locale-set", `${table} does not list ${tag}`);
  }
}

/**
 * Locale-keyed data outside the dictionaries.
 *
 * These files hold `Record<ObserverLocale, string>` literals. TypeScript already requires every
 * key, so this counts tags instead: an imbalance means a record was hand-edited into a shape the
 * compiler accepted for one entry but not consistently across the file.
 */
const LOCALE_KEYED_SOURCES = [
  "apps/observer/src/observer/claims.ts",
  "apps/observer/src/observer/capability.ts",
  "apps/observer/src/map/lens.ts",
  "apps/observer/src/map/lenses.ts",
];

for (const relative of LOCALE_KEYED_SOURCES) {
  const source = readSource(relative);
  const counts = LOCALE_TAGS.map(
    (tag) => [tag, (source.match(new RegExp(`"${tag}":`, "g")) ?? []).length],
  );
  const [, expected] = counts[0];
  for (const [tag, count] of counts) {
    if (count !== expected) {
      fail(
        "locale-keyed",
        `${relative} has ${count} ${tag} entries but ${expected} ${counts[0][0]} entries`,
      );
    }
  }
}

/* ------------------------------------------------------ Rust templates -- */

/**
 * The Rust renderer's five-element format tables.
 *
 * rustc checks the arity; nothing checks that a placeholder present in the English template
 * survives into the other four. A dropped `{name}` would render a claim without its schema.
 */
const renderSource = readSource("crates/causafera-explanation/src/render.rs");
const rustTables = [...renderSource.matchAll(/const ([A-Z_]+): \[&str; 5\] = \[([\s\S]*?)\];/g)];

if (rustTables.length === 0) {
  fail("rust", "found no [&str; 5] template tables in render.rs");
}

for (const [, name, body] of rustTables) {
  const entries = [...body.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((match) => match[1]);
  if (entries.length !== 5) {
    fail("rust", `${name} holds ${entries.length} strings; the locale set has ${LOCALE_TAGS.length}`);
    continue;
  }
  const expected = placeholders(entries[0]);
  entries.forEach((entry, index) => {
    if (entry.trim() === "") fail("rust", `${name}[${index}] is empty`);
    if (!sameSet(expected, placeholders(entry))) {
      fail(
        "rust",
        `${name}[${index}] has placeholders ${[...placeholders(entry)].join(", ") || "(none)"}, ` +
          `entry 0 has ${[...expected].join(", ") || "(none)"}`,
      );
    }
  });
}

/** The Rust locale enum must carry exactly as many variants as the TypeScript union. */
const orderMatch = renderSource.match(/pub const ORDER: \[Self; (\d+)\]/);
if (orderMatch === null) {
  fail("rust", "could not find ObserverLocale::ORDER in render.rs");
} else if (Number(orderMatch[1]) !== LOCALE_TAGS.length) {
  fail(
    "rust",
    `ObserverLocale::ORDER holds ${orderMatch[1]} locales; the observer offers ${LOCALE_TAGS.length}`,
  );
}

/* ------------------------------------------------------------- report -- */

const keyCount = baseline.size;
if (failures.length === 0) {
  console.log(
    `[i18n-audit] ${LOCALE_TAGS.length} locales · ${keyCount} keys each · ` +
      `${LOCALE_KEYED_SOURCES.length} locale-keyed sources · ${rustTables.length} Rust templates`,
  );
  console.log("[i18n-audit] pass");
  process.exit(0);
}

const byCategory = new Map();
for (const { category, message } of failures) {
  if (!byCategory.has(category)) byCategory.set(category, []);
  byCategory.get(category).push(message);
}

for (const [category, messages] of byCategory) {
  console.error(`\n[i18n-audit] ${category} (${messages.length}):`);
  for (const message of messages) console.error(`  - ${message}`);
}
console.error(`\n[i18n-audit] fail: ${failures.length} problem(s)`);
process.exit(1);
