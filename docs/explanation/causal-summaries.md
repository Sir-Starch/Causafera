# Causal Summaries

Causal summaries reconstruct the history of a phenomenon as a chain of causally connected events. They answer "why" questions by tracing provenance.

## Example Query

> Why does this district prohibit bells after sunset?

**Potential causal graph:**

```text
geological formation
→ quarry extraction
→ bakery oven
→ fermentation anomaly
→ observation
→ prayer hypothesis
→ ritual
→ copper bell adoption
→ stable rhythmic pattern
→ mana response
→ medical diagnostic practice
→ guild authority
→ district regulation
```

## Summary Rendering

The deterministic renderer may produce:

> The prohibition ultimately developed from a bakery practice introduced 164 years earlier. Copper bells became common after bakers misattributed unusual fermentation to prayer. Their synchronized use later altered local mana conditions and contributed to a diagnostic practice adopted by the guild. The modern restriction was introduced after nighttime bell use began producing false diagnostic responses.

The text is derived from causal graph structure. It is not generated as lore.

## Requirements

Causal summaries must:

- trace actual provenance chains;
- identify key transition points;
- indicate where inference fills gaps;
- preserve temporal scale;
- distinguish observed from inferred causes;
- expose confidence at each stage.

## Related Documents

- `docs/explanation/explanation-ir.md` - IR causal trace references
- `docs/explanation/deterministic-rendering.md` - Rendering causal chains
- `docs/explanation/confidence.md` - Confidence in causal claims
- `docs/architecture/invariants.md` - INV-019: Emergence must be inspectable
