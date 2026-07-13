import type { RuntimeSummary } from "@ontopolis/observer-protocol";

import type { Copy } from "../i18n";

interface CausalFlowProps {
  summary?: RuntimeSummary;
  copy: Copy;
}

export function CausalFlow({ summary, copy }: CausalFlowProps) {
  const stages = [
    { label: copy.physics, primary: summary?.physicalEvents, detail: copy.physicalEvents },
    { label: copy.mana, primary: summary?.manaCellChanges, detail: copy.manaEffects, secondary: summary?.manaPhysicalEffects },
    { label: copy.resolution, primary: summary?.resolutionTransitions, detail: copy.resolutionTransitions },
    { label: copy.actions, primary: summary?.actorActionsCommitted, detail: copy.rejected, secondary: summary?.actorActionsRejected },
    { label: copy.population, primary: summary?.populationTotal, detail: copy.movements, secondary: summary?.populationMovements },
  ];

  return (
    <section className="panel causal-flow">
      <div className="panel-heading">
        <span className="eyebrow">{copy.causalLoop}</span>
        <h2>{copy.causalActivity}</h2>
      </div>
      <ol className="causal-stages">
        {stages.map((stage, index) => (
          <li key={stage.label}>
            <span className="stage-index numeric">{String(index + 1).padStart(2, "0")}</span>
            <div>
              <strong>{stage.label}</strong>
              <span>{stage.detail}</span>
            </div>
            <div className="stage-value numeric">
              <strong>{stage.primary?.toString() ?? "—"}</strong>
              {stage.secondary !== undefined && <small>{stage.secondary.toString()}</small>}
            </div>
          </li>
        ))}
      </ol>
    </section>
  );
}
