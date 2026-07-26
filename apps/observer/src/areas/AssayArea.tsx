/**
 * Assay — the Explanation workspace.
 *
 * Explanation is read-only and never authoritative (INV-012, INV-013). Every claim is shown
 * with its evidence state, confidence and trace anchors together (INV-026): there is no view
 * here that displays a value without them.
 *
 * `Unknown` is presented as a result, not as a missing field. Absence of evidence is not
 * negative evidence, and the interface states that where a reader would otherwise assume it.
 */

import { useMemo, useState } from "react";

import { Assessment, EvidenceState, type ExplanationClaim } from "@causafera/observer-protocol";

import {
  Field,
  Fields,
  Meter,
  Notice,
  Panel,
  Readout,
  StatusTag,
  TraceChip,
  Unsurveyed,
} from "../components/primitives";
import {
  assessmentTone,
  claimDelta,
  claimDescriptor,
  COMPARISON_CONTEXT,
  evidenceTone,
  formatClaimValue,
  type StatusTone,
} from "../observer/claims";
import { formatPercent, type ObserverLocale } from "../observer/format";
import { useActions, useCopy, useSession } from "../observer/instance";
import { SIGNAL_VARIABLE } from "../observer/models";
import type { AreaProps } from "../workspace";

function toneLabel(tone: StatusTone, copy: ReturnType<typeof useCopy>): string {
  if (tone === "supported") return copy.assay.supported;
  if (tone === "partial") return copy.assay.partial;
  if (tone === "unsupported") return copy.assay.unsupported;
  return copy.assay.unknown;
}

export function AssayArea({ workspace, update }: AreaProps) {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const report = useSession((state) => state.explanation);
  const explanationTicks = useSession((state) => state.explanationTicks);
  const currentTicks = useSession((state) => state.summary?.simulationTicks);
  const analyzing = useSession((state) => state.analyzing);
  const attached = useSession((state) => state.connection === "connected");
  const actions = useActions();
  const [frameIndex, setFrameIndex] = useState(0);

  const frame = report?.frames[Math.min(frameIndex, report.frames.length - 1)];
  const stale =
    report !== undefined &&
    explanationTicks !== undefined &&
    currentTicks !== undefined &&
    explanationTicks !== currentTicks;

  const unknownCount = useMemo(
    () =>
      frame?.claims.filter((claim) => claim.evidenceState === EvidenceState.Unknown).length ?? 0,
    [frame],
  );

  const runButton = (
    <button
      type="button"
      className="btn btn--primary btn--lg"
      disabled={!attached || analyzing}
      onClick={actions.analyze}
    >
      {analyzing ? copy.assay.running : copy.assay.run}
    </button>
  );

  return (
    <>
      <div className="area-head">
        <div className="area-head__meta">{runButton}</div>
      </div>

      {stale && (
        <Notice tone="caution">
          <span>
            <b>{copy.assay.staleTitle}.</b> {copy.assay.staleBody} ({explanationTicks?.toString()} →{" "}
            {currentTicks?.toString()})
          </span>
        </Notice>
      )}

      {report === undefined || frame === undefined ? (
        <Unsurveyed title={copy.assay.empty} centred action={runButton}>
          {copy.assay.emptyBody}
        </Unsurveyed>
      ) : (
        <>
          <Panel variant="accent" flushBody>
            <div className="cluster">
              <Readout
                label={copy.assay.experiment}
                value={`#${report.experimentId}`}
                signal="trace"
                size="compact"
              />
              <Readout
                label={copy.assay.checkpoint}
                value={frame.checkpointTicks.toString()}
                unit={copy.transport.ticks.toLowerCase()}
                signal="physical"
                size="compact"
              />
              <Readout
                label={copy.assay.overall}
                value={
                  <StatusTag
                    tone={assessmentTone(report.overallAssessment)}
                    label={toneLabel(assessmentTone(report.overallAssessment), copy)}
                  />
                }
                signal="resolution"
                size="compact"
              />
              <Readout
                label={copy.assay.claims}
                value={frame.claims.length}
                signal="trace"
                size="compact"
              />
              <Readout
                label={copy.assay.unknown}
                value={unknownCount}
                note={unknownCount > 0 ? copy.assay.unknownTitle : undefined}
                signal="refused"
                size="compact"
              />
            </div>
          </Panel>

          {report.frames.length > 1 && (
            <div className="tabs" role="tablist" aria-label={copy.assay.frames}>
              {report.frames.map((candidate, index) => (
                <button
                  key={index}
                  type="button"
                  role="tab"
                  className="tabs__tab"
                  aria-selected={index === frameIndex}
                  onClick={() => setFrameIndex(index)}
                >
                  {copy.assay.checkpoint} {candidate.checkpointTicks.toString()}
                </button>
              ))}
            </div>
          )}

          <div className="grid grid--halves">
            {frame.claims.map((claim, index) => (
              <ClaimCard
                key={`${claim.schemaId}:${index}`}
                claim={claim}
                locale={locale}
                copy={copy}
                active={workspace.traceFilter !== undefined && claim.evidenceTraceIds.includes(workspace.traceFilter)}
                traceFilter={workspace.traceFilter}
                onTrace={(id) =>
                  update({ traceFilter: workspace.traceFilter === id ? undefined : id })
                }
              />
            ))}
          </div>

          {unknownCount > 0 && (
            <Notice tone="survey">
              <span>
                <b>{copy.assay.unknownTitle}.</b> {copy.assay.unknownBody}
              </span>
            </Notice>
          )}
        </>
      )}
    </>
  );
}

function ClaimCard({
  claim,
  locale,
  copy,
  active,
  traceFilter,
  onTrace,
}: {
  claim: ExplanationClaim;
  locale: ObserverLocale;
  copy: ReturnType<typeof useCopy>;
  active: boolean;
  traceFilter?: bigint;
  onTrace(id: bigint): void;
}) {
  const descriptor = claimDescriptor(claim.schemaId);
  const tone = evidenceTone(claim.evidenceState);
  const delta = claimDelta(claim.value);
  const comparison = COMPARISON_CONTEXT[claim.comparison.kind]?.[locale];

  return (
    <Panel
      variant={active ? "accent" : "default"}
      eyebrow={`${copy.assay.schema} ${claim.schemaId.toString()}`}
      title={descriptor?.title[locale] ?? copy.assay.unknownSchema}
      tools={<StatusTag tone={tone} label={toneLabel(tone, copy)} />}
    >
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          gap: "var(--s3)",
          padding: "var(--s2) 0 var(--s3)",
        }}
      >
        <span
          className="numeric"
          style={{
            fontSize: "1.75rem",
            lineHeight: 1,
            color: descriptor === undefined ? "var(--ink)" : SIGNAL_VARIABLE[descriptor.signal],
          }}
        >
          {formatClaimValue(claim.value, descriptor, locale)}
        </span>
        {descriptor?.unit !== undefined && (
          <span className="readout__unit">{descriptor.unit[locale]}</span>
        )}
        {delta !== undefined && delta !== 0 && (
          <span className="delta" data-direction={delta > 0 ? "up" : "down"}>
            {delta > 0 ? "▲" : "▼"} {Math.abs(delta)}
          </span>
        )}
      </div>

      <Meter
        fraction={claim.confidence}
        signal={descriptor?.signal ?? "trace"}
        left={copy.assay.confidence}
        right={formatPercent(claim.confidence, 0)}
      />

      <p className="lede" style={{ marginTop: "var(--s3)" }}>
        {descriptor?.reading[locale] ?? copy.assay.unknownSchemaBody}
      </p>

      <Fields>
        <Field label={copy.assay.comparison} text>
          {comparison ?? `#${claim.comparison.kind}`}
        </Field>
        <Field label={copy.assay.traces} stacked>
          {claim.evidenceTraceIds.length === 0 ? (
            <span className="muted">{copy.common.none}</span>
          ) : (
            <span className="trace-chips">
              {claim.evidenceTraceIds.map((id) => (
                <TraceChip
                  key={id.toString()}
                  id={id}
                  active={traceFilter === id}
                  onSelect={onTrace}
                />
              ))}
            </span>
          )}
        </Field>
      </Fields>
    </Panel>
  );
}

export function AssayDock() {
  const copy = useCopy();
  const report = useSession((state) => state.explanation);
  const explanationTicks = useSession((state) => state.explanationTicks);

  return (
    <>
      <Panel variant="flush" title={copy.assay.evidence} eyebrow={copy.assay.eyebrow}>
        <Fields>
          <Field label={<StatusTag tone="supported" label={copy.assay.supported} />} text>
            {copy.common.readOnly}
          </Field>
          <Field label={<StatusTag tone="partial" label={copy.assay.partial} />} text>
            {copy.common.readOnly}
          </Field>
          <Field label={<StatusTag tone="unsupported" label={copy.assay.unsupported} />} text>
            {copy.common.readOnly}
          </Field>
          <Field label={<StatusTag tone="unknown" label={copy.assay.unknown} />} text>
            {copy.common.readOnly}
          </Field>
        </Fields>
        <p className="lede" style={{ marginTop: "var(--s3)" }}>
          {copy.assay.unknownBody}
        </p>
      </Panel>

      {report !== undefined && (
        <Panel variant="flush" title={copy.assay.frames} eyebrow={copy.common.bounded}>
          <Fields>
            <Field label={copy.assay.experiment}>#{report.experimentId.toString()}</Field>
            <Field label={copy.assay.frames}>{report.frames.length}</Field>
            <Field label={copy.transport.ticks}>{explanationTicks?.toString() ?? "—"}</Field>
            <Field label={copy.assay.overall}>
              <StatusTag
                tone={assessmentTone(report.overallAssessment)}
                label={
                  report.overallAssessment === Assessment.Supported
                    ? copy.assay.supported
                    : report.overallAssessment === Assessment.Partial
                      ? copy.assay.partial
                      : report.overallAssessment === Assessment.Unsupported
                        ? copy.assay.unsupported
                        : copy.assay.unknown
                }
              />
            </Field>
          </Fields>
        </Panel>
      )}
    </>
  );
}
