/**
 * Shared surface primitives.
 *
 * Every panel, readout and status mark in the application is composed from these, so the
 * cartographic treatment (corner registration ticks, hatched unsurveyed ground, reserved
 * status hues) stays consistent as areas are added.
 */

import type { CSSProperties, ReactNode } from "react";

import { SIGNAL_VARIABLE, SIGNAL_WASH, type SignalId } from "../observer/models";
import type { StatusTone } from "../observer/claims";

export function signalStyle(signal: SignalId | undefined): CSSProperties {
  if (signal === undefined) return {};
  return {
    ["--signal" as string]: SIGNAL_VARIABLE[signal],
    ["--signal-wash" as string]: SIGNAL_WASH[signal],
  };
}

/* ----------------------------------------------------------------- panel -- */

interface PanelProps {
  title?: ReactNode;
  eyebrow?: ReactNode;
  lede?: ReactNode;
  tools?: ReactNode;
  foot?: ReactNode;
  variant?: "default" | "flush" | "accent";
  flushBody?: boolean;
  className?: string;
  style?: CSSProperties;
  children?: ReactNode;
}

export function Panel({
  title,
  eyebrow,
  lede,
  tools,
  foot,
  variant = "default",
  flushBody = false,
  className,
  style,
  children,
}: PanelProps) {
  const classes = ["panel"];
  if (variant === "flush") classes.push("panel--flush");
  if (variant === "accent") classes.push("panel--accent");
  if (className !== undefined) classes.push(className);
  return (
    <section className={classes.join(" ")} style={style}>
      {(title !== undefined || tools !== undefined) && (
        <header className="panel__head">
          <div className="panel__titles">
            {eyebrow !== undefined && <span className="eyebrow">{eyebrow}</span>}
            {title !== undefined && <h2>{title}</h2>}
            {lede !== undefined && <p className="lede">{lede}</p>}
          </div>
          {tools !== undefined && <div className="panel__tools">{tools}</div>}
        </header>
      )}
      <div className={flushBody ? "panel__body panel__body--flush" : "panel__body"}>{children}</div>
      {foot !== undefined && <footer className="panel__foot">{foot}</footer>}
    </section>
  );
}

export function Division({ children }: { children: ReactNode }) {
  return <div className="division">{children}</div>;
}

/* --------------------------------------------------------------- readout -- */

interface ReadoutProps {
  label: ReactNode;
  value: ReactNode;
  unit?: ReactNode;
  note?: ReactNode;
  signal?: SignalId;
  size?: "default" | "compact" | "hero";
  children?: ReactNode;
}

export function Readout({
  label,
  value,
  unit,
  note,
  signal,
  size = "default",
  children,
}: ReadoutProps) {
  const classes = ["readout"];
  if (size === "compact") classes.push("readout--compact");
  if (size === "hero") classes.push("readout--hero");
  return (
    <div className={classes.join(" ")} style={signalStyle(signal)}>
      <span className="readout__label">{label}</span>
      <span className="readout__value">
        {value}
        {unit !== undefined && <span className="readout__unit">{unit}</span>}
      </span>
      {note !== undefined && <span className="readout__note">{note}</span>}
      {children}
    </div>
  );
}

/* ---------------------------------------------------------------- fields -- */

export function Fields({ children }: { children: ReactNode }) {
  return <dl className="fields">{children}</dl>;
}

interface FieldProps {
  label: ReactNode;
  children: ReactNode;
  text?: boolean;
  stacked?: boolean;
}

export function Field({ label, children, text = false, stacked = false }: FieldProps) {
  return (
    <div className={stacked ? "field field--stacked" : "field"}>
      <dt className="field__key">{label}</dt>
      <dd className={text ? "field__value field__value--text" : "field__value"}>{children}</dd>
    </div>
  );
}

/* ------------------------------------------------------------------ tags -- */

export function Tag({
  tone,
  children,
  dot = false,
}: {
  tone?: StatusTone | "live" | "quiet";
  children: ReactNode;
  dot?: boolean;
}) {
  const classes = ["tag"];
  if (tone !== undefined) classes.push(`tag--${tone}`);
  return (
    <span className={classes.join(" ")}>
      {dot && <span className="tag__dot" aria-hidden="true" />}
      {children}
    </span>
  );
}

/** Evidence and assessment marks always pair a hue with a word — never colour alone. */
export function StatusTag({ tone, label }: { tone: StatusTone; label: string }) {
  return (
    <Tag tone={tone} dot={tone !== "unknown"}>
      {label}
    </Tag>
  );
}

export function MaturityPips({ level, max = 5 }: { level: number; max?: number }) {
  return (
    <span className="maturity" aria-label={`M${level}`}>
      {Array.from({ length: max }, (_, index) => (
        <span key={index} className="maturity__pip" data-on={index < level} />
      ))}
    </span>
  );
}

/* ----------------------------------------------------------------- meter -- */

export function Meter({
  fraction,
  signal,
  left,
  right,
  ticks = false,
}: {
  fraction: number;
  signal?: SignalId;
  left?: ReactNode;
  right?: ReactNode;
  ticks?: boolean;
}) {
  const clamped = Math.max(0, Math.min(1, Number.isFinite(fraction) ? fraction : 0));
  return (
    <div className="meter" style={signalStyle(signal)}>
      <div className="meter__track">
        <div className="meter__fill" style={{ width: `${clamped * 100}%` }} />
        {ticks && <div className="meter__ticks" aria-hidden="true" />}
      </div>
      {(left !== undefined || right !== undefined) && (
        <div className="meter__caption">
          <span>{left}</span>
          <span>{right}</span>
        </div>
      )}
    </div>
  );
}

/* ------------------------------------------------------- unsurveyed state -- */

export function Unsurveyed({
  title,
  children,
  centred = false,
  action,
}: {
  title: ReactNode;
  children?: ReactNode;
  centred?: boolean;
  action?: ReactNode;
}) {
  return (
    <div className={centred ? "unsurveyed unsurveyed--centred" : "unsurveyed"}>
      <h4>{title}</h4>
      {children !== undefined && <p>{children}</p>}
      {action}
    </div>
  );
}

export function Notice({
  children,
  tone = "survey",
}: {
  children: ReactNode;
  tone?: "survey" | "caution" | "alarm";
}) {
  return <div className={`notice notice--${tone}`}>{children}</div>;
}

export function Derived({ children }: { children: ReactNode }) {
  return <span className="derived">{children}</span>;
}

/* ------------------------------------------------------------- provenance -- */

export function TraceChip({
  id,
  active = false,
  onSelect,
  title,
}: {
  id: bigint;
  active?: boolean;
  onSelect?: (id: bigint) => void;
  title?: string;
}) {
  if (onSelect === undefined) {
    return (
      <span className="trace-chip trace-chip--static" title={title}>
        #{id.toString()}
      </span>
    );
  }
  return (
    <button
      type="button"
      className="trace-chip"
      data-active={active}
      title={title}
      onClick={() => onSelect(id)}
    >
      #{id.toString()}
    </button>
  );
}

/**
 * A digest is rendered as discrete byte cells rather than a single hex run, so that a
 * divergence between two runs is visible positionally. It is identity, not distance
 * (INV-038): no interface here compares digests numerically.
 */
export function DigestPlate({
  label,
  bytes,
  compare,
  count = 6,
  full,
}: {
  label: ReactNode;
  bytes: Uint8Array;
  compare?: Uint8Array;
  count?: number;
  full?: string;
}) {
  const cells: ReactNode[] = [];
  for (let index = 0; index < Math.min(count, bytes.length); index += 1) {
    const value = bytes[index]!;
    const changed = compare !== undefined && compare.length > index && compare[index] !== value;
    cells.push(
      <span key={index} className="digest__byte" data-changed={changed}>
        {value.toString(16).padStart(2, "0")}
      </span>,
    );
  }
  return (
    <div className="digest" title={full}>
      <span className="digest__label">{label}</span>
      <span className="digest__value">{cells}</span>
    </div>
  );
}

/* ------------------------------------------------------------------ lamp -- */

export function Lamp({ state, label }: { state: string; label: string }) {
  return (
    <span className="lamp" data-state={state}>
      <span className="lamp__mark" aria-hidden="true" />
      {label}
    </span>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="kbd">{children}</kbd>;
}
