# Observer Projection Gaps

Requests from the observer frontend to the observer layer, ordered by how much interface they
unblock per unit of backend work. Each entry names the read model or wire encoding required, so a
later agent can pick one up without re-deriving the need.

Protocol changes require an ExecPlan (`PLANS.md`). Nothing here is implemented in the frontend
against imagined data: the capability register in `src/observer/capability.ts` states each of these
as unavailable, and the interface renders that state rather than a placeholder.

## 1. Deterministic explanation text over the wire

**Needed:** transport for `RenderedExplanation` (the proto message already exists) produced by the
Rust `DeterministicExplanationRenderer`, as a query kind or as a field on the Explanation response.

**Why:** the renderer is authoritative and already emits Russian and English templates. The
frontend deliberately does not reproduce its sentences — a TypeScript port would be a second
implementation of authoritative wording that could silently diverge. Today the Assay area presents
claim structure with per-schema reading notes instead. With the rendered text transported, the area
gains the authoritative narrative beside the structure at no risk of divergence.

**Effort:** low. No new read model; the renderer output only needs encoding.

## 2. Resolution policy thresholds

**Needed:** the level thresholds and the relevance ceiling of the active `ResolutionPolicy` in the
runtime summary or the world snapshot.

**Why:** the observer receives `resolution_relevance` and `resolution_level` but not the scale they
live on, so relevance can only be shown as a raw integer. With the thresholds, relevance becomes a
ladder with the level boundaries marked — the difference between a number and a measurement.

**Effort:** low. Additive scalar fields on an existing message.

## 3. Trace ancestry query

**Needed:** a query kind over `CausalTraceStore` returning a bounded ancestry window for a trace
identifier: parents, depth, and the committed event kind.

**Why:** trace anchors are already threaded through the whole interface — transitions, gate
transitions, and Explanation claims all carry them, and selecting one filters the ledger. That is
the full extent of provenance navigation the protocol allows. An ancestry window turns the anchors
into a walkable chain, which is the single largest analytical capability the observer is missing.

**Effort:** medium. The store supports traversal; the query surface, wire encoding, and a decoder
are new. Bounding must be explicit — ancestry is unbounded in principle.

## 4. Per-cell mana and resolution fields

**Needed:** a spatial read model projecting per-cell mana intensity and resolution relevance for a
requested chunk, bounded by cell count.

**Why:** the runtime holds both fields; the observer receives chunk totals and one peak value. The
chart profile currently shows a chunk as a single mana magnitude. With a per-cell projection the
Survey area gains a real field plate rather than a summary bar, and the existing canvas layer can
render it without new infrastructure.

**Effort:** medium. A 32³ chunk is 32768 cells, so the projection needs a slice or downsample
contract rather than a full dump.

## 5. Performance telemetry

**Needed:** wire encoding for the existing `PerformanceMetrics` message.

**Why:** the Instrument area measures the client side of every exchange — bytes, duration, outcome
— from real transport activity. The runtime side of the same picture is missing, so the area can
report what the observer costs but not what the simulation costs.

**Effort:** low. Proto defined, metrics partially collected.

## 6. Entity summaries

**Needed:** an `EntitySummary` read model and wire encoding.

**Why:** actors and population appear only as counts. Entity summaries would give the observer its
first per-agent surface. This is deliberately ranked last: it is the largest new surface, it
depends on decisions about which entity attributes are observable at all, and it must not become a
back door to Ground Truth identity (INV-027).

**Effort:** high, and it needs a contract decision before an implementation.

## Not requested

- **Streaming subscriptions.** The stream hub already supports multiple streams, but request and
  response over the Tauri bridge is adequate at the current tick rates, and the demand registry
  already stops traffic for closed panels. Revisit when a second stream kind has real data.
- **Historical state queries.** Blocked on persistence maturity, not on the observer layer. The
  frontend labels its own series as observer-side rather than implying stored history.

## Related Documents

- `docs/ui/observer-application.md` — frontend architecture
- `docs/observer/architecture.md` — observer pipeline
- `docs/architecture/protocol.md` — protocol boundaries and versioning
- `docs/observer/capability-maturity-map.md` — capability readiness assessment
