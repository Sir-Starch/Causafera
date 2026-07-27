# Media

Images used by the repository's own documentation. Nothing here is concept art, a mock-up, or a
render of a state the software cannot reach: every screenshot is the desktop observer attached to a
real deterministic runtime, and each one records the seed and tick it was taken at so it can be
reproduced.

| File | Shows | Session |
|------|-------|---------|
| `observer-chart.png` | The chart instrument under the mana lens, with measured isolines, the chunk-to-chunk gradient, population and material-surface overlays | Seed 2026, tick 20 |
| `observer-flux.png` | Causal activity: derived rate recorders, the bounded surface condition ladder, the transition ledger and the rate register | Seed 2026, tick 404 |
| `observer-explanation.png` | Typed Explanation IR — six claims with confidence, evidence traces and an overall assessment, including one deliberately marked unknown | Seed 2026, tick 404 |

## Retaking them

```bash
pnpm --filter causafera-observer desktop
```

Set the seed in the meridian, advance to the tick named above, and the same world is drawn: the
runtime is deterministic, so a screenshot is reproducible rather than illustrative. Keep the window
at 1920 × 1080 so the set stays consistent, and keep the file names stable — `README.md` links to
them directly.

An image that no longer matches what the software does is worse than no image. Retake the affected
screenshot in the same change that alters the surface it shows.
