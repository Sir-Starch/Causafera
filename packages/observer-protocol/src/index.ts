// Protocol types placeholder
export interface ConnectRequest {
  clientVersion: string;
  locale: string;
}

export interface ConnectResponse {
  serverVersion: string;
  protocolVersion: number;
  currentTime: { ticks: number };
}
