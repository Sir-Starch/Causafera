import { spawn } from "node:child_process";

const env = { ...process.env };
const useCompatibilityProfile =
  process.platform === "linux" &&
  Boolean(env.WAYLAND_DISPLAY) &&
  Boolean(env.DISPLAY) &&
  env.CAUSAFERA_NATIVE_WAYLAND !== "1";

if (useCompatibilityProfile) {
  // WebKitGTK can terminate the entire Tauri process with GDK protocol error 71
  // on some Wayland, NVIDIA, and remote-desktop combinations. XWayland plus
  // software rendering is slower, but it is the reliable development default.
  env.GDK_BACKEND = "x11";
  env.WEBKIT_DISABLE_DMABUF_RENDERER = "1";
  env.WEBKIT_DISABLE_COMPOSITING_MODE = "1";
  env.LIBGL_ALWAYS_SOFTWARE = "1";

  console.info(
    "Causafera Observer: Wayland detected; using the WebKitGTK compatibility profile.",
  );
  console.info(
    "Set CAUSAFERA_NATIVE_WAYLAND=1 or run desktop:raw to test native Wayland.",
  );
}

const executable = process.platform === "win32" ? "tauri.cmd" : "tauri";
const forwardedArgs = process.argv.slice(2);
if (forwardedArgs[0] === "--") {
  forwardedArgs.shift();
}

const child = spawn(executable, ["dev", ...forwardedArgs], {
  env,
  stdio: "inherit",
});

child.once("error", (error) => {
  console.error(`Unable to start Tauri: ${error.message}`);
  process.exitCode = 1;
});

child.once("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exitCode = code ?? 1;
});
