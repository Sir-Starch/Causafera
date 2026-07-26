# Localization

The observer UI supports multiple human languages. Localization is an observer concern, not a simulation concern.

## Language Independence

The authoritative simulation must never depend on English strings or any other human language. Changing observer UI locale must not alter authoritative simulation state.

**Mandatory test concept:**

> Identical simulation inputs executed with different observer UI locales must produce identical canonical simulation state hashes.

This is enforced, not merely stated. `crates/causafera-runtime/tests/observer_boundary.rs` advances one runtime per locale from a shared seed and asserts that the physical and history digests stay identical at every tick, and `apps/observer/src-tauri/src/session.rs` asserts the same for the session envelope including its payload bytes.

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

## Locale Properties

A locale may affect:

- word choice;
- grammatical structures;
- number and date formatting;
- cultural conventions for uncertainty expression;
- formality levels.

## Supported Locales

The desktop observer presents itself in five languages:

| Locale Tag | Language | Role |
| :--- | :--- | :--- |
| `en-US` | English | Baseline and fallback |
| `ru-RU` | Russian (Русский) | Complete |
| `zh-Hans` | Simplified Chinese (简体中文) | Complete |
| `de-DE` | German (Deutsch) | Complete |
| `es-ES` | Spanish (Español) | Complete |

`zh-Hans` carries a script subtag rather than a region because the distinction that matters for Chinese is the script. Traditional-script tags (`zh-Hant`, `zh-TW`, `zh-HK`, `zh-MO`) deliberately do **not** resolve to `zh-Hans`: there is no traditional dictionary, and answering such a request with simplified text would misstate the instrument's coverage. They fall back to English like any other unsupported tag.

The tag travels to the protocol handler on connect (`ConnectRequest.locale`) so the session knows which locale is observing. It reaches nothing authoritative.

## Where The Strings Live

Three layers carry human language, and each is checked differently.

**1. UI chrome — `apps/observer/src/i18n/`.**

- `en.ts` is the baseline dictionary. `dictionary.ts` derives `Copy` from it with `typeof en`, so a key added to English is a compile error in the other four until it is translated.
- `ru.ts`, `zh-Hans.ts`, `de.ts`, `es.ts` each declare `const x: Copy`, which is what makes that check bite.
- `index.ts` provides `copyFor(locale)`, the switcher tables (`LOCALES`, `LOCALE_NAMES`, `LOCALE_MARKS`), tag resolution (`normaliseLocale`), and the preference logic below.

**2. Locale-keyed data — outside the dictionaries.**

Some presentation content is data rather than chrome and is keyed by locale in place, as `Record<ObserverLocale, string>`:

- `src/observer/claims.ts` — Explanation claim schema descriptors and comparison contexts;
- `src/observer/capability.ts` — the coverage register;
- `src/map/lens.ts` and `src/map/lenses.ts` — the lens catalogue, its groups and availability states.

Widening `ObserverLocale` makes every one of these records a compile error until it is complete, which is the intended pressure.

**3. Explanation text — `crates/causafera-explanation/src/render.rs`.**

The deterministic renderer is authoritative and is not reimplemented in TypeScript. `ObserverLocale` there is a five-variant enum with `ORDER` fixing the order every translation table is written in; format strings live in `[&str; 5]` tables with named placeholders (`{name}`, `{confidence}`, `{cohort}`). `ObserverLocale::parse` resolves an incoming tag by primary subtag, with the script deciding for Chinese.

## Persistence & Fallback Logic

- **Explicit choice**: a selection made in the language switcher is written to `localStorage` under `"causafera-observer-locale"`. Storage access is guarded — a sandboxed or private-mode window may refuse it, and a refused write must not break the instrument, so the choice still applies to the running session.
- **First run**: with no stored preference, `navigator.languages` is walked in order and the first tag that resolves to a supported locale wins.
- **Fallback**: anything unrecognised resolves to `en-US`. A missing dictionary is never rendered as a key or an empty string.

Switching the locale renegotiates the observer locale with the protocol handler. That renegotiation cannot change any state hash (INV-007); it only tells the session which locale is observing.

## Reaching The Switcher

The meridian carries a single language cell rather than one button per language: five buttons in a bar that dense would crowd out the transport controls. Options name themselves in their own language — a reader who needs the switcher cannot be asked to recognise their language written in a language they do not read. Every locale is also a command in the palette (`Ctrl+K`), searchable by its endonym.

## Automated Verification Engine

```bash
pnpm audit:i18n
```

`tools/audit/validate-i18n.mjs` runs as part of `pnpm lint` and checks what the compiler cannot:

1. **Placeholder parity** — `{n}` present in English but dropped in another locale is a well-typed string that renders a hole.
2. **Untranslated leakage** — a value byte-identical to English elsewhere is either a forgotten translation or a deliberate identity. Deliberate ones (product name, protocol nouns, SI symbols, words genuinely spelled alike) are listed by key with a reason; anything else fails.
3. **Empty values**, which render as an invisible label.
4. **Locale-set agreement** across the places that enumerate locales independently: the `ObserverLocale` union, the switcher tables, the locale-keyed data files, and the Rust enum's `ORDER`.
5. **Rust template parity** — `rustc` checks that a `[&str; 5]` table has five entries, but nothing checks that `{name}` survives into all five. This does.

Key parity itself is left to TypeScript, which catches it earlier and with a better message.

## Related Documents

- `docs/explanation/deterministic-rendering.md` - Template-based rendering
- `docs/explanation/glossing.md` - Gloss localization
- `docs/architecture/invariants.md` - INV-006: Simulation has no privileged human UI language
- `docs/architecture/invariants.md` - INV-007: Changing observer locale cannot change simulation state hash
