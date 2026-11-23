import { rm } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");

await rm(path.join(root, "dist"), { recursive: true, force: true });

const status = await new Promise((resolve, reject) => {
  const compiler = path.join(root, "node_modules", "typescript", "bin", "tsc");
  const child = spawn(process.execPath, [compiler, "-p", "tsconfig.json"], { cwd: root, stdio: "inherit" });
  child.on("error", reject);
  child.on("close", (code, signal) => resolve(signal === null ? (code ?? 1) : 1));
});

if (status !== 0) process.exitCode = status;
