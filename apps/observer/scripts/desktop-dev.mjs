import { spawn } from "node:child_process";

const env = { ...process.env };
const onLinux = process.platform === "linux";
const wayland = Boolean(env.WAYLAND_DISPLAY) && Boolean(env.DISPLAY);
const nativeWayland = env.CAUSAFERA_NATIVE_WAYLAND === "1";

/*
 * Two separate problems, two separate switches.
 *
 * WebKitGTK can terminate the whole Tauri process with GDK protocol error 71 on some Wayland,
 * NVIDIA and remote-desktop combinations. XWayland plus the DMABUF renderer disabled is the
 * narrow fix for that, and it costs nothing at run time.
 *
 * Disabling the compositor and forcing software GL is a much heavier hammer: it turns off
 * layer compositing, so every scroll repaints the whole window on the CPU. It is a last
 * resort for machines that still fail, not a default — the interface is noticeably slower
 * under it than the same build is in a browser.
 */
if (onLinux && wayland && !nativeWayland) {
  env.GDK_BACKEND = "x11";
  env.WEBKIT_DISABLE_DMABUF_RENDERER = "1";
  console.info(
    "Causafera Observer: Wayland detected; using XWayland with the DMABUF renderer disabled.",
  );
  console.info(
    "Set CAUSAFERA_NATIVE_WAYLAND=1 or run desktop:raw to test native Wayland.",
  );
}

if (env.CAUSAFERA_SOFTWARE_RENDER === "1") {
  env.WEBKIT_DISABLE_COMPOSITING_MODE = "1";
  env.LIBGL_ALWAYS_SOFTWARE = "1";
  console.info(
    "Causafera Observer: software rendering requested; compositing is off and scrolling will be slow.",
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
