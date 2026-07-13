import type { ConnectionState } from "../useObserverSession";
import type { Copy } from "../i18n";

interface ConnectionStatusProps {
  connection: ConnectionState;
  isPlaying: boolean;
  ticks?: bigint;
  copy: Copy;
}

export function ConnectionStatus({
  connection,
  isPlaying,
  ticks,
  copy,
}: ConnectionStatusProps) {
  const label =
    connection === "connected"
      ? copy.connected
      : connection === "connecting"
        ? copy.connecting
        : connection === "unavailable"
          ? copy.unavailable
          : copy.error;

  return (
    <div className="connection-status" aria-live="polite">
      <span className={`status-dot status-dot--${connection}`} aria-hidden="true" />
      <span>{label}</span>
      {connection === "connected" && (
        <>
          <span className="separator" aria-hidden="true" />
          <span>{isPlaying ? copy.playing : copy.paused}</span>
          <span className="separator" aria-hidden="true" />
          <span className="numeric">
            {copy.tick} {ticks?.toString() ?? "0"}
          </span>
        </>
      )}
    </div>
  );
}
