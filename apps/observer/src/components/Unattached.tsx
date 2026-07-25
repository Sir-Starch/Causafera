/**
 * The unattached state.
 *
 * When the observer has no runtime to read, it shows no readings. Substituting demonstration
 * state here would be the single most damaging thing this interface could do (INV-039), so
 * the empty instrument is designed rather than apologised for: the chart is blank because
 * the ground is unsurveyed, and the register still states what the instrument would read.
 */

import { CAPABILITY_REGISTER } from "../observer/capability";
import { useActions, useCopy, useSession } from "../observer/instance";
import { Division, Panel, Tag } from "./primitives";
import { Sigil } from "./Sigil";

export function Unattached() {
  const copy = useCopy();
  const locale = useSession((state) => state.locale);
  const connection = useSession((state) => state.connection);
  const error = useSession((state) => state.error);
  const actions = useActions();
  const failed = connection === "error";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--s6)",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "100%",
        padding: "var(--s10) var(--s5)",
        textAlign: "center",
      }}
    >
      <div style={{ color: "var(--rule-strong)" }}>
        <Sigil size={72} />
      </div>

      <div style={{ maxWidth: "46rem", display: "flex", flexDirection: "column", gap: "var(--s3)" }}>
        <span className="eyebrow">{copy.observer}</span>
        <h1 className="display">
          {failed ? copy.connection.errorTitle : copy.connection.unavailableTitle}
        </h1>
        <p className="lede" style={{ margin: "0 auto" }}>
          {copy.connection.unavailableBody}
        </p>
        {error !== undefined && (
          <p className="numeric" style={{ color: "var(--state-unsupported)", fontSize: "var(--t-small)" }}>
            {error}
          </p>
        )}
        <div style={{ display: "flex", gap: "var(--s2)", justifyContent: "center", paddingTop: "var(--s2)" }}>
          <button type="button" className="btn btn--primary btn--lg" onClick={actions.reconnect}>
            {copy.connection.reconnect}
          </button>
        </div>
        <p className="chart__caption">{copy.connection.unavailableHint}</p>
      </div>

      <Panel
        title={copy.instrument.register}
        eyebrow={copy.instrument.eyebrow}
        lede={copy.instrument.registerLede}
        style={{ maxWidth: "58rem", width: "100%", textAlign: "left" }}
      >
        {CAPABILITY_REGISTER.map((group) => (
          <div key={group.id}>
            <Division>{group.title[locale]}</Division>
            <div className="trace-chips" style={{ paddingBottom: "var(--s2)" }}>
              {group.entries.map((entry) => (
                <Tag
                  key={entry.id}
                  tone={
                    entry.state === "live"
                      ? "supported"
                      : entry.state === "bounded"
                        ? "partial"
                        : "unknown"
                  }
                >
                  {entry.title[locale]}
                </Tag>
              ))}
            </div>
          </div>
        ))}
      </Panel>
    </div>
  );
}
