import test from "node:test";
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { captureMeasuredPage, preparePage, installPreferenceRoute, loadChromium } from "./capture.mjs";

const PROFILE = fileURLToPath(new URL("./fixtures/capture-profile.json", import.meta.url));
const PREFERENCES = JSON.parse(await readFile(PROFILE, "utf8")).preferences;

test("full-page preparation reaches lazy content and reports failed media", async (t) => {
  let chromium;
  try { chromium = await loadChromium(); }
  catch { t.skip("Playwright unavailable"); return; }
  let browser;
  try { browser = await chromium.launch(); }
  catch { t.skip("Playwright browser unavailable"); return; }
  t.after(() => browser.close());
  const server = createServer((req, res) => {
    if (req.url === "/good.svg") {
      res.setHeader("Content-Type", "image/svg+xml");
      res.end('<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8"/></svg>');
    } else if (req.url === "/missing.png") {
      res.writeHead(404); res.end();
    } else if (req.url === "/traversal-gap") {
      res.setHeader("Content-Type", "text/html");
      res.end('<!doctype html><style>html{scroll-snap-type:y mandatory}body{margin:0}' +
        'h2{scroll-snap-align:start;margin:0}section{height:600px}section:first-of-type{height:1800px}</style>' +
        '<section><h2>Start</h2></section><section><h2>End</h2></section>');
    } else if (req.url === "/traversal-growth") {
      res.setHeader("Content-Type", "text/html");
      res.end('<!doctype html><style>body{margin:0}main{height:1200px}</style><main>Start</main>' +
        '<script>addEventListener("scroll",function grow(){if(scrollY<600)return;' +
        'removeEventListener("scroll",grow);const tail=document.createElement("section");' +
        'tail.style.height="1200px";tail.innerHTML="<div style=height:1150px></div><div>New end</div>";' +
        'document.body.append(tail);new IntersectionObserver(entries=>{if(entries.some(e=>e.isIntersecting))' +
        'window.fixtureReached="grown"}).observe(tail.lastElementChild)})</script>');
    } else {
      res.setHeader("Content-Type", "text/html");
      res.end('<!doctype html><style>body{margin:0}.spacer{height:6000px}</style>' +
        '<div class="spacer"></div><section id="end"><img loading="lazy" width="8" height="8" src="/good.svg">' +
        '<img loading="lazy" width="8" height="8" src="/missing.png"></section>' +
        '<script>new IntersectionObserver(entries=>{if(entries.some(e=>e.isIntersecting))window.fixtureReached="yes"})' +
        '.observe(document.querySelector("#end"))</script>');
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise((resolve) => server.close(resolve)));
  const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  assert.equal(await page.evaluate(() => window.fixtureReached), undefined);
  const { failedImages: failed, documentHeight } = await preparePage(page);
  assert.equal(await page.evaluate(() => window.fixtureReached), "yes");
  assert.equal(await page.evaluate(() => scrollY), 0);
  assert.equal(await page.locator('img[src="/good.svg"]').evaluate((img) => img.naturalWidth), 8);
  assert.equal(failed.length, 1);
  assert.match(failed[0], /missing\.png$/);
  assert.ok(documentHeight > 6000);

  await page.goto(`http://127.0.0.1:${server.address().port}/traversal-growth`);
  assert.equal((await preparePage(page)).documentHeight, 2400);
  assert.equal(await page.evaluate(() => window.fixtureReached), "grown");

  await page.goto(`http://127.0.0.1:${server.address().port}/traversal-gap`);
  await assert.rejects(preparePage(page), /full-page traversal left a gap/);
});


test("System capture uses the actual preference contract rather than the world default", async () => {
  const stored = new Map([["color-mode", "dark"], ["motion-preference", "reduce"]]);
  const previous = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      setItem: (key, value) => stored.set(key, value),
      removeItem: (key) => stored.delete(key),
    },
  });
  try {
    const context = { addInitScript: async (script, args) => script(args) };
    await installPreferenceRoute(context, "system", {
      colorScheme: "light", reducedMotion: "no-preference",
    }, PREFERENCES);
    assert.equal(stored.get("color-mode"), "system");
    assert.equal(stored.has("motion-preference"), false);
    await installPreferenceRoute(context, "explicit", {
      colorScheme: "light", reducedMotion: "no-preference",
    }, PREFERENCES);
    assert.equal(stored.get("color-mode"), "light");
    assert.equal(stored.get("motion-preference"), "allow");
  } finally {
    if (previous === undefined) delete globalThis.localStorage;
    else Object.defineProperty(globalThis, "localStorage", previous);
  }
});

test("capture retries changed measurements and refuses a persistently unstable pair", async () => {
  const url = "http://localhost/";
  const pageFor = (widths) => ({
    evaluate: async () => ({ env: { url }, width: widths.shift() }),
    screenshot: async () => Buffer.from(String(widths[0])),
    url: () => url,
    waitForTimeout: async () => {},
  });
  const stable = await captureMeasuredPage(pageFor([10, 15, 15, 15]), {}, { finalUrl: url });
  assert.equal(stable.attempts, 2);
  assert.equal(stable.measurements.width, 15);
  assert.equal(stable.image.toString(), "15");
  await assert.rejects(captureMeasuredPage(pageFor([10, 15, 20, 25, 30, 35, 40, 45]),
    {}, { finalUrl: url }), /changed across all four capture attempts/);
  const grew = { ...pageFor([]), evaluate: async () => ({ env: { url, scrollY: 0, documentHeight: 1800 } }) };
  grew.screenshot = async () => Buffer.from("grown");
  await assert.rejects(captureMeasuredPage(grew, {}, { finalUrl: url }, { documentHeight: 1200 }),
    /height or top position changed after traversal/);
});

test("mobile fallback cells keep the selected context and reject the wrong layout width", async (t) => {
  const output = await mkdtemp(join(tmpdir(), "wsdesign-mobile-capture-"));
  t.after(() => rm(output, { recursive: true, force: true }));
  const server = createServer((req, res) => {
    const viewport = req.url === "/sized" ? '<meta name="viewport" content="width=device-width,initial-scale=1">' : "";
    res.setHeader("Content-Type", "text/html");
    res.end('<!doctype html><html data-mode="light"><title>Mobile fixture</title>' + viewport +
      '<script>document.documentElement.dataset.mode="system"</script><p>Mobile evidence</p>');
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise((resolve) => server.close(resolve)));
  const base = `http://127.0.0.1:${server.address().port}`;
  const script = fileURLToPath(new URL("./capture.mjs", import.meta.url));
  const result = await promisify(execFile)(process.execPath, [script, "--profile", PROFILE, "--out", output,
    "--widths", "phone", "--states", "dark-reduced", "--routes", "system",
    "--contexts", "mobile", "--no-js", "--forced-colors", base + "/sized", base + "/missing-viewport"])
    .then(({ stdout }) => ({ code: 0, stdout }), (error) => error);
  if (result.code === 2) {
    t.skip(result.stdout.trim());
    return;
  }
  assert.equal(result.code, 1, result.stdout);
  const rows = JSON.parse(await readFile(join(output, "cells.json"), "utf8"));
  const sized = rows.filter((row) => row.url.endsWith("/sized"));
  assert.deepEqual(sized.map((row) => [row.js, Boolean(row.forced)]),
    [[true, false], [false, false], [true, true], [false, true]]);
  assert.ok(sized.every((row) => row.ctx === "mobile" && row.env.media.coarse &&
    row.env.viewport[0] === row.width && row.env.media.dark && row.env.media.reduce && !row.problems.length));
  assert.ok(sized.every((row) => row.env.dataMode === (row.js ? "system" : "light")));
  const missing = rows.filter((row) => row.url.endsWith("/missing-viewport"));
  assert.equal(missing.length, 4);
  assert.ok(missing.every((row) => row.problems.some((problem) => problem.startsWith("emulation: layout viewport width"))));
  assert.ok(missing.every((row) => row.capture === undefined && row.evidence.images.length === 0));
});

test("browser capture keeps distinct pages, measurements, and scroll evidence", async (t) => {
  const output = await mkdtemp(join(tmpdir(), "wsdesign-capture-"));
  t.after(() => rm(output, { recursive: true, force: true }));
  let revision = "";
  const payload = Buffer.alloc(2 * 1024 * 1024, 65);
  const server = createServer((req, res) => {
    const url = new URL(req.url, "http://localhost");
    if (url.pathname === "/redirect") {
      res.writeHead(302, { Location: "/search?view=first" });
      res.end();
      return;
    }
    if (url.pathname === "/search.css") {
      res.writeHead(200, { "Content-Type": "text/css" });
      res.end("h1 { color: #222 }");
      return;
    }
    if (url.pathname === "/whole-index" || url.pathname === "/partial-index") {
      res.writeHead(200, { "Content-Type": "application/octet-stream", "Content-Length": payload.length });
      if (url.pathname === "/whole-index") res.end(payload);
      else {
        res.write(payload.subarray(0, 32768));
        const pending = setTimeout(() => res.end(payload.subarray(32768)), 2500);
        res.once("close", () => clearTimeout(pending));
      }
      return;
    }
    res.writeHead(url.pathname === "/missing" ? 404 : 200, { "Content-Type": "text/html" });
    res.end('<!doctype html><html data-mode="system"><title>Fixture</title>' +
      (url.pathname === "/search" ? '<link rel="stylesheet" href="/search.css">' : "") +
      (url.pathname === "/search" ? '<script>async function consume(path, entire) {' +
        'const c=new AbortController();const r=await fetch(path,{signal:c.signal});' +
        'try { if(entire) await r.arrayBuffer(); else await r.body.getReader().read(); } finally { c.abort(); }' +
        '} consume("/whole-index",true); consume("/partial-index",false);</script>' : "") +
      '<style>.hidden-title { position: absolute; width: 1px; height: 1px; ' +
      'overflow: hidden; white-space: nowrap; clip-path: inset(50%); transform: scale(1.2) } ' +
      '.clipped { width: 18px; overflow: hidden; white-space: nowrap }</style>' +
      '<h1 class="hidden-title">Screen-reader-only title</h1><p>' +
      (url.searchParams.get("view") || "Not found") + revision + "</p>" +
      (url.pathname === "/clipped" ? '<p class="clipped">Visible clipped text</p>' : "") +
      (["/hold", "/animated-hold"].includes(url.pathname) ?
        '<style>#arrival { width:32px; height:24px; background:#004990 }' +
        (url.pathname === "/animated-hold" ? '@keyframes expand { to { width:300px } }' +
          '#arrival { animation:expand 1s linear 30s both }' : "") +
        '</style><div id="arrival"></div>' : "") +
      (["/snap", "/plain-scroll"].includes(url.pathname) ?
        '<style>' + (url.pathname === "/snap" ? 'html { scroll-snap-type:y mandatory }' : "") +
        'body { margin:0 } body > p { display:none } main { height:100vh; ' +
        'scroll-snap-align:start }</style><main>Start</main><main>Middle</main><main>End</main>' : "") +
      (url.pathname === "/targets" ? '<style>body { perspective:500px }' +
        'button { position:absolute; top:120px; height:20px; width:20px; padding:0 }' +
        '#spaced { left:100px } #touching { left:124px } #near-a { left:200px } #near-b { left:222px }' +
        '#near-large { left:300px } #large { left:320px; width:40px; height:40px; top:110px }' +
        '#disabled { left:80px; pointer-events:none } #projected { left:400px; transform:rotate(8deg) }' +
        '#edge { left:421px } summary { min-height:24px }' +
        '#closed { position:absolute; top:120px; left:80px; width:20px; height:20px }</style>' +
        '<button id="spaced">1</button><button id="touching">2</button>' +
        '<button id="near-a">3</button><button id="near-b">4</button>' +
        '<button id="near-large">5</button><button id="large">6</button>' +
        '<button id="disabled" disabled>7</button><button id="projected">8</button><button id="edge">9</button>' +
        '<details><summary>Closed panel</summary><a id="closed" href="/other">Hidden link</a></details>' : "") +
      (url.pathname === "/late" ? '<script>setTimeout(() => history.replaceState(null,"","/other"),75)</script>' : "") +
      (url.pathname === "/reload" ? '<script>if (!sessionStorage.reloaded) setTimeout(() => {' +
        'sessionStorage.reloaded="yes"; location.reload()},75)</script>' : ""));
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise((resolve) => server.close(resolve)));

  const script = fileURLToPath(new URL("./capture.mjs", import.meta.url));
  const base = `http://127.0.0.1:${server.address().port}`;
  const localFile = join(output, "local.html");
  await writeFile(localFile, '<!doctype html><html data-mode="system"><title>Local</title><p>File fixture</p>');
  const urls = ["/search?view=first", "/search?view=second", "/redirect", "/missing", "/late", "/clipped", "/targets",
    "/hold", "/animated-hold", "/snap", "/plain-scroll", "/reload"]
    .map((path) => base + path);
  urls.push(new URL(`file://${localFile}`).href);
  const run = promisify(execFile);
  const result = await run(process.execPath, [script, "--profile", PROFILE, "--out", output, "--widths", "desktop",
    "--states", "light", "--routes", "system,system", "--scrub", "4", "--record-responses", ...urls, urls[0]])
    .then(() => ({ code: 0 }), (error) => error);
  if (result.code === 2) {
    t.skip(result.stdout.trim());
    return;
  }
  assert.equal(result.code, 1, result.stdout);
  const rows = JSON.parse(await readFile(join(output, "cells.json"), "utf8"));
  assert.deepEqual(rows.map((row) => row.url), urls);
  assert.deepEqual(rows.map((row) => row.navigation), [
    { finalUrl: urls[0], status: 200 }, { finalUrl: urls[1], status: 200 },
    { finalUrl: urls[0], status: 200 }, { finalUrl: urls[3], status: 404 },
    { finalUrl: urls[4], status: 200, observedUrl: base + "/other" },
    { finalUrl: urls[5], status: 200 },
    { finalUrl: urls[6], status: 200 },
    { finalUrl: urls[7], status: 200 }, { finalUrl: urls[8], status: 200 },
    { finalUrl: urls[9], status: 200 }, { finalUrl: urls[10], status: 200 },
    { finalUrl: urls[11], status: 200 }, { finalUrl: urls[12], status: 200 },
  ]);
  const passed = rows.slice(0, 3);
  assert.equal(new Set(passed.map((row) => row.capture)).size, passed.length);
  assert.ok(passed.every((row) => row.problems.length === 0));
  assert.notDeepEqual(await readFile(passed[0].capture), await readFile(passed[1].capture));
  assert.equal(rows[3].capture, undefined);
  assert.match(rows[3].problems[0], /navigation.*404/);
  assert.equal(rows[4].capture, undefined);
  assert.match(rows[4].problems[0], /page URL changed during capture/);
  assert.equal(rows[5].clipped.length, 1);
  assert.match(rows[5].clipped[0], /p\.clipped$/);
  const { targets, problems } = rows[6];
  assert.deepEqual(targets.spaced, ["button#spaced 20x20", "button#touching 20x20"]);
  assert.equal(targets.crowded.length, 3);
  assert.match(targets.crowded[0], /button#near-a.*button#near-b/);
  assert.match(targets.crowded[2], /button#near-large.*button#large/);
  assert.equal(problems.filter((problem) => problem.startsWith("target-spacing:")).length, 3);
  assert.equal(targets.manual.length, 2);
  assert.match(targets.manual[0], /#projected.*projected target geometry/);
  assert.match(targets.manual[1], /#edge.*#projected.*uncertain geometry/);
  assert.ok(targets.small.every((box) => !/#(?:closed|disabled)/.test(box)));
  assert.ok((await readFile(rows[7].capture)).equals(await readFile(rows[8].capture)),
    "capture must preserve the native delayed arrival instead of advancing it");
  const snap = rows[9];
  assert.deepEqual(snap.scrub.visits.map(({ requested, down, up }) => [requested, down, up]), [
    [0, 0, 0],
    [Math.round(snap.env.viewport[1] * 2 / 3), snap.env.viewport[1], snap.env.viewport[1]],
    [Math.round(snap.env.viewport[1] * 4 / 3), snap.env.viewport[1], snap.env.viewport[1]],
    [snap.env.viewport[1] * 2, snap.env.viewport[1] * 2, snap.env.viewport[1] * 2],
  ]);
  assert.equal(snap.scrub.captures.length, 4);
  assert.equal(snap.problems.filter((problem) => problem.includes("use a reachable hold")).length, 4);
  const plain = rows[10];
  assert.ok(plain.scrub.visits.every(({ requested, down, up }) => requested === down && requested === up));
  assert.equal(plain.scrub.captures.length, 8);
  assert.deepEqual(plain.problems, []);
  assert.equal(rows[11].capture, undefined);
  assert.match(rows[11].problems[0], /main document changed during capture/);
  assert.match(rows[12].evidence.documentSha256, /^[0-9a-f]{64}$/);
  assert.equal(rows[0].evidence.documentSha256, rows[2].evidence.documentSha256);
  assert.notEqual(rows[0].evidence.documentSha256, rows[1].evidence.documentSha256);
  const [document, stylesheet] = rows[0].evidence.responses;
  assert.equal(document.sha256, rows[0].evidence.documentSha256);
  assert.equal(stylesheet.url, base + "/search.css");
  assert.match(stylesheet.sha256, /^[0-9a-f]{64}$/);
  const captured = new Map(rows[0].evidence.responses.map((response) => [response.url, response]));
  assert.equal(captured.get(base + "/whole-index").sha256, createHash("sha256").update(payload).digest("hex"));
  assert.equal(captured.get(base + "/partial-index").sha256, undefined);
  assert.match(captured.get(base + "/partial-index").error, /ERR_ABORTED/);
  assert.equal(rows[2].evidence.responses[0].redirectedTo, urls[0]);
  assert.deepEqual(rows[12].evidence.responses, []);

  const originalManifest = join(output, "original-cells.json");
  await writeFile(originalManifest, JSON.stringify(rows));
  await run(process.execPath, [script, "--verify-captures", originalManifest]);
  revision = " revised";
  await run(process.execPath, [script, "--profile", PROFILE, "--out", output, "--widths", "desktop",
    "--states", "light", "--routes", "system", urls[0]]);
  const [revised] = JSON.parse(await readFile(join(output, "cells.json"), "utf8"));
  assert.notEqual(revised.capture, rows[0].capture);
  assert.notEqual(revised.evidence.documentSha256, rows[0].evidence.documentSha256);
  await run(process.execPath, [script, "--verify-captures", originalManifest]);
  await writeFile(rows[0].capture, "tampered");
  await assert.rejects(run(process.execPath, [script, "--verify-captures", originalManifest]),
    /capture bytes differ from the manifest/);
});
