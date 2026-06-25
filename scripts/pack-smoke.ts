import { execFileSync, spawn } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import net from "node:net";

const rootDir = process.cwd();
const keepTemp = process.env.DEVFLEET_KEEP_PACK_SMOKE === "1";

function getPnpmInvocation(args: string[]) {
  const npmExecPath = process.env.npm_execpath;

  if (npmExecPath && existsSync(npmExecPath)) {
    return {
      command: process.execPath,
      args: [npmExecPath, ...args],
    };
  }

  return {
    command: process.platform === "win32" ? "pnpm.cmd" : "pnpm",
    args,
  };
}

function runPnpm(args: string[], cwd: string) {
  const invocation = getPnpmInvocation(args);
  console.log(`[pack-smoke] pnpm ${args.join(" ")}`);
  execFileSync(invocation.command, invocation.args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });
}

function run(command: string, args: string[], cwd: string) {
  console.log(`[pack-smoke] ${command} ${args.join(" ")}`);
  execFileSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });
}

function assertPathExists(filePath: string) {
  if (!existsSync(filePath)) {
    throw new Error(`Expected package path to exist: ${filePath}`);
  }
}

function assertPackageBoundary(packageDir: string) {
  const requiredPaths = [
    "package.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "index.html",
    "src",
    "src-tauri",
    "scripts",
    "vite.config.ts",
  ];

  for (const relativePath of requiredPaths) {
    assertPathExists(path.join(packageDir, relativePath));
  }

  const manifest = JSON.parse(
    readFileSync(path.join(packageDir, "package.json"), "utf8"),
  ) as { private?: boolean };

  if (manifest.private !== true) {
    throw new Error("Expected packed manifest to stay private.");
  }

  const workspaceSettings = readFileSync(
    path.join(packageDir, "pnpm-workspace.yaml"),
    "utf8",
  );

  if (
    !workspaceSettings.includes("onlyBuiltDependencies") ||
    !workspaceSettings.includes("esbuild")
  ) {
    throw new Error("Expected pnpm-workspace.yaml to allow esbuild builds.");
  }

  if (existsSync(path.join(packageDir, ".github"))) {
    throw new Error("Packed source should not include .github workflows.");
  }

  if (existsSync(path.join(packageDir, "src-tauri", "target"))) {
    throw new Error("Packed source should not include src-tauri/target.");
  }
}

function getFreePort() {
  return new Promise<number>((resolve, reject) => {
    const server = net.createServer();

    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("Unable to allocate a local preview port."));
        }
      });
    });
  });
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForPreview(port: number, preview: ReturnType<typeof spawn>) {
  const url = `http://127.0.0.1:${port}/`;
  let lastError = "";

  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (preview.exitCode !== null) {
      throw new Error(`vite preview exited early with code ${preview.exitCode}`);
    }

    try {
      const response = await fetch(url);
      const body = await response.text();

      if (response.status === 200 && body.includes('id="root"')) {
        console.log(`[pack-smoke] preview responded with 200 at ${url}`);
        return;
      }

      lastError = `status=${response.status}`;
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }

    await delay(250);
  }

  throw new Error(`vite preview did not become ready: ${lastError}`);
}

function stopProcessTree(
  child: ReturnType<typeof spawn>,
  signal: NodeJS.Signals = "SIGTERM",
) {
  if (child.exitCode !== null || child.pid === undefined) {
    return;
  }

  if (process.platform === "win32") {
    try {
      execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
        stdio: "ignore",
      });
      return;
    } catch {
      child.kill();
      return;
    }
  }

  try {
    process.kill(-child.pid, signal);
  } catch {
    child.kill(signal);
  }
}

async function waitForExit(child: ReturnType<typeof spawn>, timeoutMs = 5000) {
  if (child.exitCode !== null) {
    return;
  }

  await Promise.race([
    new Promise<void>((resolve) => child.once("exit", () => resolve())),
    delay(timeoutMs),
  ]);
}

async function removeDirWithRetry(dir: string) {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    try {
      rmSync(dir, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === 4) {
        throw error;
      }

      await delay(250);
    }
  }
}

async function smokePreview(packageDir: string) {
  const port = await getFreePort();
  const invocation = getPnpmInvocation([
    "preview",
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
  ]);

  console.log(`[pack-smoke] pnpm preview --host 127.0.0.1 --port ${port}`);

  const preview = spawn(invocation.command, invocation.args, {
    cwd: packageDir,
    env: process.env,
    detached: process.platform !== "win32",
    stdio: ["ignore", "pipe", "pipe"],
  });

  preview.stdout.on("data", (chunk) => process.stdout.write(chunk));
  preview.stderr.on("data", (chunk) => process.stderr.write(chunk));

  try {
    await waitForPreview(port, preview);
  } finally {
    stopProcessTree(preview);
    await waitForExit(preview);
    if (preview.exitCode === null) {
      stopProcessTree(preview, "SIGKILL");
      await waitForExit(preview);
    }
  }
}

async function main() {
  const tempDir = mkdtempSync(path.join(tmpdir(), "devfleet-pack-smoke-"));
  const packDir = path.join(tempDir, "packed");
  const unpackDir = path.join(tempDir, "unpacked");

  mkdirSync(packDir);
  mkdirSync(unpackDir);

  try {
    runPnpm(["pack", "--pack-destination", packDir], rootDir);

    const tarballs = readdirSync(packDir)
      .filter((name) => name.endsWith(".tgz"))
      .map((name) => path.join(packDir, name));

    if (tarballs.length !== 1) {
      throw new Error(`Expected exactly one tarball, found ${tarballs.length}.`);
    }

    run("tar", ["-xzf", tarballs[0], "-C", unpackDir], rootDir);

    const packageDir = path.join(unpackDir, "package");
    assertPackageBoundary(packageDir);

    runPnpm(["install", "--frozen-lockfile"], packageDir);
    runPnpm(["build"], packageDir);
    await smokePreview(packageDir);

    console.log("[pack-smoke] package smoke test passed");
  } finally {
    if (keepTemp) {
      console.log(`[pack-smoke] kept temp directory: ${tempDir}`);
    } else {
      await removeDirWithRetry(tempDir);
    }
  }
}

void main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
