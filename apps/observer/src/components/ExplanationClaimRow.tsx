import type { ExplanationClaim, NumericClaimValue } from "@causafera/observer-protocol";

import type { Copy } from "../i18n";

interface ExplanationClaimRowProps {
  claim: ExplanationClaim;
  label: string;
  evidenceLabel: string;
  copy: Copy;
}

export function ExplanationClaimRow({
  claim,
  label,
  evidenceLabel,
  copy,
}: ExplanationClaimRowProps) {
  const comparison =
    claim.comparison.kind === 0
      ? "—"
      : `${claim.comparison.kind === 1 ? "matched" : "counterfactual"} #${claim.comparison.cohortId ?? "—"}`;
  return (
    <article className="claim-row">
      <div className="claim-row__identity">
        <span className="schema-id numeric">{claim.schemaId.toString().padStart(2, "0")}</span>
        <div>
          <strong>{label}</strong>
          <span>
            {evidenceLabel} · {claim.evidenceTraceIds.length} {copy.tracesCount} · {copy.comparison}: {comparison}
          </span>
        </div>
      </div>
      <div className="claim-row__value numeric">{formatClaimValue(claim.value)}</div>
      <div className="confidence-cell">
        <span>{copy.confidence}</span>
        <strong className="numeric">{Math.round(claim.confidence * 100)}%</strong>
        <i><span style={{ width: `${claim.confidence * 100}%` }} /></i>
      </div>
    </article>
  );
}

function formatClaimValue(value: NumericClaimValue): string {
  if (value.kind === "scalar") return value.value.toString();
  if (value.kind === "range") return `${value.start} — ${value.end}`;
  return `${value.numerator} / ${value.denominator}`;
}
