#!/usr/bin/env node
/** Render bounded crate-owned TeX laws into static MathML for Rustdoc. */

import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import {
  lstat,
  mkdir,
  readFile,
  readdir,
  rename,
  rm,
  writeFile,
} from "node:fs/promises";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { TextDecoder } from "node:util";

const MINIMUM_NODE_MAJOR = 20;
const TOOL_ROOT = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(TOOL_ROOT, "../..");
const MAX_FORMULAS = 256;
const MAX_TEX_BYTES = 8 * 1024;
const MAX_MATHML_BYTES = 128 * 1024;
const CRATE_COLLECTIONS = Object.freeze([
  "crates",
  "adapters",
  "interfaces",
  "sdk",
  "integrations",
  "tools",
]);
const STANDALONE_COLLECTIONS = Object.freeze([
  "interfaces",
  "sdk",
  "integrations",
  "tools",
]);
const FORBIDDEN_COMMAND = /\\(?:class|data|def|edef|futurelet|gdef|global|globaldefs|href|html|htmlClass|htmlData|htmlId|htmlStyle|id|include|includegraphics|input|let|long|newcommand|openout|providecommand|read|renewcommand|style|url|write|xdef)(?![A-Za-z])/u;
const UTF8 = new TextDecoder("utf-8", { fatal: true });

export function assertSupportedNode(version = process.versions.node) {
  const match = /^(\d+)\./u.exec(version);
  const major = match ? Number.parseInt(match[1], 10) : Number.NaN;
  if (!Number.isSafeInteger(major) || major < MINIMUM_NODE_MAJOR) {
    throw new Error(`Rustdoc math requires Node.js ${MINIMUM_NODE_MAJOR} or newer; found ${version}`);
  }
}

/** Resolve Cargo's shared build directory without imposing a host-specific path. */
export function resolveCargoTargetRoot(
  repositoryRoot = REPOSITORY_ROOT,
  configuredTarget = process.env.CARGO_TARGET_DIR,
) {
  const selected = configuredTarget?.trim();
  return selected ? resolve(repositoryRoot, selected) : join(repositoryRoot, "target");
}

assertSupportedNode();

function compareNames(left, right) {
  if (left.name < right.name) return -1;
  if (left.name > right.name) return 1;
  return 0;
}

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function confined(root, candidate) {
  const normalizedRoot = resolve(root);
  const normalizedCandidate = resolve(candidate);
  if (
    normalizedCandidate !== normalizedRoot &&
    !normalizedCandidate.startsWith(`${normalizedRoot}${sep}`)
  ) {
    throw new Error(`path escapes repository root: ${candidate}`);
  }
  return normalizedCandidate;
}

async function regularFile(path, label) {
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(`${label} must be a regular non-symlink file: ${path}`);
  }
  return stat;
}

async function regularDirectory(path, label) {
  const stat = await lstat(path);
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    throw new Error(`${label} must be a regular non-symlink directory: ${path}`);
  }
  return stat;
}

export async function verifyVendor(repositoryRoot = REPOSITORY_ROOT) {
  const metadataPath = confined(
    repositoryRoot,
    join(repositoryRoot, "tools/rustdoc-math/vendor.json"),
  );
  await regularFile(metadataPath, "vendor metadata");
  const metadata = JSON.parse(await readFile(metadataPath, "utf8"));
  if (metadata.name !== "KaTeX" || metadata.version !== "0.18.4") {
    throw new Error("unexpected Rustdoc-math renderer coordinate");
  }

  const modulePath = confined(repositoryRoot, join(repositoryRoot, metadata.module_path));
  const licensePath = confined(repositoryRoot, join(repositoryRoot, metadata.license_path));
  await regularFile(modulePath, "vendored renderer");
  await regularFile(licensePath, "vendored renderer license");
  const [moduleBytes, licenseBytes] = await Promise.all([
    readFile(modulePath),
    readFile(licensePath),
  ]);
  if (digest(moduleBytes) !== metadata.module_sha256) {
    throw new Error("vendored KaTeX module hash does not match vendor.json");
  }
  if (digest(licenseBytes) !== metadata.license_sha256) {
    throw new Error("vendored KaTeX license hash does not match vendor.json");
  }
  return { metadata, modulePath };
}

function canonicalTex(bytes, path) {
  if (bytes.length === 0 || bytes.length > MAX_TEX_BYTES) {
    throw new Error(`${path}: TeX must contain 1..=${MAX_TEX_BYTES} bytes`);
  }
  let source;
  try {
    source = UTF8.decode(bytes);
  } catch {
    throw new Error(`${path}: TeX must be valid UTF-8`);
  }
  if (
    source.includes("\0") ||
    source.includes("\r") ||
    !source.endsWith("\n") ||
    source.indexOf("\n") !== source.length - 1
  ) {
    throw new Error(`${path}: TeX must be one canonical LF-terminated line`);
  }
  const tex = source.slice(0, -1);
  if (!tex || tex !== tex.trim()) {
    throw new Error(`${path}: TeX must have no surrounding whitespace`);
  }
  if (
    tex.includes("$") ||
    tex.includes("\\(") ||
    tex.includes("\\)") ||
    tex.includes("\\[") ||
    tex.includes("\\]") ||
    /\\(?:begin|end)\s*\{/u.test(tex)
  ) {
    throw new Error(`${path}: store one bare TeX expression without delimiters`);
  }
  if (FORBIDDEN_COMMAND.test(tex)) {
    throw new Error(`${path}: TeX contains a forbidden active-content command`);
  }
  return tex;
}

function mathKind(path) {
  if (path.endsWith(".display.tex")) return "display";
  if (path.endsWith(".inline.tex")) return "inline";
  throw new Error(`${path}: expected .display.tex or .inline.tex suffix`);
}

function outputPath(path) {
  return path.slice(0, -extname(path).length) + ".mathml";
}

export function renderFormula(katex, tex, kind, sourceLabel = "formula") {
  if (kind !== "display" && kind !== "inline") {
    throw new Error(`${sourceLabel}: math kind must be display or inline`);
  }
  if (FORBIDDEN_COMMAND.test(tex)) {
    throw new Error(`${sourceLabel}: TeX contains a forbidden active-content command`);
  }
  if (Buffer.byteLength(tex, "utf8") > MAX_TEX_BYTES) {
    throw new Error(`${sourceLabel}: TeX exceeds the ${MAX_TEX_BYTES}-byte limit`);
  }
  const rendered = katex.renderToString(tex, {
    displayMode: kind === "display",
    globalGroup: false,
    macros: {},
    maxExpand: 1000,
    maxSize: 20,
    output: "mathml",
    strict: "error",
    throwOnError: true,
    trust: false,
  });
  const match = rendered.match(/<math\b[\s\S]*<\/math>/u);
  if (!match || rendered.match(/<math\b/gu)?.length !== 1) {
    throw new Error(`${sourceLabel}: KaTeX did not emit exactly one MathML root`);
  }
  const mathml = match[0];
  if (Buffer.byteLength(mathml, "utf8") > MAX_MATHML_BYTES) {
    throw new Error(`${sourceLabel}: generated MathML exceeds the ${MAX_MATHML_BYTES}-byte limit`);
  }
  const withoutNamespace = mathml.replace(
    'xmlns="http://www.w3.org/1998/Math/MathML"',
    "",
  );
  if (/<(?:script|link)\b|\b(?:href|src)=|https?:\/\/|url\s*\(/iu.test(withoutNamespace)) {
    throw new Error(`${sourceLabel}: generated MathML contains an external or active reference`);
  }
  if (!mathml.includes('<annotation encoding="application/x-tex">')) {
    throw new Error(`${sourceLabel}: generated MathML lost its TeX annotation`);
  }
  const escapedTex = tex.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
  if (!mathml.includes(`<annotation encoding="application/x-tex">${escapedTex}</annotation>`)) {
    throw new Error(`${sourceLabel}: generated MathML changed its TeX annotation`);
  }
  const rustdocSafeMathml = mathml
    .replaceAll("\\", "&#92;")
    .replaceAll("_", "&#95;")
    .replaceAll("[", "&#91;")
    .replaceAll("]", "&#93;");
  return `${rustdocSafeMathml}\n`;
}

async function walk(directory, accept, files = []) {
  const entries = await readdir(directory, { withFileTypes: true });
  entries.sort(compareNames);
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      throw new Error(`symlinks are forbidden in Rustdoc math custody: ${path}`);
    }
    if (entry.isDirectory()) {
      await walk(path, accept, files);
    } else if (entry.isFile() && accept(path)) {
      files.push(path);
    }
  }
  return files;
}

async function appendCrateMathFiles(crateRoot, files) {
  const mathRoot = join(crateRoot, "docs/math");
  try {
    const stat = await lstat(mathRoot);
    if (!stat.isDirectory() || stat.isSymbolicLink()) {
      throw new Error(`crate math root must be a regular directory: ${mathRoot}`);
    }
    await walk(mathRoot, (path) => path.endsWith(".tex") || path.endsWith(".mathml"), files);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

async function governedMathFiles(cratesRoot, files = []) {
  const crates = await readdir(cratesRoot, { withFileTypes: true });
  crates.sort(compareNames);
  for (const crate of crates) {
    if (crate.isSymbolicLink()) {
      throw new Error(`crate symlinks are outside Rustdoc math custody: ${join(cratesRoot, crate.name)}`);
    }
    if (!crate.isDirectory()) continue;
    await appendCrateMathFiles(join(cratesRoot, crate.name), files);
  }
  return files;
}

async function repositoryMathFiles(repositoryRoot) {
  const files = [];
  for (const collection of CRATE_COLLECTIONS) {
    const root = confined(repositoryRoot, join(repositoryRoot, collection));
    try {
      const stat = await lstat(root);
      if (!stat.isDirectory() || stat.isSymbolicLink()) {
        throw new Error(`crate collection must be a regular directory: ${root}`);
      }
      await governedMathFiles(root, files);
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  return files;
}

async function selectedMathFiles(repositoryRoot, mathRoot) {
  if (mathRoot === undefined) return repositoryMathFiles(repositoryRoot);
  const root = confined(repositoryRoot, mathRoot);
  await regularDirectory(root, "crate root");
  return governedMathFiles(root);
}

async function writeAtomic(path, bytes) {
  await mkdir(dirname(path), { recursive: true });
  const temporary = `${path}.pale-stage-${process.pid}`;
  await rm(temporary, { force: true });
  try {
    await writeFile(temporary, bytes, { flag: "wx", mode: 0o644 });
    await rename(temporary, path);
  } finally {
    await rm(temporary, { force: true });
  }
}

export async function renderTree({
  repositoryRoot = REPOSITORY_ROOT,
  mode,
  mathRoot,
} = {}) {
  if (!new Set(["check", "write"]).has(mode)) {
    throw new Error("render mode must be check or write");
  }
  const { modulePath } = await verifyVendor(repositoryRoot);
  const { default: katex } = await import(pathToFileURL(modulePath));
  const files = await selectedMathFiles(repositoryRoot, mathRoot);
  const sources = files.filter((path) => path.endsWith(".tex"));
  const actualOutputs = new Set(files.filter((path) => path.endsWith(".mathml")));
  if (sources.length === 0 || sources.length > MAX_FORMULAS) {
    throw new Error(`expected 1..=${MAX_FORMULAS} governed TeX formulas, found ${sources.length}`);
  }

  const expectedOutputs = new Set(sources.map(outputPath));
  const orphans = [...actualOutputs].filter((path) => !expectedOutputs.has(path));
  if (orphans.length > 0) {
    throw new Error(`orphan generated MathML: ${orphans.map((path) => relative(repositoryRoot, path)).join(", ")}`);
  }
  let changed = 0;
  for (const sourcePath of sources) {
    await regularFile(sourcePath, "TeX source");
    const tex = canonicalTex(await readFile(sourcePath), relative(repositoryRoot, sourcePath));
    const rendered = renderFormula(katex, tex, mathKind(sourcePath), relative(repositoryRoot, sourcePath));
    const destination = outputPath(sourcePath);
    let current = null;
    try {
      await regularFile(destination, "generated MathML");
      current = await readFile(destination, "utf8");
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
    if (current !== rendered) {
      changed += 1;
      if (mode === "check") {
        throw new Error(`${relative(repositoryRoot, destination)} is missing or stale`);
      }
      await writeAtomic(destination, rendered);
    }
  }
  return { formulas: sources.length, changed };
}

export async function verifyRenderedHtml({
  repositoryRoot = REPOSITORY_ROOT,
  htmlRoot = join(repositoryRoot, "target/doc"),
  mathRoot,
} = {}) {
  await renderTree({ repositoryRoot, mathRoot, mode: "check" });
  const governedFiles = await selectedMathFiles(repositoryRoot, mathRoot);
  const outputs = governedFiles.filter((path) => path.endsWith(".mathml"));
  const expectedRoots = [];
  for (const output of outputs) {
    const content = await readFile(output, "utf8");
    const match = content.match(/<math\b[\s\S]*<\/math>/u);
    if (!match) throw new Error(`${relative(repositoryRoot, output)} has no MathML root`);
    expectedRoots.push([output, withoutTexAnnotation(match[0])]);
  }

  // Rendered documentation may live in Cargo's caller-selected target
  // directory. It is read-only evidence, so require a real directory and let
  // the recursive walk retain its existing symlink rejection.
  const root = resolve(htmlRoot);
  await regularDirectory(root, "rendered Rustdoc root");
  const htmlFiles = (await walk(root, (path) => path.endsWith(".html"))).filter(
    (path) => !path.split(sep).includes("src"),
  );
  const outputByRoot = new Map();
  for (const [output, expected] of expectedRoots) {
    if (outputByRoot.has(expected)) {
      throw new Error(
        `governed formulas render identical MathML: ${relative(repositoryRoot, outputByRoot.get(expected))}, ${relative(repositoryRoot, output)}`,
      );
    }
    outputByRoot.set(expected, output);
  }
  const occurrenceCounts = new Map(expectedRoots.map(([path]) => [path, 0]));
  for (const htmlFile of htmlFiles) {
    const html = await readFile(htmlFile, "utf8");
    for (const match of html.matchAll(/<math\b[\s\S]*?<\/math>/gu)) {
      const comparableRoot = withoutTexAnnotation(match[0]);
      const output = outputByRoot.get(comparableRoot);
      if (!output) {
        throw new Error(`${relative(repositoryRoot, htmlFile)} contains ungoverned MathML`);
      }
      occurrenceCounts.set(output, occurrenceCounts.get(output) + 1);
    }
    const proseWithoutAnnotations = html
      .replace(/<annotation encoding="application\/x-tex">[\s\S]*?<\/annotation>/gu, "")
      .replace(/<pre\b[\s\S]*?<\/pre>/giu, "")
      .replace(/<code\b[\s\S]*?<\/code>/giu, "");
    if (/(\\\(|\\\)|\\\[|\\\]|\$\$|\\(?:begin|end)\{)/u.test(proseWithoutAnnotations)) {
      throw new Error(`${relative(repositoryRoot, htmlFile)} retains raw mathematical delimiters`);
    }
  }
  for (const [output, count] of occurrenceCounts) {
    if (count === 0) {
      throw new Error(`${relative(repositoryRoot, output)} does not render in Rustdoc`);
    }
  }
  return { formulas: expectedRoots.length, htmlFiles: htmlFiles.length };
}

async function hasRustdocTarget(crateRoot) {
  const sourceRoot = join(crateRoot, "src");
  try {
    await regularDirectory(sourceRoot, "Rustdoc source root");
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
  for (const member of ["lib.rs", "main.rs"]) {
    try {
      await regularFile(join(sourceRoot, member), "Rustdoc target");
      return true;
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  return false;
}

/** Discover direct standalone Cargo source homes outside the root workspace. */
export async function standaloneDocManifests(repositoryRoot = REPOSITORY_ROOT) {
  const manifests = [];
  for (const collection of STANDALONE_COLLECTIONS) {
    const collectionRoot = confined(repositoryRoot, join(repositoryRoot, collection));
    let entries;
    try {
      await regularDirectory(collectionRoot, "standalone source-home collection");
      entries = await readdir(collectionRoot, { withFileTypes: true });
    } catch (error) {
      if (error?.code === "ENOENT") continue;
      throw error;
    }
    entries.sort(compareNames);
    for (const entry of entries) {
      if (entry.isSymbolicLink()) {
        throw new Error(
          `standalone source-home symlinks are forbidden: ${join(collectionRoot, entry.name)}`,
        );
      }
      if (!entry.isDirectory()) continue;
      const crateRoot = join(collectionRoot, entry.name);
      if (!(await hasRustdocTarget(crateRoot))) continue;
      const manifest = join(crateRoot, "Cargo.toml");
      await regularFile(manifest, "standalone Cargo manifest");
      manifests.push(manifest);
    }
  }
  return manifests;
}

async function runCargoDoc(repositoryRoot, targetRoot, arguments_) {
  await new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(
      "cargo",
      ["doc", "--quiet", "--no-deps", "--locked", "--target-dir", targetRoot, ...arguments_],
      {
        cwd: repositoryRoot,
        env: {
          ...process.env,
          RUSTDOCFLAGS: "-D warnings --document-private-items",
        },
        stdio: "inherit",
      },
    );
    child.once("error", rejectPromise);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolvePromise();
      } else {
        rejectPromise(new Error(`cargo doc failed with ${signal ?? `exit code ${code}`}`));
      }
    });
  });
}

/** Build and verify one clean rendered-doc custody for every Rust source home. */
export async function verifyRepositoryRustdoc(repositoryRoot = REPOSITORY_ROOT) {
  await renderTree({ repositoryRoot, mode: "check" });
  const targetRoot = resolveCargoTargetRoot(repositoryRoot);
  await runCargoDoc(repositoryRoot, targetRoot, ["--workspace"]);
  await runCargoDoc(repositoryRoot, targetRoot, [
    "--manifest-path",
    join(repositoryRoot, "Cargo.toml"),
    "--package",
    "pale-adapter-control-storage",
    "--features",
    "bench",
  ]);
  const manifests = await standaloneDocManifests(repositoryRoot);
  for (const manifest of manifests) {
    await runCargoDoc(repositoryRoot, targetRoot, [
      "--manifest-path",
      manifest,
      "--all-features",
    ]);
  }
  const rendered = await verifyRenderedHtml({
    repositoryRoot,
    htmlRoot: join(targetRoot, "doc"),
  });
  return { ...rendered, sourceHomes: manifests.length + 1 };
}

function withoutTexAnnotation(value) {
  return value
    .replace(
      /(<annotation encoding="application\/x-tex">)[\s\S]*?(<\/annotation>)/gu,
      "$1$2",
    )
    .replaceAll("&#92;", "\\")
    .replaceAll("&#95;", "_")
    .replaceAll("&#91;", "[")
    .replaceAll("&#93;", "]");
}

async function main() {
  const [argument, ...rest] = process.argv.slice(2);
  if (new Set(["--check", "--write"]).has(argument) && rest.length === 0) {
    const result = await renderTree({ mode: argument.slice(2) });
    process.stdout.write(`rustdoc math: formulas=${result.formulas} changed=${result.changed}\n`);
    return;
  }
  if (argument === "--check-html" && rest.length <= 1) {
    const result = await verifyRenderedHtml({
      htmlRoot: rest[0] ? resolve(rest[0]) : join(REPOSITORY_ROOT, "target/doc"),
    });
    process.stdout.write(`rustdoc math HTML: formulas=${result.formulas} files=${result.htmlFiles}\n`);
    return;
  }
  if (argument === "--check-rustdoc" && rest.length === 0) {
    const result = await verifyRepositoryRustdoc();
    process.stdout.write(
      `rustdoc math repository: formulas=${result.formulas} `
        + `files=${result.htmlFiles} source-homes=${result.sourceHomes}\n`,
    );
    return;
  }
  throw new Error(
    "usage: node tools/rustdoc-math/render.mjs "
      + "--check|--write|--check-rustdoc|--check-html [target/doc]",
  );
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  main().catch((error) => {
    process.stderr.write(`rustdoc math: ${error.message}\n`);
    process.exitCode = 1;
  });
}
