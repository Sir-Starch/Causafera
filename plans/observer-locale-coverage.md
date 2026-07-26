# Observer Locale Coverage ExecPlan

**Status:** Accepted and implemented.

## Goal

Present the whole observer surface in five languages — `en-US`, `ru-RU`, `zh-Hans`, `de-DE`,
`es-ES` — without letting presentation reach authoritative state, and without leaving any of the
three layers that carry human language half-translated (`TODO-UI-006`).

## Context

The redesigned observer shipped with two locales, `ru-RU` and `en-US`, and with the language
switcher as two buttons in the meridian. That was coherent but incomplete in three ways that only
show up once a third language exists:

1. **The dictionary had no baseline.** `Copy` was derived from the Russian dictionary and English
   was checked against it, while the fallback path resolved to English. The type that enforces
   parity and the language that catches failures were different languages.
2. **Human language lived in three layers, not one.** Beyond `src/i18n/`, locale-keyed data sits in
   `src/observer/claims.ts`, `src/observer/capability.ts`, `src/map/lens.ts` and
   `src/map/lenses.ts` as `Record<ObserverLocale, string>`, and the authoritative Explanation text
   lives in `crates/causafera-explanation/src/render.rs` as a two-variant Rust enum. A locale added
   to the dictionary alone would have left the map legend and every rendered claim in the wrong
   language.
3. **Units were hard-coded in one language.** The relief, elevation-range, roughness and contour
   lenses formatted values as `м` and `мм` regardless of locale, so an English session read metres
   in Cyrillic. This was a live defect, not a gap.

There was no persistence and no system-language detection: the session opened in `ru-RU` on every
run for every reader.

A previous attempt (PR #17) added five dictionaries under `apps/observer/src/locales/` against the
pre-redesign UI. Nothing in the redesigned observer imported them, the audit tool validated that
unimported code and reported success, and the documentation described a `LocaleCopy` architecture
the application did not have. That work was removed from history rather than reverted; the intent
survives here, the artefacts do not.

## Relevant invariants

- INV-006 — the simulation has no privileged human UI language. Every string touched by this work
  is an observer classification.
- INV-007 — changing observer locale cannot change a simulation state hash. Covered across the full
  locale set rather than one pair.
- INV-013 — observation never drives simulation. The locale travels to the protocol handler on
  connect and terminates there.

## Approach

**English becomes the baseline.** `i18n/en.ts` holds the baseline dictionary, `i18n/dictionary.ts`
derives `Copy` with `typeof en`, and the other four declare `const x: Copy`. A key added to English
is a compile error in four files until translated, and the language that enforces parity is now the
language the fallback resolves to.

**`ObserverLocale` widens once and the compiler finds the rest.** Widening the union in
`observer/format.ts` turned all 139 `Record<ObserverLocale, string>` literals across the four
locale-keyed data files into compile errors until complete. That is the intended pressure: the
work cannot be declared done while a layer is missing.

**Units come from the dictionary.** `chart.metres` and `chart.millimetres` already existed; the
lens formatters now read them through `copyFor(context.locale)`.

**The Rust renderer widens to five variants.** Format strings move from per-locale `match` arms to
`[&str; 5]` tables ordered by `ObserverLocale::ORDER`, with named placeholders. `parse` resolves by
primary subtag, with the script deciding for Chinese.

**Traditional Chinese resolves to English, not to `zh-Hans`.** There is no traditional dictionary.
Answering `zh-Hant` with simplified text would overstate coverage, which is the same failure mode
as an invented reading.

**The switcher becomes one cell.** Five buttons would crowd the transport controls out of the
meridian. Options name themselves in their own language, and every locale is also a command-palette
entry.

## Verification

- `cargo test --workspace` — including four new tests: deterministic and mutually distinct
  rendering across all five locales, tag resolution including traditional-script and malformed
  input, schema identity survival in every locale, and INV-007 across the locale set at every tick.
- `apps/observer/src-tauri/src/session.rs` — locale digest test widened from two locales to five,
  and extended to compare payload bytes, not only digests.
- `pnpm typecheck`, `pnpm lint`, `pnpm build`.
- `pnpm audit:i18n` — rewritten for this architecture; see below.

## Rejected and deferred

- **Rejected:** keeping `apps/observer/src/locales/` from PR #17. It typed a `LocaleCopy` shape
  with 60 flat keys against a UI that needs 251 nested ones, and nothing imported it.
- **Rejected:** an explicit `interface Copy` declared separately from any dictionary. It duplicates
  251 keys and lets the declaration and the baseline drift.
- **Rejected:** optional locales with a runtime fallback per key (`Partial<Record<...>>`). It makes
  a missing translation invisible at build time, which is how the previous attempt passed its own
  audit.
- **Rejected:** locale-aware compact magnitudes (`20.3k` → `2.03万`). It changes how a number reads
  between locales, and the observer's numbers are compared across sessions.
- **Deferred:** traditional Chinese, right-to-left layout, and locale-specific date formats. None
  is needed by the current surface, and each is a real tranche rather than a translation pass.

## Documentation changes

`CHANGELOG.md`, `README.md`, `docs/explanation/localization.md` (rewritten against the implemented
architecture), `docs/architecture/invariants.md` (INV-006, INV-007), `docs/ui/observer-application.md`,
`docs/ontology/domain-coverage-matrix.md`, `docs/roadmap/roadmap.md`,
`docs/development/todo-backlog.md` (`TODO-UI-006`), and this plan.

## TODO changes

- `TODO-UI-006` — opened and completed.

## Progress

- Wave 1 — the whole slice, integrated and verified together: baseline dictionary split, three new
  dictionaries, widened `ObserverLocale`, locale-keyed data across four files, the unit-leak fix,
  persistence and system detection, the switcher and palette entries, the widened Rust renderer,
  four new tests, and the rewritten audit tool.

## Audit tool

`tools/audit/validate-i18n.mjs` was rewritten. The previous tool asserted key parity, which
TypeScript now catches earlier and with a better message. The rewrite checks what the compiler
cannot see: placeholder parity, untranslated leakage against an allowlist that states a reason per
key, empty values, agreement between the four places that enumerate locales independently, and
placeholder parity inside the Rust `[&str; 5]` template tables — where `rustc` checks arity but
nothing checks that `{name}` survives into all five entries.

The tool was verified against seeded regressions: a placeholder dropped from a German string and a
`{name}` removed from the German Rust template were both caught, with a non-zero exit.
