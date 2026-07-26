#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import ts from "typescript";

const __filename = url.fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "../../");
const localesDir = path.join(projectRoot, "apps/observer/src/locales");

const EXPECTED_LOCALES = ["en", "ru", "zh-Hans", "de", "es"];
const EXEMPT_FROM_DUPLICATE_CHECK = new Set([
  "product",
  "seed",
  "protocol",
  "tick",
  "play",
  "pause",
  "step",
  "reset",
  "chunk",
  "chart",
  "world",
  "mana",
  "traces",
  "physics",
  "evidence",
  "confidence",
  "checkpoint",
  "supported",
  "partial",
  "unsupported",
  "unknown",
  "comparison",
  "max",
  "experimentId",
]);

function loadTsDictionary(filePath) {
  const content = fs.readFileSync(filePath, "utf-8");
  const transpiled = ts.transpileModule(content, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2022,
    },
  }).outputText;

  const mockExports = {};
  const mockRequire = (id) => {
    if (id === "./types") return {};
    throw new Error(`Unexpected import of ${id} in dictionary`);
  };

  const fn = new Function("exports", "require", transpiled);
  fn(mockExports, mockRequire);

  const exportKeys = Object.keys(mockExports);
  if (exportKeys.length === 0) {
    throw new Error(`No exports found in ${path.basename(filePath)}`);
  }
  return mockExports[exportKeys[0]];
}

function runAudit() {
  console.log(`[i18n-audit] Inspecting localization resources in: ${localesDir}`);
  const dictionaries = {};

  for (const loc of EXPECTED_LOCALES) {
    const file = path.join(localesDir, `${loc}.ts`);
    if (!fs.existsSync(file)) {
      console.error(`[ERROR] Missing expected dictionary file: ${file}`);
      process.exit(1);
    }
    try {
      dictionaries[loc] = loadTsDictionary(file);
      console.log(`[i18n-audit] Loaded dictionary: ${loc}`);
    } catch (err) {
      console.error(`[ERROR] Failed to evaluate dictionary ${loc}:`, err);
      process.exit(1);
    }
  }

  const base = dictionaries.en;
  const baseKeys = new Set(Object.keys(base));
  let errors = 0;

  // 1. Verify exact key equality with English fallback
  for (const [loc, dict] of Object.entries(dictionaries)) {
    const locKeys = new Set(Object.keys(dict));

    for (const key of baseKeys) {
      if (!locKeys.has(key)) {
        console.error(`[ERROR] Locale "${loc}" missing expected key: "${key}"`);
        errors++;
      }
    }
    for (const key of locKeys) {
      if (!baseKeys.has(key)) {
        console.error(`[ERROR] Locale "${loc}" contains unexpected key: "${key}"`);
        errors++;
      }
    }

    // 2. Check schemaNames consistency
    if (typeof dict.schemaNames !== "object" || dict.schemaNames === null) {
      console.error(`[ERROR] Locale "${loc}" schemaNames is not a valid object`);
      errors++;
    } else {
      for (let id = 1; id <= 15; id++) {
        const desc = dict.schemaNames[id];
        if (typeof desc !== "string" || desc.trim().length === 0) {
          console.error(`[ERROR] Locale "${loc}" schemaNames[${id}] is invalid or empty`);
          errors++;
        }
      }
    }

    // 3. Check formatter functions with test arguments
    const testCases = [
      { name: "formatActiveChunks", args: [0, 1, 2, 5, 21, 100] },
      { name: "formatTracesCount", args: [0, 1, 2, 5, 10] },
      { name: "formatMatchedCohort", args: ["42"] },
      { name: "formatCounterfactualCohort", args: ["99"] },
      { name: "formatUnknownSchema", args: ["77"] },
      { name: "formatMax", args: ["10000"] },
      { name: "formatSchema", args: ["v2", 1] },
    ];

    for (const { name, args } of testCases) {
      const fn = dict[name];
      if (typeof fn !== "function") {
        console.error(`[ERROR] Locale "${loc}" property "${name}" must be a function`);
        errors++;
        continue;
      }
      for (const arg of args) {
        try {
          const result = fn(arg);
          if (typeof result !== "string" || result.trim().length === 0) {
            console.error(`[ERROR] Locale "${loc}" ${name}(${arg}) returned empty string`);
            errors++;
          }
        } catch (e) {
          console.error(`[ERROR] Locale "${loc}" ${name}(${arg}) threw an exception:`, e.message);
          errors++;
        }
      }
    }

    // 4. Verify that non-English dictionaries don't leave translated descriptive strings identical to English
    if (loc !== "en") {
      const longTextKeys = [
        "objectiveProjection",
        "boundedChart",
        "unavailable",
        "timeline",
        "physicalEvents",
        "activeChunks",
        "resolution",
        "causalActivity",
        "noData",
        "needMoreSamples",
        "selectChunk",
        "elevation",
        "roughness",
        "relevance",
        "latestTrace",
        "physicalDigest",
        "historyDigest",
        "causalLoop",
        "actions",
        "movements",
        "manaEffects",
        "resolutionTransitions",
        "analysisTitle",
        "analysisDescription",
        "runAnalysis",
        "analyzing",
        "tracesCount",
        "noExplanation",
        "projectionNotice",
      ];
      for (const key of longTextKeys) {
        if (dict[key] === base[key] && !EXEMPT_FROM_DUPLICATE_CHECK.has(key)) {
          console.error(`[ERROR] Locale "${loc}" string "${key}" appears untranslated (identical to English fallback: "${dict[key]}")`);
          errors++;
        }
      }
    }
  }

  if (errors > 0) {
    console.error(`\n[FAILED] i18n audit completed with ${errors} error(s).`);
    process.exit(1);
  } else {
    console.log(`\n[SUCCESS] All ${EXPECTED_LOCALES.length} locale resources passed rigorous consistency and formatting verification!`);
    process.exit(0);
  }
}

try {
  runAudit();
} catch (e) {
  console.error("[FATAL] Unhandled exception during audit:", e);
  process.exit(1);
}
