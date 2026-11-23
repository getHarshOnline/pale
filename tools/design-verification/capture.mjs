#!/usr/bin/env node
/**
 * Capture rendered pages for the look pass: every test width, presentation state, colour-mode
 * route and context, with measurements saved beside each capture. Needs Node 18 or later and
 * Playwright with Chromium installed in the workspace (npm i -D playwright; npx playwright install
 * chromium). Run from the consuming workspace, or select an existing local module with
 * PLAYWRIGHT_MODULE. Run it; do not read it.
 *
 * Usage: capture.mjs [options] URL [URL ...]      (http(s) or file URLs of rendered pages)
 *        capture.mjs --verify-captures DIR/cells.json
 *        capture.mjs -h | --help
 * Options:
 *   --out DIR          where captures and cells.json go (default: ./captures)
 *   --json            write a machine-readable summary
 *   --profile FILE    JSON with widths, compactWidth, targetMin, targetProject and optional preferences
 *   --widths LIST      width names in the profile, comma-separated (default: all)
 *   --states LIST      light,light-reduced,dark,dark-reduced (default: all four)
 *   --routes LIST      system,explicit (default: system)
 *   --contexts LIST    web,mobile (default: web at every width, mobile below layout.test-width.tablet)
 *   --viewport         capture the initial viewport after traversal, not a full-page image;
 *                      records declare this scope separately
 *   --no-js            add scripts-off world-default cells in each selected context and OS preference;
 *                      their records do not claim a stored preference was restored
 *   --forced-colors    add a forced-colours cell the same way; with --no-js as well, also one cell
 *                      per state carrying both, which is a capability checked inside a state
 *   --scrub N          in every scripted cell, visit N evenly spaced scroll positions from above,
 *                      then from below; capture each settled viewport both ways
 *   --build ID         build or commit identifier written into every file name and record
 *   --record-responses record the same-origin HTTP GETs observed in each capture cell
 * Exit:  0 no failures · 1 failures · 2 skipped (Playwright or a browser missing, or no URL);
 *        a skip is never a pass.
 *
 * Pass resolved project values in the profile. An optional preference mapping
 * specifies which project storage keys and document attributes to exercise.
 *
 * Per cell it records the requested URL, final URL, navigation status and the environment
 * read from the page (viewport, devicePixelRatio, user agent,
 * matchMedia results, data-mode and data-motion) and fails a cell whose emulation did not apply:
 * verify the layout width and media features on every route, plus the restored attributes
 * on scripted routes.
 * Capture evidence includes hashes for the main response body and PNGs. The build label
 * remains caller-supplied. Offline verification checks the PNG bytes recorded in the manifest.
 * Native animation stays enabled. Measurements taken before and after a still must agree
 * within a bounded capture attempt. This is evidence for those measurements, not motion.
 * Measured, failing:
 *   overflow      root scrollWidth above clientWidth, zero tolerance
 *   escaping      an element's box past the viewport edge, unless inside a local scroll container
 *                 (overflow-x auto or scroll); a box clipped by overflow hidden still counts
 *   unrevealed-text  text encountered in the viewport that stayed fully transparent throughout
 *                 traversal, allowing up to one second at each visit for native arrival;
 *                 inert, aria-hidden and screen-reader-only content is excluded
 *   clipped-text  a text element whose content is wider than its box while overflow is hidden
 *   contrast      text colour against a solid composited background below 4.5:1 (3:1 for large
 *                 text); text over an image, gradient, translucent layer or filter is listed as
 *                 not measured, never passed
 *   target-spacing  an undersized control whose spacing circle overlaps another target or circle;
 *                   raw sizes, clear spacing and the separate project floor are reported
 *   scrub         (with --scrub) a requested offset the page did not reach, a settled frame that
 *                 differs by arrival direction, or a frame that does not settle
 *                 (cinematography the edit, "Position is the picture")
 * NOT CHECKED: focus order and visibility, dialogs, motion, meaning. Captures must still be read
 * at native scale (design-qa visual audit, "The look pass").
 */
import { createHash } from "node:crypto";
import { lstatSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const STATES = {
  "light": { colorScheme: "light", reducedMotion: "no-preference" },
  "light-reduced": { colorScheme: "light", reducedMotion: "reduce" },
  "dark": { colorScheme: "dark", reducedMotion: "no-preference" },
  "dark-reduced": { colorScheme: "dark", reducedMotion: "reduce" },
};

function usage(code) {
  const m = readFileSync(fileURLToPath(import.meta.url), "utf8").match(/\/\*\*([\s\S]*?)\*\//);
  console.log(m ? m[1].replace(/^ \* ?/gm, "").trim() : "");
  process.exit(code);
}

function skip(reason, json) {
  console.log(json ? JSON.stringify({ status: "skipped", manifest: null, cells: 0, problems: [reason] })
    : `SKIPPED: ${reason}`);
  process.exit(2);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function storeCapture(stem, image, evidence) {
  const hash = sha256(image);
  const path = `${stem}-${hash.slice(0, 16)}.png`;
  try {
    writeFileSync(path, image, { flag: "wx" });
  } catch (error) {
    if (error.code !== "EEXIST") throw error;
    if (!lstatSync(path).isFile() || sha256(readFileSync(path)) !== hash)
      throw new Error(`existing capture differs from its recorded identity: ${path}`);
  }
  evidence.images.push({ file: basename(path), sha256: hash });
  return path;
}

function verifyCaptures(manifest) {
  const rows = JSON.parse(readFileSync(manifest, "utf8"));
  if (!Array.isArray(rows)) throw new Error("capture manifest must contain cell records");
  let count = 0;
  for (const row of rows) {
    const expected = [row.capture, ...(row.scrub?.captures ?? [])].filter(Boolean).map((path) => basename(path));
    const images = row.evidence?.images ?? [];
    if (!Array.isArray(images) || images.length !== expected.length ||
        images.some((image, i) => image.file !== expected[i]))
      throw new Error(`capture inventory differs from the cell's paths: ${row.cell}`);
    if (images.length && !/^[0-9a-f]{64}$/.test(row.evidence.documentSha256 ?? ""))
      throw new Error(`missing main response identity for ${row.cell}`);
    for (const image of images) {
      const path = join(dirname(manifest), image.file);
      if (!lstatSync(path).isFile() || sha256(readFileSync(path)) !== image.sha256)
        throw new Error(`capture bytes differ from the manifest: ${image.file}`);
      count++;
    }
  }
  if (!count) throw new Error("capture manifest has no PNG evidence to verify");
  console.log(`${count} capture file(s) verified against ${manifest}`);
}

function readProfile(path) {
  if (!path) throw new Error("pass --profile with the viewport and target values");
  const profile = JSON.parse(readFileSync(path, "utf8"));
  const positive = (n) => Number.isSafeInteger(n) && n > 0;
  const widths = Object.entries(profile.widths ?? {});
  if (!widths.length || widths.some(([name, width]) => !/^[a-z][a-z0-9-]*$/.test(name) || !positive(width)))
    throw new Error("profile widths must map names to positive CSS pixel widths");
  for (const key of ["compactWidth", "targetMin", "targetProject"])
    if (!positive(profile[key])) throw new Error(`profile needs a positive ${key}`);
  const prefs = profile.preferences;
  if (prefs != null) {
    for (const key of ["colorKey", "motionKey", "modeAttribute", "motionAttribute", "system", "reduce", "allow"])
      if (typeof prefs[key] !== "string" || !prefs[key].trim())
        throw new Error(`preference mapping needs ${key}`);
  }
  return profile;
}

function parseArgs(argv) {
  const opts = { out: "captures", widths: null, states: Object.keys(STATES), routes: ["system"],
    contexts: null, viewport: false, noJs: false, forcedColors: false, scrub: 0, build: "unbuilt", urls: [] };
  const list = (value) => [...new Set(value.split(","))];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => { if (i + 1 >= argv.length) { console.log(`missing value for ${a}`); process.exit(1); } return argv[++i]; };
    if (a === "-h" || a === "--help") usage(0);
    else if (a === "--out") opts.out = next();
    else if (a === "--profile") opts.profile = next();
    else if (a === "--json") opts.json = true;
    else if (a === "--widths") opts.widths = list(next());
    else if (a === "--states") opts.states = list(next());
    else if (a === "--routes") opts.routes = list(next());
    else if (a === "--contexts") opts.contexts = list(next());
    else if (a === "--viewport") opts.viewport = true;
    else if (a === "--no-js") opts.noJs = true;
    else if (a === "--forced-colors") opts.forcedColors = true;
    else if (a === "--scrub") { opts.scrub = parseInt(next(), 10); if (!(opts.scrub >= 2)) { console.log("--scrub needs a count of 2 or more"); process.exit(1); } }
    else if (a === "--build") opts.build = next();
    else if (a === "--record-responses") opts.recordResponses = true;
    else if (a.startsWith("--")) { console.log(`unknown option ${a}`); process.exit(1); }
    else {
      let url;
      try { url = new URL(a); } catch {}
      if (!url || !["http:", "https:", "file:"].includes(url.protocol)) {
        console.log(`expected an HTTP(S) or file URL: ${a}`);
        process.exit(1);
      }
      opts.urls.push(url.href);
    }
  }
  for (const s of opts.states) if (!Object.hasOwn(STATES, s)) { console.log(`unknown state ${s}`); process.exit(1); }
  for (const route of opts.routes) if (!["system", "explicit"].includes(route)) { console.log(`unknown route ${route}`); process.exit(1); }
  for (const context of opts.contexts ?? []) if (!["web", "mobile"].includes(context)) { console.log(`unknown context ${context}`); process.exit(1); }
  opts.urls = [...new Set(opts.urls)];
  return opts;
}

async function observeResponses(page, requestedUrl) {
  const requests = new Map();
  const responses = new Map();
  const finished = new Set();
  const streamed = new URL(requestedUrl).origin === "null" ? new Map() : await streamFetchBodies(page, requestedUrl);
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (request.method() === "GET" && ["http:", "https:"].includes(url.protocol))
      requests.set(request, url);
  });
  page.on("response", (response) => responses.set(response.request(), response));
  page.on("requestfinished", (request) => finished.add(request));
  return (finalUrl) => Promise.all([...requests]
    .filter(([, url]) => url.origin === new URL(finalUrl).origin)
    .map(async ([request, url]) => {
      const response = responses.get(request);
      const status = response?.status() ?? null;
      const entry = { url: url.href, type: request.resourceType(), status };
      if (response?.fromServiceWorker()) return { ...entry, error: "service worker response" };
      if (status >= 300 && status < 400 && request.redirectedTo())
        return { ...entry, redirectedTo: request.redirectedTo().url() };
      if (status !== 200) return { ...entry, error: request.failure()?.errorText ?? "no full HTTP 200 body" };
      if (!finished.has(request)) {
        const candidates = streamed.get(url.href);
        // A consumer can abort after reading the entire body. Only use a complete,
        // unambiguous network stream, retaining the actual browser failure beside it.
        if (request.failure()?.errorText === "net::ERR_ABORTED" && candidates?.length === 1 &&
            [...requests.values()].filter((observed) => observed.href === url.href).length === 1) {
          const hash = await candidates[0].result;
          if (hash) return { ...entry, sha256: hash, transport: "complete-stream-before-abort" };
        }
        return { ...entry, error: request.failure()?.errorText ?? "request did not finish during capture" };
      }
      try {
        return { ...entry, sha256: sha256(await response.body()) };
      } catch {
        return { ...entry, error: "browser could not read the response body" };
      }
    }));
}

async function streamFetchBodies(page, requestedUrl) {
  const client = await page.context().newCDPSession(page);
  try { await client.send("Network.enable"); }
  catch { await client.detach(); return new Map(); }
  const origin = new URL(requestedUrl).origin;
  const gets = new Set();
  const streams = new Map();
  const byUrl = new Map();
  client.on("Network.requestWillBeSent", ({ requestId, request }) => {
    if (request.method === "GET" && new URL(request.url).origin === origin) gets.add(requestId);
    else gets.delete(requestId);
  });
  client.on("Network.responseReceived", ({ requestId, type, response }) => {
    if (!gets.has(requestId) || !["Fetch", "XHR"].includes(type) || response.status !== 200 ||
        response.fromServiceWorker || response.fromDiskCache) return;
    const headers = Object.fromEntries(Object.entries(response.headers).map(([k, v]) => [k.toLowerCase(), v]));
    const length = Number(headers["content-length"]);
    if (!Number.isSafeInteger(length) || length < 1 || length > 64 * 1024 * 1024 ||
        (headers["content-encoding"] && headers["content-encoding"] !== "identity")) return;
    const stream = { length, size: 0, chunks: [] };
    stream.start = client.send("Network.streamResourceContent", { requestId })
      .then(({ bufferedData }) => Buffer.from(bufferedData, "base64"), () => null);
    streams.set(requestId, stream);
    byUrl.set(response.url, [...(byUrl.get(response.url) ?? []), stream]);
  });
  client.on("Network.dataReceived", ({ requestId, data }) => {
    const stream = streams.get(requestId);
    if (!stream || !data) return;
    const chunk = Buffer.from(data, "base64");
    stream.size += chunk.length;
    if (stream.size <= stream.length) stream.chunks.push(chunk);
    else stream.chunks.length = 0;
  });
  const seal = ({ requestId }) => {
    const stream = streams.get(requestId);
    if (!stream) return;
    streams.delete(requestId);
    stream.result = stream.start.then((first) => {
      const chunks = stream.chunks.splice(0);
      if (!first || first.length + stream.size !== stream.length) return null;
      const hash = createHash("sha256").update(first);
      for (const chunk of chunks) hash.update(chunk);
      return hash.digest("hex");
    });
  };
  client.on("Network.loadingFinished", seal);
  client.on("Network.loadingFailed", seal);
  return byUrl;
}

// A project can keep an explicit System choice in its own storage.
export async function installPreferenceRoute(context, route, wanted, preferences) {
  const mode = route === "system" ? preferences.system : wanted.colorScheme;
  const motion = route === "system" ? null
    : wanted.reducedMotion === "reduce" ? preferences.reduce : preferences.allow;
  await context.addInitScript(([colorKey, motionKey, colour, movement]) => {
    try {
      localStorage.setItem(colorKey, colour);
      if (movement === null) localStorage.removeItem(motionKey);
      else localStorage.setItem(motionKey, movement);
    } catch {}
  }, [preferences.colorKey, preferences.motionKey, mode, motion]);
}

// Runs in the page. Returns the environment and the measurements for one cell.
function measure({ targetMin, targetProject, preferences }) {
  const root = document.documentElement;
  const vw = root.clientWidth;
  const canvas = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
  const rgba = (css) => {
    canvas.clearRect(0, 0, 1, 1);
    canvas.fillStyle = "rgba(0,0,0,0)";
    canvas.fillStyle = css;
    canvas.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = canvas.getImageData(0, 0, 1, 1).data;
    return [r, g, b, a / 255];
  };
  const lum = ([r, g, b]) => {
    const c = [r, g, b].map((v) => { v /= 255; return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; });
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  };
  const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  const path = (el) => {
    const parts = [];
    for (let n = el; n && n.nodeType === 1 && parts.length < 4; n = n.parentElement) {
      let s = n.localName;
      if (n.id) { parts.unshift(`${s}#${n.id}`); break; }
      if (n.classList.length) s += "." + [...n.classList].slice(0, 2).join(".");
      parts.unshift(s);
    }
    return parts.join(" > ");
  };
  const visible = (el) => {
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    const visuallyHidden = (cs.position === "absolute" || cs.position === "fixed") &&
      el.offsetWidth <= 1 && el.offsetHeight <= 1 &&
      (cs.clipPath === "inset(50%)" || cs.clip === "rect(0px, 0px, 0px, 0px)");
    return !visuallyHidden && cs.visibility !== "hidden" && cs.display !== "none" &&
      r.width > 0 && r.height > 0 && (el.checkVisibility?.() ?? true);
  };
  const inScroller = (el) => {
    for (let n = el.parentElement; n && n !== root; n = n.parentElement) {
      const ox = getComputedStyle(n).overflowX;
      if (ox === "auto" || ox === "scroll") return true; // a declared local scroll container
    }
    return false;
  };
  const all = [...document.body.querySelectorAll("*")].filter(visible);

  const escaping = all.filter((el) => {
    const r = el.getBoundingClientRect();
    return (r.right > vw || r.left < 0) && !inScroller(el);
  }).map(path);

  const textEls = all.filter((el) => [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()));
  const clipped = textEls.filter((el) => {
    const cs = getComputedStyle(el);
    return (cs.overflowX === "hidden" || cs.overflowX === "clip") && el.scrollWidth > el.clientWidth;
  }).map(path);

  const contrast = { failed: [], notMeasured: [], measured: 0 };
  for (const el of textEls) {
    const cs = getComputedStyle(el);
    const fg = rgba(cs.color);
    let bg = null, reason = null;
    for (let n = el; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.backgroundImage !== "none") { reason = "background image or gradient"; break; }
      if (parseFloat(s.opacity) < 1) { reason = "translucent layer"; break; }
      if (s.filter !== "none" || s.backdropFilter && s.backdropFilter !== "none" || s.mixBlendMode !== "normal") { reason = "filter or blend"; break; }
      const c = rgba(s.backgroundColor);
      if (c[3] === 1) { bg = c; break; }
      if (c[3] > 0) { reason = "translucent background"; break; }
    }
    if (!bg && !reason) bg = rgba(getComputedStyle(root).backgroundColor)[3] === 1 ? rgba(getComputedStyle(root).backgroundColor) : null;
    if (!bg) { contrast.notMeasured.push(`${path(el)} (${reason || "no solid background"})`); continue; }
    const shown = [0, 1, 2].map((i) => fg[i] * fg[3] + bg[i] * (1 - fg[3]));
    const size = parseFloat(cs.fontSize);
    const large = size >= 24 || (size >= 18.66 && parseInt(cs.fontWeight, 10) >= 700);
    const need = large ? 3 : 4.5;
    const got = ratio(shown, bg);
    contrast.measured++;
    if (got < need) contrast.failed.push(`${path(el)} ${got.toFixed(2)}:1 < ${need}:1`);
  }

  const controls = all.filter((el) => el.matches("a[href], button, input:not([type=hidden]), select, textarea, summary, [role=button], [role=link]") &&
    !el.matches(":disabled") && !el.closest("[inert]") && getComputedStyle(el).pointerEvents !== "none");
  const projected = (el) => {
    for (let ancestor = el; ancestor; ancestor = ancestor.parentElement) {
      const style = getComputedStyle(ancestor);
      if (style.rotate && style.rotate !== "none") return true;
      if (style.translate?.split(/\s+/).length > 2) return true;
      if (style.transform === "none") continue;
      const matrix = new DOMMatrixReadOnly(style.transform);
      if (!matrix.is2D || matrix.b !== 0 || matrix.c !== 0) return true;
    }
    return false;
  };
  const boxes = [];
  for (const el of controls) {
    const r = el.getBoundingClientRect();
    const inline = el.localName === "a" && getComputedStyle(el).display === "inline" && el.parentElement &&
      [...el.parentElement.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    boxes.push({
      label: `${path(el)} ${Math.round(r.width)}x${Math.round(r.height)}`,
      rects: [...el.getClientRects()].filter((rect) => rect.width > 0 && rect.height > 0),
      x: r.left + r.width / 2,
      y: r.top + r.height / 2,
      undersized: r.width < targetMin || r.height < targetMin,
      belowProject: r.width < targetProject || r.height < targetProject,
      inline,
      projected: projected(el),
    });
  }
  const targets = { small: [], belowProject: [], spaced: [], crowded: [], manual: [] };
  const radius = targetMin / 2;
  const intersectsRect = (box, r) => {
    const dx = Math.max(r.left - box.x, 0, box.x - r.right);
    const dy = Math.max(r.top - box.y, 0, box.y - r.bottom);
    return Math.hypot(dx, dy) < radius;
  };
  for (const box of boxes) {
    if (box.inline) continue;
    if (box.belowProject) targets.belowProject.push(box.label);
    if (box.undersized) targets.small.push(box.label);
    if (box.rects.length !== 1 || box.projected) {
      targets.manual.push(`${box.label} (${box.projected ? "projected" : "fragmented"} target geometry)`);
      continue;
    }
    if (!box.undersized) continue;
    const neighbors = boxes.filter((other) => other !== box &&
      (other.rects.some((rect) => intersectsRect(box, rect)) ||
       (!other.inline && other.undersized && other.rects.length === 1 &&
        Math.hypot(box.x - other.x, box.y - other.y) < targetMin)));
    const neighbor = neighbors.find((other) => !other.projected && (other.inline || other.rects.length === 1));
    if (neighbor) targets.crowded.push(`${box.label} near ${neighbor.label}`);
    else if (neighbors.length) targets.manual.push(`${box.label} (near ${neighbors[0].label} with uncertain geometry)`);
    else targets.spaced.push(box.label);
  }

  return {
    env: {
      url: location.href, viewport: [innerWidth, innerHeight], devicePixelRatio,
      scrollY, documentHeight: document.scrollingElement.scrollHeight,
      userAgent: navigator.userAgent,
      media: {
        dark: matchMedia("(prefers-color-scheme: dark)").matches,
        reduce: matchMedia("(prefers-reduced-motion: reduce)").matches,
        coarse: matchMedia("(pointer: coarse)").matches,
        forced: matchMedia("(forced-colors: active)").matches,
      },
      dataMode: preferences ? root.getAttribute(preferences.modeAttribute) : null,
      dataMotion: preferences ? root.getAttribute(preferences.motionAttribute) : null,
      fonts: [...document.fonts].filter((f) => f.status === "loaded").map((f) => `${f.family} ${f.weight} ${f.style}`),
    },
    overflow: { scrollWidth: root.scrollWidth, clientWidth: root.clientWidth },
    escaping, clipped, contrast, targets,
  };
}

// Track actual text nodes while their rectangles pass through the viewport. A
// reverse reveal may hide them again later; seeing them once still establishes arrival.
function observeTextVisibility(observed) {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  const canvas = document.createElement("canvas").getContext("2d");
  const alpha = (color) => {
    canvas.clearRect(0, 0, 1, 1);
    canvas.fillStyle = color;
    canvas.fillRect(0, 0, 1, 1);
    return canvas.getImageData(0, 0, 1, 1).data[3];
  };
  let waiting = false;
  for (let node; (node = walker.nextNode());) {
    const text = node.textContent.trim();
    const el = node.parentElement;
    if (!text || !el || el.closest('script, style, template, [inert], [aria-hidden="true"]') ||
        !el.checkVisibility({ visibilityProperty: true })) continue;
    const cs = getComputedStyle(el);
    if ((cs.position === "absolute" || cs.position === "fixed") &&
        el.offsetWidth <= 1 && el.offsetHeight <= 1 &&
        (cs.clipPath === "inset(50%)" || cs.clip === "rect(0px, 0px, 0px, 0px)")) continue;
    const range = document.createRange();
    range.selectNode(node);
    if (![...range.getClientRects()].some((r) => {
      let left = Math.max(0, r.left), right = Math.min(innerWidth, r.right);
      let top = Math.max(0, r.top), bottom = Math.min(innerHeight, r.bottom);
      for (let ancestor = el; ancestor; ancestor = ancestor.parentElement) {
        const style = getComputedStyle(ancestor);
        if (style.display === "inline") continue;
        const bounds = ancestor.getBoundingClientRect();
        const sx = ancestor.offsetWidth ? bounds.width / ancestor.offsetWidth : 1;
        const sy = ancestor.offsetHeight ? bounds.height / ancestor.offsetHeight : 1;
        if (style.overflowX !== "visible") {
          const edge = bounds.left + ancestor.clientLeft * sx;
          left = Math.max(left, edge); right = Math.min(right, edge + ancestor.clientWidth * sx);
        }
        if (style.overflowY !== "visible") {
          const edge = bounds.top + ancestor.clientTop * sy;
          top = Math.max(top, edge); bottom = Math.min(bottom, edge + ancestor.clientHeight * sy);
        }
      }
      if (left >= right || top >= bottom) return false;
      // A covered card or clipped local scroller is not a text arrival observed
      // by vertical traversal. Opacity-zero text itself still receives hit tests.
      const hit = document.elementFromPoint((left + right) / 2, (top + bottom) / 2);
      return hit && (el.contains(hit) || hit.contains(el));
    })) continue;
    let opacity = 1, paintedBackground = false;
    for (let ancestor = el; ancestor; ancestor = ancestor.parentElement) {
      const style = getComputedStyle(ancestor);
      opacity *= Number(style.opacity);
      if (style.backgroundClip === "text" &&
          (style.backgroundImage !== "none" || alpha(style.backgroundColor))) paintedBackground = true;
    }
    // Computed shadows serialize their colours before the lengths. A transparent
    // shadow does not rescue transparent text; parse only actual CSS colour tokens.
    const shadowColours = cs.textShadow.match(/(?:[a-z-]+\([^()]*\)|#[\da-f]+|[a-z]+)/gi) ?? [];
    const paintedShadow = shadowColours.some((colour) => CSS.supports("color", colour) && alpha(colour) > 0);
    const visible = opacity > 0 && (alpha(cs.webkitTextFillColor || cs.color) > 0 ||
      paintedBackground || paintedShadow ||
      (parseFloat(cs.webkitTextStrokeWidth) > 0 && alpha(cs.webkitTextStrokeColor) > 0));
    const prior = observed.get(node);
    const seen = visible || (prior?.text === text && prior.seen);
    observed.set(node, { seen, text,
      element: el.id ? `${el.localName}#${el.id}` : el.localName });
    waiting ||= !seen;
  }
  return waiting;
}

export async function preparePage(page) {
  const observed = await page.evaluateHandle(() => new Map());
  const geometry = () => page.evaluate(() => ({
    top: scrollY,
    viewport: innerHeight,
    height: document.scrollingElement.scrollHeight,
  }));
  const visit = async (target) => {
    await page.evaluate((v) => window.scrollTo({ top: v, behavior: "instant" }), target);
    await page.waitForTimeout(50);
    // Let native reveals advance, without forcing their end state or stopping loops.
    for (let attempt = 0; attempt < 11; attempt++) {
      if (!await observed.evaluate(observeTextVisibility) || attempt === 10) break;
      await page.waitForTimeout(100);
    }
    return geometry();
  };
  const waitForMediaOrGrowth = (edge) => page.waitForFunction((visitedEdge) =>
    document.scrollingElement.scrollHeight > visitedEdge + 1 ||
    (document.fonts.status === "loaded" && [...document.images].every((img) => img.complete)),
  edge, { timeout: 10000 });
  // Allow one CSS pixel for rounding at the document and viewport edges.
  let covered = 0;
  let target = 0;
  try {
    for (let step = 1; step <= 200; step++) {
      const { top, viewport, height } = await visit(target);
      if (top > covered + 1)
        throw new Error(`full-page traversal left a gap from ${covered}px to ${top}px; inspect reachable holds directly`);
      covered = Math.max(covered, top + viewport);
      let documentHeight = height;
      if (top + viewport >= height - 1) {
        // A new lazy region may extend the document while the existing images settle.
        await waitForMediaOrGrowth(covered);
        documentHeight = (await geometry()).height;
        if (documentHeight <= covered + 1) {
          await visit(0);
          await waitForMediaOrGrowth(covered);
          const start = await geometry();
          if (start.top !== 0)
            throw new Error(`full-page traversal could not return to the top (landed at ${start.top}px)`);
          if (start.height <= covered + 1) {
            const failedImages = await page.evaluate(() => [...document.images].filter((img) => !img.naturalWidth)
              .map((img) => img.currentSrc || img.src));
            const unrevealedText = await observed.evaluate((entries) => [...entries]
              .filter(([node, value]) => node.isConnected && !value.seen)
              .map(([, value]) => `${value.element}: ${value.text.replace(/\s+/g, " ").slice(0, 80)}`));
            return { failedImages, unrevealedText, documentHeight: start.height, steps: step };
          }
          // A change on returning to the top can invalidate the earlier viewport visits.
          target = 0;
          covered = 0;
          continue;
        }
      }
      target = Math.min(documentHeight - viewport, Math.max(target, top) + Math.max(1, Math.floor(viewport * 0.8)));
    }
    throw new Error("full-page traversal exceeded 200 steps; scope the page explicitly");
  } finally {
    await observed.dispose();
  }
}

export async function captureMeasuredPage(page, thresholds, navigation, traversal, fullPage = true) {
  for (let attempt = 1; attempt <= 4; attempt++) {
    const before = await page.evaluate(measure, thresholds);
    const image = await page.screenshot({ fullPage, caret: "hide" });
    const after = await page.evaluate(measure, thresholds);
    const observedUrl = [before.env.url, after.env.url, page.url()]
      .find((url) => url !== navigation.finalUrl);
    if (observedUrl) {
      navigation.observedUrl = observedUrl;
      throw new Error(`page URL changed during capture preparation; recapture the observed URL: ${observedUrl}`);
    }
    if (JSON.stringify(before) === JSON.stringify(after)) {
      if (traversal && (after.env.scrollY !== 0 || after.env.documentHeight !== traversal.documentHeight))
        throw new Error("page height or top position changed after traversal; repeat the look pass");
      return { image, measurements: after, attempts: attempt };
    }
    if (attempt < 4) await page.waitForTimeout(150);
  }
  throw new Error("measurements changed across all four capture attempts; inspect the live state separately");
}

// Keep an offset with each frame. Equal pixels can recur at different offsets.
async function settled(page) {
  const sample = async () => {
    const before = await page.evaluate(() => scrollY);
    const image = await page.screenshot({ caret: "hide" });
    const offset = await page.evaluate(() => scrollY);
    return { offset, image: before === offset ? image : null };
  };
  let last = await sample();
  for (let waited = 0; waited < 4000; waited += 200) {
    await page.waitForTimeout(200);
    const now = await sample();
    if (now.offset === last.offset && now.image && last.image && now.image.equals(last.image)) return now;
    last = now;
  }
  return { offset: last.offset, image: null };
}

// Visits evenly spaced positions from above, then from below, and compares the settled frames.
async function scrub(page, count, base, record, assertDocument) {
  const max = await page.evaluate(() => Math.max(0, document.documentElement.scrollHeight - innerHeight));
  if (!max) { record.scrub = { positions: [], note: "the page does not scroll" }; return; }
  const ys = Array.from({ length: count }, (_, i) => Math.round((max * i) / (count - 1)));
  const down = [], up = [];
  for (const y of ys) { await page.evaluate((v) => window.scrollTo(0, v), y); down.push(await settled(page)); }
  for (const y of [...ys].reverse()) { await page.evaluate((v) => window.scrollTo(0, v), y); up.unshift(await settled(page)); }
  assertDocument();
  record.scrub = { positions: ys, captures: [], visits: [] };
  ys.forEach((y, i) => {
    record.scrub.visits.push({ requested: y, down: down[i].offset, up: up[i].offset });
    for (const [dir, { image, offset }] of [["down", down[i]], ["up", up[i]]]) {
      if (!image) { record.problems.push(`scrub: the frame requested at ${y}px going ${dir} never settles (last offset ${offset}px)`); continue; }
      if (offset !== y) { record.problems.push(`scrub: requested ${y}px going ${dir}, settled at ${offset}px; use a reachable hold`); continue; }
      const stem = `${base}_scrub-${String(i).padStart(2, "0")}-${dir}`;
      record.scrub.captures.push(storeCapture(stem, image, record.evidence));
    }
    if (down[i].offset === y && up[i].offset === y && down[i].image && up[i].image && !down[i].image.equals(up[i].image))
      record.problems.push(`scrub: the frame at ${y}px differs when reached from above and from below`);
  });
}

function slug(url) {
  const u = new URL(url);
  const parts = u.pathname.replace(/\/index\.html?$/, "/").split("/").filter(Boolean);
  const tail = (u.protocol === "file:" ? parts.slice(-2) : [u.host, ...parts]).join("-");
  const label = tail.replace(/\.html?$/, "").replace(/[^A-Za-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(-60) || "home";
  const identity = sha256(u.href).slice(0, 16);
  return `${label}-${identity}`;
}

export async function loadChromium() {
  if (process.env.PLAYWRIGHT_MODULE) {
    const moduleUrl = pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE));
    return (await import(moduleUrl.href)).chromium;
  }
  const requireWorkspace = createRequire(join(process.cwd(), "package.json"));
  let modulePath;
  try { modulePath = requireWorkspace.resolve("playwright"); }
  catch (error) {
    if (error.code !== "MODULE_NOT_FOUND") throw error;
    return (await import("playwright")).chromium;
  }
  return requireWorkspace(modulePath).chromium;
}

async function main() {
  const argv = process.argv.slice(2);
  if (argv[0] === "--verify-captures") {
    try {
      if (argv.length !== 2) throw new Error("expected one capture manifest path");
      verifyCaptures(argv[1]);
    } catch (error) {
      console.error(`FAIL: ${error.message}`);
      process.exitCode = 1;
    }
    return;
  }
  const opts = parseArgs(argv);
  if (!opts.urls.length) skip("no URL given", opts.json);
  const { widths, compactWidth, preferences, targetMin, targetProject } = readProfile(opts.profile);
  const chosen = opts.widths ?? Object.keys(widths);
  for (const width of chosen)
    if (!Object.hasOwn(widths, width)) throw new Error(`unknown width ${width}`);
  if (opts.routes.includes("explicit") && !preferences)
    throw new Error("explicit route needs the project's preference mapping");
  const thresholds = { targetMin, targetProject, preferences };
  let chromium;
  try { chromium = await loadChromium(); }
  catch (error) { skip(`Playwright unavailable from the selected workspace or module: ${error.message.split("\n")[0]}`, opts.json); }
  let browser;
  try { browser = await chromium.launch(); } catch (e) { skip(`no browser for Playwright (npx playwright install chromium): ${e.message.split("\n")[0]}`, opts.json); }

  mkdirSync(opts.out, { recursive: true });
  const platform = `${process.platform}-chromium-${browser.version()}`;
  const buildLabel = opts.build.replace(/[^A-Za-z0-9._-]/g, "-").slice(0, 48);
  const buildName = buildLabel === opts.build ? buildLabel : `${buildLabel}-${sha256(opts.build).slice(0, 12)}`;
  const records = [];
  let failures = 0;

  const cells = [];
  for (const url of opts.urls) for (const w of chosen) {
    const contexts = opts.contexts ?? (widths[w] < compactWidth ? ["web", "mobile"] : ["web"]);
    for (const ctx of contexts) for (const state of opts.states) {
      for (const route of opts.routes) cells.push({ url, w, ctx, state, route, js: true });
      // Scripts-off cells observe the world default under the requested OS conditions.
      if (opts.noJs) cells.push({ url, w, ctx, state, route: "default", js: false });
      if (opts.forcedColors) cells.push({ url, w, ctx, state, route: "system", js: true, forced: true });
      if (opts.noJs && opts.forcedColors)
        cells.push({ url, w, ctx, state, route: "default", js: false, forced: true });
    }
  }

  for (const c of cells) {
    const want = STATES[c.state];
    // Explicit route: the system says the opposite, the stored choice says the state.
    const system = c.route === "explicit"
      ? { colorScheme: want.colorScheme === "dark" ? "light" : "dark", reducedMotion: want.reducedMotion === "reduce" ? "no-preference" : "reduce" }
      : want;
    const context = await browser.newContext({
      viewport: { width: widths[c.w], height: Math.round(widths[c.w] * (c.ctx === "mobile" ? 2.16 : 0.625)) },
      hasTouch: c.ctx === "mobile", isMobile: c.ctx === "mobile", javaScriptEnabled: c.js,
      colorScheme: system.colorScheme, reducedMotion: system.reducedMotion,
      forcedColors: c.forced ? "active" : "none",
    });
    if (c.js && preferences) await installPreferenceRoute(context, c.route, want, preferences);
    const page = await context.newPage();
    const collectResponses = opts.recordResponses ? await observeResponses(page, c.url) : null;
    let navigations = 0;
    page.on("framenavigated", (frame) => { if (frame === page.mainFrame()) navigations++; });
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    const name = [slug(c.url), c.w, c.state, c.route, c.ctx, c.js ? "js" : "nojs",
      ...(c.forced ? ["forced"] : []), platform, buildName].join("_");
    const record = { cell: name, ...c, width: widths[c.w], platform, build: opts.build,
      preferenceSource: c.js ? (preferences ? c.route : "emulated-os") : "page-default-under-emulated-os",
      captureScope: opts.viewport ? "viewport" : "full-page", problems: [] };
    try {
      const response = await page.goto(c.url, { waitUntil: "load" });
      const loadedNavigation = navigations;
      const finalUrl = page.url();
      const status = response?.status() ?? null;
      record.navigation = { finalUrl, status };
      if (!response || status < 200 || status >= 300)
        throw new Error(`navigation did not return a successful page (${status ?? "no response"}): ${finalUrl}`);
      const assertDocument = () => {
        if (page.url() !== finalUrl || navigations !== loadedNavigation)
          throw new Error(`main document changed during capture: ${page.url()}`);
      };
      record.evidence = { documentSha256: sha256(await response.body()), images: [] };
      await page.waitForLoadState("networkidle");
      const { failedImages, unrevealedText, ...traversal } = await preparePage(page);
      const { image, measurements: m, attempts } = await captureMeasuredPage(page, thresholds, record.navigation, traversal, !opts.viewport);
      assertDocument();
      record.traversal = traversal;
      record.measurementAttempts = attempts;
      Object.assign(record, m, { pageErrors: errors });
      // The emulation must have applied, on the route under test.
      if (m.env.viewport[0] !== widths[c.w])
        record.problems.push(`emulation: layout viewport width ${m.env.viewport[0]} differs from requested ${widths[c.w]}`);
      if (m.env.media.dark !== (system.colorScheme === "dark") || m.env.media.reduce !== (system.reducedMotion === "reduce"))
        record.problems.push("emulation: the page does not report the requested media features");
      if (c.route === "system" && c.js && preferences && (m.env.dataMode !== preferences.system || m.env.dataMotion !== null))
        record.problems.push("preference: System colour choice or inherited motion preference was not restored by the page");
      const motion = want.reducedMotion === "reduce" ? preferences?.reduce : preferences?.allow;
      if (c.route === "explicit" && c.js && m.env.dataMode !== want.colorScheme)
        record.problems.push(`emulation: data-mode is ${m.env.dataMode} after storing the color choice ${want.colorScheme} (restore script missing or storage key differs)`);
      if (c.route === "explicit" && c.js && m.env.dataMotion !== motion)
        record.problems.push(`emulation: data-motion is ${m.env.dataMotion} after storing the motion choice ${motion} (restore script missing or storage key differs)`);
      if (m.env.media.coarse !== (c.ctx === "mobile")) record.problems.push("emulation: pointer does not match the context");
      if (m.env.media.forced !== Boolean(c.forced))
        record.problems.push(`emulation: forced-colors is ${m.env.media.forced ? "active" : "none"} in a cell asking for ${c.forced ? "active" : "none"}`);
      if (!record.problems.length) {
        record.capture = storeCapture(join(opts.out, name), image, record.evidence);
        if (opts.scrub && c.js) await scrub(page, opts.scrub, join(opts.out, name), record, assertDocument);
      }
      if (collectResponses) record.evidence.responses = await collectResponses(finalUrl);
      record.unrevealedText = unrevealedText;
      for (const text of unrevealedText) record.problems.push(`unrevealed-text: ${text}`);
      for (const failedImage of failedImages) record.problems.push(`image: failed to load ${failedImage}`);
      if (m.overflow.scrollWidth > m.overflow.clientWidth)
        record.problems.push(`overflow: root scrollWidth ${m.overflow.scrollWidth} > clientWidth ${m.overflow.clientWidth}`);
      for (const e of m.escaping.slice(0, 10)) record.problems.push(`escaping: ${e}`);
      for (const e of m.clipped.slice(0, 10)) record.problems.push(`clipped-text: ${e}`);
      for (const e of m.contrast.failed.slice(0, 10)) record.problems.push(`contrast: ${e}`);
      for (const e of m.targets.crowded.slice(0, 10)) record.problems.push(`target-spacing: ${e}`);
      for (const e of errors) record.problems.push(`page-error: ${e}`);
    } catch (e) {
      record.problems.push(`capture: ${e.message.split("\n")[0]}`);
    }
    await context.close();
    records.push(record);
    failures += record.problems.length;
    const notMeasured = record.contrast ? record.contrast.notMeasured.length : 0;
    const manualTargets = record.targets ? record.targets.manual.length : 0;
    if (!opts.json) {
      console.log(`${record.problems.length ? "FAIL" : "ok  "} ${name}` +
        (notMeasured ? ` (contrast not measured on ${notMeasured} element(s))` : "") +
        (manualTargets ? ` (manual target geometry: ${manualTargets} element(s))` : ""));
      for (const problem of record.problems) console.log(`     ${problem}`);
    }
  }
  await browser.close();
  const manifest = join(opts.out, "cells.json");
  writeFileSync(manifest, JSON.stringify(records, null, 1) + "\n");
  if (opts.json) {
    const problems = records.flatMap((row) => row.problems.map((message) => ({ cell: row.cell, message })));
    console.log(JSON.stringify({ status: failures ? "failed" : "passed", manifest, cells: records.length, problems }));
  } else {
    console.log(`${records.length} cell(s) captured on ${platform}, ${failures} failure(s); records: ${manifest}`);
    console.log("NOT CHECKED: focus order and visibility, dialogs, motion, meaning; read every capture at native scale");
  }
  process.exit(failures ? 1 : 0);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href)
  main().catch((error) => { console.error(`FAIL: ${error.message}`); process.exitCode = 1; });
