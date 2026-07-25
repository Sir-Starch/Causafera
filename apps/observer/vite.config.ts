import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";

const REPLAY_ROUTE = "/__causafera_dev__/replay.json";
const REPLAY_FILE = fileURLToPath(new URL("./dev/replay/capture.json", import.meta.url));

/**
 * Serves a captured observer session to the development server only.
 *
 * The capture holds real protocol bytes produced by `ObserverSession`; see
 * `src/dev/replayChannel.ts`. It is never part of a production build, and when the file is
 * absent the request 404s and the observer reports itself unattached.
 */
function devReplayCapture(): Plugin {
  return {
    name: "causafera-dev-replay-capture",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use(REPLAY_ROUTE, (_request, response) => {
        void readFile(REPLAY_FILE, "utf8").then(
          (body) => {
            response.setHeader("content-type", "application/json");
            response.end(body);
          },
          () => {
            response.statusCode = 404;
            response.end("{}");
          },
        );
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), devReplayCapture()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**", "**/dev/replay/**"],
    },
  },
});
