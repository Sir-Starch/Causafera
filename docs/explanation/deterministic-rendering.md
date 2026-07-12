# Deterministic Rendering

Most UI text must not require an LLM. Create localized deterministic renderers for Explanation IR.

## Template-Based Rendering

Example template concept:

```text
[ConceptLabel] originally referred to [origin_feature_summary].
Over [duration], its use became associated with [later_association].
It is now primarily used as [current_classification].
```

**Result:**

> Tren originally referred to people displaying a distinctive rhythmic hand motion. Over 83 years, the term became increasingly associated with South Canal bakery workers. It is now primarily used for a local occupational community.

## Renderer Requirements

The renderer must:

- use localization resources;
- use analytical glosses;
- preserve uncertainty;
- avoid inventing claims;
- never expose raw internal enum debug names.

## Localization Integration

Renderers must integrate with the localization system to produce text in the observer's chosen language. The same Explanation IR must produce equivalent meaning in all supported locales.

## Fallback Behavior

When no suitable template exists:

- use generic descriptive patterns;
- indicate that a specialized renderer is missing;
- preserve all structured information even if presentation is plain.

## Related Documents

- `docs/explanation/explanation-ir.md` - Source representation
- `docs/explanation/localization.md` - Localization system
- `docs/explanation/optional-llm-surface.md` - LLM enhancement (optional)
