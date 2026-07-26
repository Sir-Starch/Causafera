/**
 * Bundles and runs the render smoke check.
 *
 * The check imports application source directly, so it needs the same TSX and path handling
 * Vite gives the browser. esbuild produces one Node bundle in a few milliseconds; CSS imports
 * become inert strings because a server render never consults them.
 */

import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { build } from "esbuild";

const here = fileURLToPath(new URL(".", import.meta.url));
// The bundle must sit inside the package so Node resolves the externalised react packages.
const output = join(here, "..", "node_modules", ".cache", "render-smoke");
const bundle = join(output, "render-smoke.mjs");
mkdirSync(output, { recursive: true });

try {
  await build({
    entryPoints: [join(here, "renderSmoke.tsx")],
    outfile: bundle,
    bundle: true,
    platform: "node",
    format: "esm",
    jsx: "automatic",
    logLevel: "warning",
    loader: { ".css": "text" },
    define: { "import.meta.env": JSON.stringify({ DEV: false }) },
    external: ["react", "react-dom", "react-dom/server"],
  });

  const result = spawnSync(process.execPath, [bundle, ...process.argv.slice(2)], {
    stdio: "inherit",
    cwd: join(here, ".."),
  });
  process.exitCode = result.status ?? 1;
} finally {
  rmSync(output, { recursive: true, force: true });
}
