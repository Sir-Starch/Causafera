interface DefinitionRowProps {
  term: string;
  value: string;
}

export function DefinitionRow({ term, value }: DefinitionRowProps) {
  return (
    <div>
      <dt>{term}</dt>
      <dd className="numeric">{value}</dd>
    </div>
  );
}
