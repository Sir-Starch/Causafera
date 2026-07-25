import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { installDevChannel } from "./observer/instance";

import "./design/tokens.css";
import "./design/base.css";
import "./design/chrome.css";
import "./design/surfaces.css";
import "./design/controls.css";
import "./design/data.css";
import "./design/viz.css";
import "./design/map.css";

const root = createRoot(document.getElementById("root") as HTMLElement);

async function boot(): Promise<void> {
  // Development builds may replay a recorded observer session when no Tauri bridge exists.
  // The branch is compiled out of production bundles; see `src/dev/replayChannel.ts`.
  if (import.meta.env.DEV) {
    const { createReplayChannel } = await import("./dev/replayChannel");
    installDevChannel(await createReplayChannel());
  }
  root.render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void boot();
