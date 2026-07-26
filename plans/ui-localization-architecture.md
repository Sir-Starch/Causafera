# Complete Observer UI Localization Architecture

## Goal

Implement complete, maintainable interface localization for Causafera's observer front-end in five languages: English (`en`), Russian (`ru`), Simplified Chinese (`zh-Hans`), German (`de`), and Spanish (`es`). Ensure zero duplication of localized string literals in components, support interpolation and pluralization, persist user language selection in `localStorage`, fall back safely to English when translations are missing or unselected, and implement automated testing to prevent missing keys or placeholder content.

## Context

The current observer front-end uses a rigid object-map in `i18n.ts` supporting only `"ru-RU"` and `"en-US"`, with several localized string mappings (e.g., `claimLabels` in `ExplanationPanel.tsx`) hardcoded inside individual React components and fragile string manipulations (such as `.toLocaleLowerCase()` on counter units in `WorldViewport.tsx`). Furthermore, project documentation contains historical references to unsupported UI languages (e.g., Ukrainian in INV-006). A mature simulation interface requires clean separation of presentation resources from component rendering, robust multi-locale support, and strict validation of locale independence (INV-007).

## Relevant Invariants

- **INV-006**: UI language is not part of authoritative simulation semantics. The simulation must never depend on human language strings.
- **INV-007**: Changing observer locale cannot change simulation state hash. Identical inputs with different locales must produce identical authoritative digests.
- **INV-011**: Explanation and non-authoritative rendering remain strictly observer-side presentation.
- **INV-039**: No demo fixtures or shortcuts in production runtime sessions.

## Ontology Domains Affected

- **Observer**: Non-authoritative query negotiation and presentation layer.
- **UI**: Tauri/React observer presentation interface and localization pipeline.

## Causal Carriers Affected

None. Localization remains strictly on the observer wire and presentation boundary; no physical or causal carriers inside the authoritative simulation runtime are modified.

## Relevant Documents

- `docs/architecture/invariants.md`
- `docs/explanation/localization.md`
- `docs/ui/observer-application.md`
- `docs/architecture/determinism.md`

## Current State

- `apps/observer/src/i18n.ts` houses a simple inline object containing translations for `"ru-RU"` and `"en-US"`.
- `apps/observer/src/useObserverSession.ts` defines `ObserverLocale = "ru-RU" | "en-US"`, defaulting hardcoded to `"ru-RU"`.
- Several components contain hardcoded strings and localized mappings: `ExplanationPanel.tsx` contains duplicated `claimLabels`, `ExplanationClaimRow.tsx` hardcodes `"matched"` / `"counterfactual"`, and `WorldViewport.tsx` does ad-hoc counter pluralization.
- No automated verification mechanism exists to ensure all locale keys match canonical English or that translations remain complete and free of placeholder text.

## Proposed Architecture

1. **Locale Resource Hierarchy & Types (`apps/observer/src/locales/`)**:
   - Define canonical `ObserverLocale = "en" | "ru" | "zh-Hans" | "de" | "es"`.
   - Create a typed contract (`LocaleCopy`) in `locales/types.ts` that includes all user-visible interface copy, schema claim names (schemas 1–15 plus generic fallbacks), comparison contexts, and helper formatting methods for interpolation and pluralization.
   - Separate dictionaries into dedicated files: `en.ts`, `ru.ts`, `zh-Hans.ts`, `de.ts`, and `es.ts`.

2. **Language Resolution and Persistence (`locales/index.ts` & `useObserverSession.ts`)**:
   - On initialization, query `localStorage` for a saved language preference under key `"causafera-observer-locale"`.
   - If unconfigured or invalid, inspect `window.navigator.languages` / `window.navigator.language`, matching language prefixes (`en`, `ru`, `zh`, `de`, `es`).
   - Fall back deterministically to English (`en`).
   - When the user switches languages, persist the selected code to `localStorage` immediately.

3. **Component Clean-up & Layout Stability**:
   - Update `App.tsx`, `ExplanationPanel.tsx`, `WorldViewport.tsx`, `CausalFlow.tsx`, `ConnectionStatus.tsx`, `ExplanationClaimRow.tsx`, `InspectorPanel.tsx`, `SimulationControls.tsx`, and `TimelinePanel.tsx` to consume translations solely through the centralized copy object.
   - Refine `styles.css` to allow the language switcher (`.locale-control`) and sidebar footer (`.sidebar-footer`) to wrap gracefully without fixed column widths, displaying all five native language names ("English", "Русский", "简体中文", "Deutsch", "Español") without clipping.

4. **Automated Validation Engine**:
   - Add a developer tool (`tools/audit/validate-i18n.mjs` and check script in frontend package) to verify key parity across all supported locales, detecting missing keys, extra keys, or identical untranslated placeholder text across locales.

## Primitive vs Emergent Review

UI localization is an intentional, non-authoritative presentation layer primitive for observer inspection. It does not introduce simulation state or alter emergent causal loops.

## Non-Goals

- Translating canonical simulation internal IDs, debug output, API schema field identities, or logs.
- Redesigning the layout, visual theme, component structure, or navigation of the UI beyond layout accommodations for native language buttons.
- Mutating simulation authoritative language mechanics or ontology.

## Implementation Stages

- **Wave 1: Frontend i18n Architecture, Translation Resources, and Components Refactoring**
  - Create `apps/observer/src/locales/` with `types.ts`, `en.ts`, `ru.ts`, `zh-Hans.ts`, `de.ts`, `es.ts`, and `index.ts`.
  - Refactor `useObserverSession.ts`, `i18n.ts`, and all components in `apps/observer/src/components/` to consume clean localized strings and helper formatting functions.
  - Modify `styles.css` for layout wrapping and native language switcher display.
  - Verify with `pnpm --dir apps/observer lint && pnpm --dir apps/observer typecheck && pnpm --dir apps/observer build`.

- **Wave 2: Automated Localization Verification Engine**
  - Create automated validator script in `tools/audit/validate-i18n.mjs` to enforce strict key completeness and prevent placeholder text across all five locales.
  - Add test script to verify locale fallback, language persistence, browser language detection, and pluralization rules.
  - Integrate into standard testing pipeline and verify all test scripts pass.

- **Wave 3: Backend Verification & Documentation Sync**
  - Verify observer locale independence across all five locale codes in Rust tests (`apps/observer/src-tauri/src/session.rs` and `crates/causafera-runtime/tests/observer_boundary.rs`).
  - Run workspace verification checks.
  - Update `README.md`, `docs/architecture/invariants.md`, `docs/explanation/localization.md`, `docs/ui/observer-application.md`, `CHANGELOG.md`, `docs/development/todo-backlog.md`, `docs/roadmap/roadmap.md`, and `docs/ontology/domain-coverage-matrix.md` to document the five supported languages and remove outdated references to Ukrainian or unexecuted languages.

## Verification

- `pnpm --dir apps/observer lint && pnpm --dir apps/observer typecheck && pnpm --dir apps/observer build`
- `node tools/audit/validate-i18n.mjs`
- `cargo test --workspace --no-default-features`
- Manual UI layout verification for wrapping, persistence, and fallback.

## Benchmark Plan

No performance regressions allowed in observer rendering or wire communication; verification via existing runtime benchmark and test suites.

## Determinism Impact

None; backed by INV-007 verification proving identical physical and history digests regardless of the observer wire locale string.

## Memory Impact

Negligible observer frontend bundle impact from static translation dictionary structures.

## Observer Impact

Allows connecting with and requesting observer sessions using standard language tags (`en`, `ru`, `zh-Hans`, `de`, `es`).

## Explanation Impact

Moves presentation layer Explanation claim labels directly into standardized frontend localized dictionaries without touching Explanation IR structures.

## Persistence Impact

Adds client-side browser `localStorage` persistence under `"causafera-observer-locale"`. Authoritative runtime state and replay logs remain completely locale-agnostic.

## Cross-Domain Effects

None.

## Risks

- Text clipping or button overflow in constrained sidebar layouts; mitigated by making `.locale-control` buttons flex-wrap with adaptive horizontal padding.
- Complex declension in counter strings (e.g. Russian and German); mitigated by explicit plural/formatting helper methods in `LocaleCopy`.

## Documentation Changes

- `README.md`: Restrained mention of supported interface languages.
- `docs/architecture/invariants.md`: Replace Ukrainian reference in INV-006 with supported language list.
- `docs/explanation/localization.md`: Detailed documentation of supported locale codes, directory structure, fallback logic, and verification scripts.
- `docs/ui/observer-application.md`, `CHANGELOG.md`, `docs/development/todo-backlog.md`, `docs/roadmap/roadmap.md`, `docs/ontology/domain-coverage-matrix.md`: Updated to reflect complete multi-locale interface capability.

## TODO Changes

- Mark UI localization backlog entries as resolved in `todo-backlog.md`.

## Decision Log

- Selected `en`, `ru`, `zh-Hans`, `de`, and `es` as canonical locale tags.
- Opted for TypeScript translation dictionaries under `apps/observer/src/locales/` to guarantee compile-time type safety over loose JSON files.

## Progress

- [ ] Wave 1: Frontend i18n Architecture, Translation Resources, and Components Refactoring
- [ ] Wave 2: Automated Localization Verification Engine
- [ ] Wave 3: Backend Verification & Documentation Sync
