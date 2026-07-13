interface MetricCardProps {
  label: string;
  value: string;
  detail?: string;
}

export function MetricCard({ label, value, detail }: MetricCardProps) {
  return (
    <article className="metric-card">
      <span>{label}</span>
      <strong className="numeric">{value}</strong>
      {detail !== undefined && <small>{detail}</small>}
    </article>
  );
}
