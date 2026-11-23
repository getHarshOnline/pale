import assert from "node:assert/strict";
import {
  copyFile,
  mkdtemp,
  mkdir,
  readFile,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import test from "node:test";

import {
  assertSupportedNode,
  renderFormula,
  renderTree,
  resolveCargoTargetRoot,
  standaloneDocManifests,
  verifyRenderedHtml,
  verifyVendor,
} from "../render.mjs";

const TOOL_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const REPOSITORY_ROOT = resolve(TOOL_ROOT, "../..");
const { modulePath } = await verifyVendor();
const { default: katex } = await import(pathToFileURL(modulePath));

test("declares and enforces the supported Node.js floor", () => {
  assert.doesNotThrow(() => assertSupportedNode("20.0.0"));
  assert.throws(() => assertSupportedNode("19.9.0"), /Node\.js 20 or newer/u);
  assert.throws(() => assertSupportedNode("unknown"), /found unknown/u);
});

test("reuses Cargo's portable target directory", () => {
  const repositoryRoot = resolve("fixture-repository");
  assert.equal(resolveCargoTargetRoot(repositoryRoot, undefined), join(repositoryRoot, "target"));
  assert.equal(
    resolveCargoTargetRoot(repositoryRoot, "custom-target"),
    join(repositoryRoot, "custom-target"),
  );
  assert.equal(
    resolveCargoTargetRoot(repositoryRoot, join(repositoryRoot, "absolute-target")),
    join(repositoryRoot, "absolute-target"),
  );
  assert.equal(resolveCargoTargetRoot(repositoryRoot, "  "), join(repositoryRoot, "target"));
});

test("renders deterministic static MathML with exact TeX custody", () => {
  const tex = String.raw`Q_{\mathrm{used}}[G]=\sum_{i=1}^{n}Q_i`;
  const first = renderFormula(katex, tex, "display", "test formula");
  const second = renderFormula(katex, tex, "display", "test formula");
  assert.equal(first, second);
  assert.match(first, /<math[^>]*display="block"/u);
  assert.match(first, /<annotation encoding="application\/x-tex">Q&#95;/u);
  assert.match(first, /&#91;G&#93;/u);
  assert.doesNotMatch(first, /<(?:script|link)\b|\b(?:href|src)=/iu);

  const inline = renderFormula(katex, String.raw`n\ge1`, "inline", "inline test");
  assert.match(inline, /<math\b/u);
  assert.doesNotMatch(inline, /display="block"/u);
});

test("rejects malformed and active-content TeX", () => {
  assert.throws(
    () => renderFormula(katex, String.raw`\frac{1`, "display", "broken"),
    /KaTeX parse error/u,
  );
  for (const command of [
    String.raw`\def\x{y}`,
    String.raw`\gdef\x{y}`,
    String.raw`\edef\x{y}`,
    String.raw`\xdef\x{y}`,
    String.raw`\let\x=y`,
    String.raw`\futurelet\x\y z`,
    String.raw`\newcommand{\x}{y}`,
    String.raw`\renewcommand{\x}{y}`,
    String.raw`\providecommand{\x}{y}`,
    String.raw`\global\def\x{y}`,
    String.raw`\href{https:\/\/example.test}{x}`,
    String.raw`\htmlClass{unsafe}{x}`,
    String.raw`\includegraphics{asset}`,
    String.raw`\input{asset}`,
    String.raw`\write18{command}`,
    String.raw`\write18foo`,
    String.raw`\openout1file`,
    String.raw`\def1`,
  ]) {
    assert.throws(
      () => renderFormula(katex, command, "inline", "active"),
      /forbidden active-content/u,
      command,
    );
  }
});

test("bounds both TeX input and generated MathML", () => {
  assert.throws(
    () => renderFormula(katex, "x".repeat(8 * 1024 + 1), "inline", "oversized source"),
    /TeX exceeds/u,
  );
  const expansion = Array.from({ length: 800 }, () => String.raw`\boxed{x}`).join("+");
  assert.throws(
    () => renderFormula(katex, expansion, "display", "oversized output"),
    /generated MathML exceeds/u,
  );
});

test("writes then checks a confined one-to-one source tree", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const mathRoot = join(root, "crates/example/docs/math");
  await mkdir(mathRoot, { recursive: true });
  await writeFile(join(mathRoot, "law.display.tex"), "x^2+y^2=z^2\n");

  const written = await renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "write" });
  assert.deepEqual(written, { formulas: 1, changed: 1 });
  const checked = await renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "check" });
  assert.deepEqual(checked, { formulas: 1, changed: 0 });
});

test("default custody covers collection and standalone crate roots", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-layout-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const adapterMath = join(root, "adapters/example/docs/math");
  const apiMath = join(root, "sdk/rust/docs/math");
  await mkdir(adapterMath, { recursive: true });
  await mkdir(apiMath, { recursive: true });
  await writeFile(join(adapterMath, "adapter.display.tex"), "a=b\n");
  await writeFile(join(apiMath, "api.inline.tex"), "x=y\n");

  const written = await renderTree({ repositoryRoot: root, mode: "write" });
  assert.deepEqual(written, { formulas: 2, changed: 2 });
  const checked = await renderTree({ repositoryRoot: root, mode: "check" });
  assert.deepEqual(checked, { formulas: 2, changed: 0 });
});

test("discovers direct standalone Cargo source homes with Rustdoc targets", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-source-homes-");
  context.after(() => rm(root, { recursive: true, force: true }));
  for (const member of ["sdk/rust", "interfaces/lsp", "sdk/js", "tools/test-only"]) {
    await mkdir(join(root, member), { recursive: true });
    await writeFile(
      join(root, member, "Cargo.toml"),
      "[package]\nname='fixture'\nversion='0.0.1'\n",
    );
  }
  await mkdir(join(root, "sdk/rust/src"), { recursive: true });
  await mkdir(join(root, "interfaces/lsp/src"), { recursive: true });
  await mkdir(join(root, "sdk/js/src"), { recursive: true });
  await writeFile(join(root, "sdk/rust/src/lib.rs"), "//! API\n");
  await writeFile(join(root, "interfaces/lsp/src/main.rs"), "//! LSP\nfn main() {}\n");
  await writeFile(join(root, "sdk/js/src/lib.rs"), "//! Node\n");

  const manifests = await standaloneDocManifests(root);
  assert.deepEqual(
    manifests.map((manifest) => manifest.slice(root.length + 1)),
    ["interfaces/lsp/Cargo.toml", "sdk/js/Cargo.toml", "sdk/rust/Cargo.toml"],
  );
});

test("rejects a symlinked standalone Rust source root", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-source-home-boundary-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const crateRoot = join(root, "interfaces/lsp");
  await mkdir(crateRoot, { recursive: true });
  await writeFile(
    join(crateRoot, "Cargo.toml"),
    "[package]\nname='fixture'\nversion='0.0.1'\n",
  );
  await symlink(join(REPOSITORY_ROOT, "sdk/rust/src"), join(crateRoot, "src"));

  await assert.rejects(
    () => standaloneDocManifests(root),
    /Rustdoc source root must be a regular non-symlink directory/u,
  );
});

test("rejects symlinks in governed math custody", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-boundary-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const mathRoot = join(root, "crates/example/docs/math");
  await mkdir(mathRoot, { recursive: true });
  await symlink(join(REPOSITORY_ROOT, "README.md"), join(mathRoot, "escape.display.tex"));
  await assert.rejects(
    () => renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "check" }),
    /symlinks are forbidden/u,
  );
});

test("rejects a symlinked caller-selected crate root", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-root-boundary-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const externalCrates = await mkdtemp(join(tmpdir(), "pale-rustdoc-math-external-"));
  context.after(() => rm(externalCrates, { recursive: true, force: true }));
  await rm(join(root, "crates"), { recursive: true });
  await symlink(externalCrates, join(root, "crates"));
  await assert.rejects(
    () => renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "check" }),
    /crate root must be a regular non-symlink directory/u,
  );
});

test("rejects orphan outputs and noncanonical TeX bytes", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-orphan-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const mathRoot = join(root, "crates/example/docs/math");
  await mkdir(mathRoot, { recursive: true });
  await writeFile(join(mathRoot, "law.display.tex"), "\\operatorname{not\\_started}=x\n");
  await writeFile(join(mathRoot, "orphan.display.mathml"), "<math/>\n");
  await assert.rejects(
    () => renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "check" }),
    /orphan generated MathML/u,
  );

  await rm(join(mathRoot, "orphan.display.mathml"));
  await writeFile(join(mathRoot, "law.display.tex"), Buffer.from([0xff, 0x0a]));
  await assert.rejects(
    () => renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "check" }),
    /valid UTF-8/u,
  );
});

test("rejects a vendored module whose bytes drift", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-vendor-");
  context.after(() => rm(root, { recursive: true, force: true }));
  const metadata = JSON.parse(
    await readFile(join(root, "tools/rustdoc-math/vendor.json"), "utf8"),
  );
  await writeFile(join(root, metadata.module_path), "tampered\n");
  await assert.rejects(() => verifyVendor(root), /module hash does not match/u);
});

test("verifies the actual rendered Rustdoc tree", async (context) => {
  const root = await temporaryRepository("pale-rustdoc-math-html-");
  const target = await mkdtemp(join(tmpdir(), "pale-rustdoc-math-target-"));
  context.after(() => rm(root, { recursive: true, force: true }));
  context.after(() => rm(target, { recursive: true, force: true }));
  const mathRoot = join(root, "crates/example/docs/math");
  const htmlRoot = join(target, "doc");
  await mkdir(mathRoot, { recursive: true });
  await mkdir(join(htmlRoot, "src"), { recursive: true });
  await writeFile(join(mathRoot, "law.display.tex"), "x=[y]\n");
  await renderTree({ repositoryRoot: root, mathRoot: join(root, "crates"), mode: "write" });
  const mathml = await readFile(join(mathRoot, "law.display.mathml"), "utf8");
  const rootElement = mathml.match(/<math\b[\s\S]*<\/math>/u)[0];
  const rustdocRoot = rootElement
    .replaceAll("\\_", "_")
    .replaceAll("&#91;", "[")
    .replaceAll("&#93;", "]");
  await writeFile(
    join(htmlRoot, "index.html"),
    `<html><body>${rustdocRoot}${rustdocRoot}<pre class="rust item-decl"><code>const PATTERN: &amp;str = r&quot;\\(literal\\)&quot;;</code></pre></body></html>\n`,
  );
  await writeFile(join(htmlRoot, "src/source.html"), "<code>\\(not rendered source\\)</code>\n");
  const result = await verifyRenderedHtml({
    repositoryRoot: root,
    mathRoot: join(root, "crates"),
    htmlRoot,
  });
  assert.deepEqual(result, { formulas: 1, htmlFiles: 1 });

  await writeFile(
    join(htmlRoot, "index.html"),
    `<html><body>${rustdocRoot}${rustdocRoot}<p>\\(raw\\)</p></body></html>\n`,
  );
  await assert.rejects(
    () => verifyRenderedHtml({ repositoryRoot: root, mathRoot: join(root, "crates"), htmlRoot }),
    /retains raw mathematical delimiters/u,
  );
});

async function temporaryRepository(prefix) {
  const root = await mkdtemp(join(tmpdir(), prefix));
  const metadata = JSON.parse(await readFile(join(TOOL_ROOT, "vendor.json"), "utf8"));
  const moduleDestination = join(root, metadata.module_path);
  const licenseDestination = join(root, metadata.license_path);
  await mkdir(dirname(moduleDestination), { recursive: true });
  await mkdir(dirname(licenseDestination), { recursive: true });
  await mkdir(join(root, "tools/rustdoc-math"), { recursive: true });
  await copyFile(join(REPOSITORY_ROOT, metadata.module_path), moduleDestination);
  await copyFile(join(REPOSITORY_ROOT, metadata.license_path), licenseDestination);
  await copyFile(join(TOOL_ROOT, "vendor.json"), join(root, "tools/rustdoc-math/vendor.json"));
  await mkdir(join(root, "crates"), { recursive: true });
  return root;
}
