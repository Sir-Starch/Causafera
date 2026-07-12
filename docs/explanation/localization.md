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

## Locale Properties

A locale may affect:

- word choice;
- grammatical structures;
- number and date formatting;
- cultural conventions for uncertainty expression;
- formality levels.

## Related Documents

- `docs/explanation/deterministic-rendering.md` - Template-based rendering
- `docs/explanation/glossing.md` - Gloss localization
- `docs/architecture/invariants.md` - INV-006: Simulation has no privileged human UI language
- `docs/architecture/invariants.md` - INV-007: Changing observer locale cannot change simulation state hash
