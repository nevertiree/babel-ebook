import { spawn, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";

export async function cleanupBrowserProcesses() {
  // E2E tests launch the Tauri app which spawns WebView2 processes. On Windows
  // these processes often survive a graceful app exit and can conflict with the
  // next test's CDP connection. Force-kill any leftover app/WebView2 processes
  // before launching a new instance.
  for (const image of ["babel-ebook-desktop.exe", "msedgewebview2.exe"]) {
    try {
      spawn("taskkill", ["/F", "/IM", image], { shell: true });
    } catch {
      // ignore cleanup errors
    }
  }
  // Give Windows a moment to release handles and ports.
  await new Promise((r) => setTimeout(r, 1500));
}

export async function getFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (address && typeof address === "object" && address.port) {
        const port = address.port;
        server.close(() => resolve(port));
      } else {
        server.close(() => reject(new Error("could not get free port")));
      }
    });
    server.on("error", reject);
  });
}

export async function waitForCdp(cdpUrl: string, retries = 30): Promise<boolean> {
  for (let i = 0; i < retries; i += 1) {
    try {
      const res = await fetch(`${cdpUrl}/json/version`);
      if (res.ok) return true;
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  return false;
}

export async function forceKill(appProcess: ChildProcess | null) {
  if (!appProcess) return;
  if (!appProcess.killed) {
    appProcess.kill();
  }
  // On Windows a graceful kill may not terminate the WebView2 process tree;
  // force-kill by PID after a short delay.
  await new Promise((r) => setTimeout(r, 1000));
  if (appProcess.pid) {
    try {
      spawn("taskkill", ["/F", "/T", "/PID", String(appProcess.pid)], { shell: true });
    } catch {
      // ignore cleanup errors
    }
  }
}
