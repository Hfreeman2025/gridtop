import * as pw from "playwright";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";
import { readdir, writeFile, mkdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
const engine = process.argv[2] || "chromium";
if (!["chromium", "webkit", "firefox"].includes(engine))
  throw Error("Unknown browser");
process.env.TMPDIR = resolve(".tmp");
await mkdir(process.env.TMPDIR, { recursive: true });
// Match actual browser engines, not unrelated background servers.
const ps = execFileSync("ps", ["-axo", "pid=,comm="], {
  encoding: "utf8",
}).split("\n");
const active = ps.filter((line) =>
  /ms-playwright\/|chrome-headless-shell|\.tmp\/browsers\/|MiniBrowser|firefox\/Contents\/MacOS\/firefox/.test(
    line,
  ),
);
if (active.length) {
  console.error(
    `Browser slot busy (${active.map((line) => line.trim().split(/\s+/)[0]).join(", ")}); no browser launched.`,
  );
  process.exit(75);
}
function silentAndMeasured() {
  const Offline = window.OfflineAudioContext;
  if (Offline) {
    class SilentContext extends Offline {
      constructor() {
        super(2, 128, 44100);
      }
      resume() {
        return Promise.resolve();
      }
      close() {
        return Promise.resolve();
      }
    }
    window.AudioContext = SilentContext;
    window.webkitAudioContext = SilentContext;
  }
  HTMLMediaElement.prototype.play = function () {
    return Promise.reject(new Error("Silent capture"));
  };
  window.captureCLS = {
    supported: PerformanceObserver.supportedEntryTypes.includes("layout-shift"),
    value: 0,
    entries: [],
  };
  if (window.captureCLS.supported)
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries())
        if (!entry.hadRecentInput) {
          window.captureCLS.value += entry.value;
          window.captureCLS.entries.push({
            value: entry.value,
            time: entry.startTime,
          });
        }
    }).observe({ type: "layout-shift", buffered: true });
}
const browser = await pw[engine].launch(
  engine === "chromium"
    ? { args: ["--mute-audio"] }
    : engine === "firefox"
      ? { firefoxUserPrefs: { "media.volume_scale": "0.0" } }
      : {},
);
const results = [];
try {
  const context = await browser.newContext({ deviceScaleFactor: 1 });
  await context.addInitScript(silentAndMeasured);
  const external = [];
  await context.route("**/*", (route) =>
    /^(file:|data:)/.test(route.request().url())
      ? route.continue()
      : (external.push(route.request().url()), route.abort()),
  );
  for (const gallery of ["runner", "map", "themes"]) {
    const dir = `.tmp/captures/${gallery}`;
    const names = (await readdir(dir))
      .filter((name) => name.endsWith(".html") && !name.startsWith("contact-"))
      .sort();
    const expected = { runner: 42, map: 30, themes: 9 }[gallery];
    if (names.length !== expected) {
      throw new Error(
        `Expected ${expected} ${gallery} frames; found ${names.length}. Generate the exports first.`,
      );
    }
    for (const name of names) {
      const phone = name.includes("phone"),
        compact = name.includes("compact");
      const viewport = {
        width: phone ? 390 : compact ? 800 : 1440,
        height: phone ? 844 : compact ? 500 : 900,
      };
      const page = await context.newPage();
      await page.setViewportSize(viewport);
      const errors = [];
      page.on("pageerror", (error) => errors.push(error.message));
      page.on("console", (message) => {
        if (["error", "warning"].includes(message.type()))
          errors.push(message.text());
      });
      await page.goto(pathToFileURL(resolve(`${dir}/${name}`)).href);
      await page.evaluate(async () => {
        await Promise.all([...document.fonts].map((face) => face.load()));
        await document.fonts.ready;
      });
      await page.waitForTimeout(120);
      const initial = await page.evaluate(() => ({
        ...window.captureCLS,
        scroll: {
          width: document.documentElement.scrollWidth,
          height: document.documentElement.scrollHeight,
        },
        families: [
          ...new Set(
            [...document.querySelectorAll("pre span")]
              .filter((e) => e.textContent.trim())
              .map((e) => getComputedStyle(e).fontFamily),
          ),
        ],
        faces: [...document.fonts].map((face) => ({
          family: face.family,
          weight: face.weight,
          status: face.status,
        })),
      }));
      if (
        initial.scroll.width > viewport.width ||
        initial.scroll.height > viewport.height
      )
        errors.push(`overflow ${JSON.stringify(initial.scroll)}`);
      if (initial.value !== 0) errors.push(`initial CLS ${initial.value}`);
      for (const family of [
        "IBM Plex Sans",
        "IBM Plex Mono",
        "IBM Plex Sans Condensed",
      ])
        if (
          !initial.faces.some(
            (face) =>
              face.family.replaceAll('\"', "") === family &&
              face.status === "loaded",
          )
        )
          errors.push(`${family} not loaded`);
      // Exercise a visible fallback, then swap to already-local Plex after 250ms.
      // This isolates geometry from network timing; all fonts remain self-contained.
      let swap = null;
      {
        const style = await page.addStyleTag({
          content:
            "pre span{font-family:monospace!important}pre span.reading,pre span.sign{font-family:sans-serif!important}",
        });
        await page.waitForTimeout(250);
        const before = await page.locator("pre span").evaluateAll((elements) =>
          elements.map((e) => {
            const r = e.getBoundingClientRect();
            return [r.x, r.y, r.width, r.height];
          }),
        );
        await style.evaluate((element) => element.remove());
        await page.waitForTimeout(120);
        const after = await page.locator("pre span").evaluateAll((elements) =>
          elements.map((e) => {
            const r = e.getBoundingClientRect();
            return [r.x, r.y, r.width, r.height];
          }),
        );
        swap = await page.evaluate(() => ({ ...window.captureCLS }));
        swap.geometryChanged = JSON.stringify(before) !== JSON.stringify(after);
        if (swap.value !== 0 || swap.geometryChanged)
          errors.push(`font swap moved layout: ${JSON.stringify(swap)}`);
      }
      await page.screenshot({
        path: `${dir}/${name.replace(".html", engine === "chromium" ? ".png" : `.${engine}.png`)}`,
      });
      results.push({
        gallery,
        name,
        engine,
        browserVersion: browser.version(),
        viewport,
        initial,
        swap,
        errors,
        silent: true,
        workers: 1,
        externalRequests: external.length,
      });
      await page.close();
    }
  }
  await context.close();
  if (external.length) throw Error("Unexpected external request");
} finally {
  await browser.close();
}
await writeFile(
  `.tmp/browser-report-${engine}.json`,
  JSON.stringify(results, null, 2) + "\n",
);
console.log(
  JSON.stringify({
    engine,
    states: results.length,
    failures: results
      .filter((r) => r.errors.length)
      .map((r) => ({ gallery: r.gallery, name: r.name, errors: r.errors })),
    clsSupported: results[0]?.initial.supported,
    silent: true,
    workers: 1,
  }),
);
if (results.some((r) => r.errors.length)) process.exitCode = 1;
