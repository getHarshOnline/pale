// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import { NapiCli } from "@napi-rs/cli";
import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { nativeDeclarations } from "./declarations.mjs";

const root = new URL("../", import.meta.url);
await mkdir(new URL("target/", root), { recursive: true });
const manifest = JSON.parse(await readFile(new URL("package.json", root), "utf8"));
const binaryName = manifest.napi?.binaryName;
if (typeof binaryName !== "string" || binaryName.length === 0) {
  throw new Error("package.json must define napi.binaryName");
}
const build = await new NapiCli().build({
  cwd: fileURLToPath(root), platform: true, release: true,
  cargoOptions: ["--locked"], dts: "target/napi-raw.d.ts",
});
await build.task;
for (const entry of await readdir(root, { withFileTypes: true })) {
  if (
    entry.isFile()
    && entry.name.endsWith(".node")
    && !entry.name.startsWith(`${binaryName}.`)
  ) {
    await rm(new URL(entry.name, root));
  }
}
await writeFile(new URL("index.d.ts", root), nativeDeclarations(
  await readFile(new URL("target/napi-raw.d.ts", root), "utf8"),
));
