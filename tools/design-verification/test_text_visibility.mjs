import test from "node:test";
import assert from "node:assert/strict";
import { loadChromium, preparePage, captureMeasuredPage } from "./capture.mjs";

test("traversal reports text that never arrives and preserves intentional hidden states", async (t) => {
  const chromium = await loadChromium();
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
  await page.setContent(`<!doctype html><style>
    body { margin: 0; color: black; background: white }
    section { height: 650px }
    .absent { opacity: 0 }
    .screen-reader { position: absolute; width: 1px; height: 1px; clip-path: inset(50%) }
    .gradient { color: transparent; background: linear-gradient(red, blue); background-clip: text }
    #delayed { opacity: 0; transition: opacity .2s }
  </style><main>
    <section><p>Ordinary reading</p><p class="screen-reader">Accessible title</p>
      <p style="display:none">Closed panel</p><p style="visibility:hidden">Hidden panel</p>
      <p class="absent" aria-hidden="true">Inactive scene</p>
      <div inert><p class="absent">Inactive card</p></div>
      <p class="gradient">Gradient title</p>
      <p style="color:transparent;text-shadow:0 0 0 transparent, 1px 1px 0 red">Painted shadow</p>
      <div style="width:80px;height:50px;overflow:hidden"><span style="display:block;transform:translateX(150px);opacity:0">Clipped text</span></div><p id="delayed">Delayed entry</p>
      <div style="position:relative;height:30px"><p class="absent" style="margin:0">Covered inactive text</p>
        <div style="position:absolute;inset:0;background:white"></div></div></section>
    <section><p id="reversible" class="absent">Reversible entry</p></section>
    <section><div class="absent"><p id="broken-parent">Broken parent reveal</p></div>
      <p id="broken-fill" style="color:transparent;pointer-events:none">Broken foreground</p>
      <p id="broken-shadow" style="color:transparent;text-shadow:0 0 0 transparent">Transparent shadow</p>
      <p id="broken-webkit" style="-webkit-text-fill-color:transparent">Broken text fill</p></section>
  </main><script>
    setTimeout(() => document.querySelector('#delayed').style.opacity = 1, 250);
    new IntersectionObserver(entries => {
      for (const entry of entries) entry.target.style.opacity = entry.isIntersecting ? 1 : 0;
    }).observe(document.querySelector('#reversible'));
  </script>`);
  const result = await preparePage(page);
  assert.deepEqual(result.unrevealedText.sort(), [
    "p#broken-fill: Broken foreground",
    "p#broken-parent: Broken parent reveal",
    "p#broken-shadow: Transparent shadow",
    "p#broken-webkit: Broken text fill",
  ]);
  assert.equal(await page.evaluate(() => scrollY), 0);
  assert.equal(await page.locator('#reversible').evaluate(el => getComputedStyle(el).opacity), "0");
});


test("viewport capture preserves touch painting on a tall page and declares its limited image", async (t) => {
  const chromium = await loadChromium();
  const browser = await chromium.launch();
  t.after(() => browser.close());
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
  await page.setContent(`<!doctype html><meta name="viewport" content="width=device-width">
    <style>body{margin:0;background:blue;color:white} @media(pointer:coarse){body{background:red}}</style>
    <main style="height:3000px">Touch specimen</main>`);
  const traversal = await preparePage(page);
  const result = await captureMeasuredPage(page, { targetMin: 24, targetProject: 44 },
    { finalUrl: page.url() }, traversal, false);
  assert.equal(result.measurements.env.media.coarse, true);
  assert.equal(result.measurements.env.documentHeight, 3000);
  const painted = await page.evaluate(async (png) => {
    const image = new Image(); image.src = png; await image.decode();
    const canvas = document.createElement("canvas"); canvas.width = image.width; canvas.height = image.height;
    const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
    return { width: image.width, height: image.height, pixel: [...context.getImageData(0, 100, 1, 1).data],
      coarse: matchMedia("(pointer:coarse)").matches, touch: navigator.maxTouchPoints };
  }, "data:image/png;base64," + result.image.toString("base64"));
  assert.deepEqual(painted, { width: 390, height: 844, pixel: [255, 0, 0, 255], coarse: true, touch: 1 });
});
