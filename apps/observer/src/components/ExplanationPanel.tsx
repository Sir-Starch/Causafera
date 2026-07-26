import {
  Assessment,
  EvidenceState,
  type ExplanationReport,
} from "@causafera/observer-protocol";

import type { Copy } from "../i18n";
import { ExplanationClaimRow } from "./ExplanationClaimRow";

interface ExplanationPanelProps {
  report?: ExplanationReport;
  isAnalyzing: boolean;
  copy: Copy;
  onAnalyze(): Promise<void>;
}

export function ExplanationPanel({
  report,
  isAnalyzing,
  copy,
  onAnalyze,
}: ExplanationPanelProps) {
  const frame = report?.frames[report.frames.length - 1];

  return (
    <section className="panel explanation-panel">
      <div className="panel-heading panel-heading--row explanation-heading">
        <div>
          <span className="eyebrow">Explanation IR</span>
          <h2>{copy.analysisTitle}</h2>
          <p>{copy.analysisDescription}</p>
        </div>
        <button
          className="button button--primary"
          type="button"
          onClick={() => void onAnalyze()}
          disabled={isAnalyzing}
        >
          {isAnalyzing ? copy.analyzing : copy.runAnalysis}
        </button>
      </div>

      {report === undefined || frame === undefined ? (
        <div className="explanation-empty">
          <p>{copy.noExplanation}</p>
          <small>{copy.projectionNotice}</small>
        </div>
      ) : (
        <>
          <div className="analysis-summary">
            <div>
              <span>{copy.checkpoint}</span>
              <strong className="numeric">{frame.checkpointTicks.toString()}</strong>
            </div>
            <div>
              <span>{copy.evidence}</span>
              <strong className={`assessment assessment--${assessmentClass(frame.overallAssessment)}`}>
                {assessmentLabel(frame.overallAssessment, copy)}
              </strong>
            </div>
            <div>
              <span>{copy.experimentId}</span>
              <strong className="numeric">#{report.experimentId.toString()}</strong>
            </div>
          </div>

          <div className="claim-list">
            {frame.claims.map((claim) => (
              <ExplanationClaimRow
                key={`${claim.schemaId}:${claim.comparison.kind}`}
                claim={claim}
                label={copy.schemaNames[Number(claim.schemaId)] ?? copy.formatUnknownSchema(claim.schemaId.toString())}
                evidenceLabel={evidenceLabel(claim.evidenceState, copy)}
                copy={copy}
              />
            ))}
          </div>
        </>
      )}
    </section>
  );
}

function assessmentClass(assessment: Assessment): string {
  if (assessment === Assessment.Supported) return "supported";
  if (assessment === Assessment.Partial) return "partial";
  if (assessment === Assessment.Unsupported) return "unsupported";
  return "unknown";
}

function assessmentLabel(assessment: Assessment, copy: Copy): string {
  if (assessment === Assessment.Supported) return copy.supported;
  if (assessment === Assessment.Partial) return copy.partial;
  if (assessment === Assessment.Unsupported) return copy.unsupported;
  return copy.unknown;
}

function evidenceLabel(state: EvidenceState, copy: Copy): string {
  if (state === EvidenceState.Supported) return copy.supported;
  if (state === EvidenceState.Unsupported) return copy.unsupported;
  return copy.unknown;
}
