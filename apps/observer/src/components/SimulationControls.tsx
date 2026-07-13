import { useState } from "react";

import type { Copy } from "../i18n";
import type { ConnectionState, PlaybackRate } from "../useObserverSession";

interface SimulationControlsProps {
  connection: ConnectionState;
  isPlaying: boolean;
  playbackRate: PlaybackRate;
  copy: Copy;
  onTogglePlayback(): void;
  onStep(): Promise<void>;
  onReset(seed: number): Promise<void>;
  onPlaybackRate(rate: PlaybackRate): void;
}

const rates: PlaybackRate[] = [1, 4, 16];

export function SimulationControls({
  connection,
  isPlaying,
  playbackRate,
  copy,
  onTogglePlayback,
  onStep,
  onReset,
  onPlaybackRate,
}: SimulationControlsProps) {
  const [seed, setSeed] = useState(0);
  const disabled = connection !== "connected";

  return (
    <section className="simulation-controls" aria-label={copy.causalActivity}>
      <div className="control-row">
        <button
          className="button button--primary control-main"
          type="button"
          onClick={onTogglePlayback}
          disabled={disabled}
        >
          {isPlaying ? copy.pause : copy.play}
        </button>
        <button
          className="button button--secondary"
          type="button"
          onClick={() => void onStep()}
          disabled={disabled || isPlaying}
        >
          {copy.step}
        </button>
      </div>

      <div className="field-group">
        <span className="field-label">{copy.speed}</span>
        <div className="segmented-control" aria-label={copy.speed}>
          {rates.map((rate) => (
            <button
              key={rate}
              type="button"
              className={playbackRate === rate ? "is-active" : undefined}
              aria-pressed={playbackRate === rate}
              onClick={() => onPlaybackRate(rate)}
              disabled={disabled}
            >
              {rate}
            </button>
          ))}
        </div>
      </div>

      <div className="field-group">
        <label className="field-label" htmlFor="session-seed">
          {copy.seed}
        </label>
        <div className="seed-control">
          <input
            id="session-seed"
            min={0}
            max={4_294_967_295}
            type="number"
            value={seed}
            onChange={(event) => setSeed(Number(event.target.value))}
            disabled={disabled}
          />
          <button
            className="button button--quiet"
            type="button"
            onClick={() => void onReset(seed)}
            disabled={disabled || !Number.isSafeInteger(seed) || seed < 0}
          >
            {copy.reset}
          </button>
        </div>
      </div>
    </section>
  );
}
