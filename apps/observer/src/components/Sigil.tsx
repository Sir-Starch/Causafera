/**
 * Cartographic marks.
 *
 * A small set of drawn glyphs — the instrument sigil, area marks, and the marginal survey
 * lines behind the shell. They carry no data; they are the chart furniture that makes the
 * application read as a survey instrument rather than a dashboard.
 */

export function Sigil({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <circle cx="12" cy="12" r="10.25" stroke="currentColor" strokeWidth="1" opacity="0.55" />
      <circle cx="12" cy="12" r="5.5" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M12 1.5v21M1.5 12h21" stroke="currentColor" strokeWidth="0.75" opacity="0.28" />
      <path d="M12 2.5 15 12l-3 9.5L9 12z" fill="currentColor" opacity="0.9" />
      <circle cx="12" cy="12" r="1.35" fill="var(--paper-deep)" />
    </svg>
  );
}

const AREA_PATHS: Record<string, string> = {
  // Observatory: a horizon with a measured altitude.
  station: "M2 13h16M6 13a4 4 0 0 1 8 0M10 3v4M10 7 7 9.5M10 7l3 2.5",
  // Survey: a transect of chunk columns.
  survey: "M3 16V8m4.5 8V5M12 16v-6m4.5 6V7M2 17.5h16",
  // Flux: a step trace with an event mark.
  flux: "M2.5 15h3.5v-4h4V7h4v-3h3.5",
  // Assay: a balance with an unresolved arm.
  assay: "M10 3.5v13M4 16.5h12M5 7h10M5 7l-2.5 4h5zM15 7l2.5 4h-5z",
  // Instrument: a dial with graduations.
  instrument: "M10 17a7 7 0 1 1 0-14 7 7 0 0 1 0 14zM10 10l3.5-2.5M10 3v1.5M17 10h-1.5M10 17v-1.5M3 10h1.5",
};

export function AreaMark({ area, size = 18 }: { area: string; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.1"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={AREA_PATHS[area] ?? AREA_PATHS.station!} />
    </svg>
  );
}

export function RunMark({ running }: { running: boolean }) {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true">
      {running ? (
        <>
          <rect x="1.5" y="1" width="2.5" height="8" rx="0.5" />
          <rect x="6" y="1" width="2.5" height="8" rx="0.5" />
        </>
      ) : (
        <path d="M2 1l7 4-7 4z" />
      )}
    </svg>
  );
}

export function StepMark() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor" aria-hidden="true">
      <path d="M1.5 1l5 4-5 4z" />
      <rect x="7.5" y="1" width="1.5" height="8" rx="0.5" />
    </svg>
  );
}

export function ResetMark() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.2"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M10 6a4 4 0 1 1-1.4-3.05" />
      <path d="M10.4 1v2.4H8" />
    </svg>
  );
}

