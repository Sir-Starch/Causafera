# Localization

The observer UI supports multiple human languages. Localization is an observer concern, not a simulation concern.

## Language Independence

The authoritative simulation must never depend on English strings or any other human language. Changing observer UI locale must not alter authoritative simulation state.

**Mandatory test concept:**

> Identical simulation inputs executed with different observer UI locales must produce identical canonical simulation state hashes.

## Localization Scope

Localization applies to:

- UI labels and menus;
- explanation text;
- glosses and classifications;
- documentation references;
- error messages.

## Localization Pipeline

```text
Explanation IR
↓
locale-specific template selection
↓
localized string lookup
↓
parameter substitution
↓
grammar adaptation
↓
rendered text
```

## Supported Locales

The desktop observer interface currently supports five canonical languages:

| Language Tag | Language Name | Status |
| :--- | :--- | :--- |
| `en` | English | Default / Fallback |
| `ru` | Russian (Русский) | Complete |
| `zh-Hans` | Simplified Chinese (简体中文) | Complete |
| `de` | German (Deutsch) | Complete |
| `es` | Spanish (Español) | Complete |

## Frontend Architecture & Directory Structure

Translation resources for the desktop observer reside under `apps/observer/src/locales/`:

- `types.ts`: Strictly types all static string properties, numerical pluralization rules, and Explanation IR schema descriptions (`LocaleCopy`).
- `en.ts`, `ru.ts`, `zh-Hans.ts`, `de.ts`, `es.ts`: Export compile-time validated dictionaries implementing `LocaleCopy`.
- `index.ts` & `src/i18n.ts`: Provide `copyFor(locale)` selection and re-export localization helpers.

## Persistence & Fallback Logic

- **Session Persistence**: User selections made via the desktop language switcher are saved across restarts in browser `localStorage` under key `"causafera-observer-locale"`.
- **System Detection & Fallback**: When an observer session launches without a saved preference, `navigator.language` is evaluated against canonical supported tags. Any unrecognized or unsupported language automatically resolves to English (`en`).

## Automated Verification Engine

To prevent missing translations, broken plural formatters, or untranslated fallback leakage, the continuous audit pipeline enforces consistency across all dictionaries:

```bash
pnpm audit:i18n
```

Executed via `node tools/audit/validate-i18n.mjs`, this tool:
1. Asserts 100% key parity across all supported dictionary files against the English baseline.
2. Invokes numeric formatters (`formatActiveChunks`, `formatTracesCount`, `formatSchema`, etc.) with test parameters to assert runtime safety and valid plural declension.
3. Checks that non-English dictionaries do not leave descriptive sentences untranslated.

## Related Documents

- `docs/explanation/deterministic-rendering.md` - Template-based rendering
- `docs/explanation/glossing.md` - Gloss localization
- `docs/architecture/invariants.md` - INV-006: Simulation has no privileged human UI language
- `docs/architecture/invariants.md` - INV-007: Changing observer locale cannot change simulation state hash
