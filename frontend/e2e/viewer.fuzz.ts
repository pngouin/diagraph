import { test, type Browser } from "@playwright/test";
import fc from "fast-check";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { actionArb, perform, type Action } from "./actions";
import { pageInvariants } from "./invariants";
import { renderHtml, writeTree } from "./manifest";
import { modelArb, type Model, type ViewportSize } from "./model";
import { regressions } from "./regressions";

const FAILURES_DIR = fileURLToPath(new URL("../fuzz-failures", import.meta.url));

// Animated zoom matters: the first dblclick-focus bug only showed on the
// transition's early frames, which reduced motion skips entirely.
const viewportArb: fc.Arbitrary<ViewportSize> = fc
  .tuple(fc.constantFrom([1400, 900], [1024, 768], [390, 844]), fc.boolean())
  .map(([[width, height], animate]) => ({ width: width!, height: height!, animate }));
const FIT_TRANSITION_MS = 650;

async function nextFrame(page: import("@playwright/test").Page) {
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
}

/** Returns every invariant violation; with `artifactsDir`, also keeps the tree,
 * HTML, trace and final screenshot there instead of a throwaway temp dir. */
async function runCase(
  browser: Browser,
  model: Model,
  viewport: ViewportSize,
  actions: Action[],
  artifactsDir: string | null
): Promise<string[]> {
  const work = artifactsDir ?? mkdtempSync(join(tmpdir(), "diagraph-fuzz-"));
  const failures: string[] = [];
  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height },
    reducedMotion: viewport.animate ? "no-preference" : "reduce",
  });
  try {
    const tree = join(work, "tree");
    const html = join(work, "diagraph.html");
    writeTree(model, tree);
    renderHtml(tree, html);

    if (artifactsDir) await context.tracing.start({ screenshots: true, snapshots: true });
    const page = await context.newPage();
    page.on("console", (m) => {
      if (m.type() === "error") failures.push(`rule5: console error: ${m.text()}`);
    });
    page.on("pageerror", (e) => failures.push(`rule5: uncaught ${e.name}: ${e.message}`));
    await page.goto(pathToFileURL(html).href);
    await nextFrame(page);

    const initial = await page.evaluate(pageInvariants, { focus: null, initial: true });
    failures.push(...initial.map((f) => `[initial] ${f}`));

    for (const [i, action] of actions.entries()) {
      if (failures.length > 0) break;
      const focus = await perform(page, action);
      if (viewport.animate && action.kind === "dblclick") await page.waitForTimeout(FIT_TRANSITION_MS + 100);
      await nextFrame(page);
      const after = await page.evaluate(pageInvariants, { focus, initial: false });
      failures.push(...after.map((f) => `[after action ${i} ${JSON.stringify(action)}] ${f}`));
    }

    if (artifactsDir) {
      await page.screenshot({ path: join(artifactsDir, "screenshot.png") });
      await context.tracing.stop({ path: join(artifactsDir, "trace.zip") });
    }
  } finally {
    await context.close();
    if (!artifactsDir) rmSync(work, { recursive: true, force: true });
  }
  return failures;
}

test("viewer invariants hold for random diagrams and interactions", async ({ browser }) => {
  const property = fc.asyncProperty(
    modelArb,
    viewportArb,
    fc.array(actionArb, { maxLength: 8 }),
    async (model, viewport, actions) => {
      const failures = await runCase(browser, model, viewport, actions, null);
      if (failures.length > 0) throw new Error(failures.join("\n"));
    }
  );

  const result = await fc.check(property, {
    numRuns: Number(process.env.FUZZ_RUNS ?? 50),
    seed: process.env.FUZZ_SEED === undefined ? undefined : Number(process.env.FUZZ_SEED),
    path: process.env.FUZZ_PATH,
    examples: regressions,
  });
  if (!result.failed || !result.counterexample) return;

  const [model, viewport, actions] = result.counterexample;
  const dir = join(FAILURES_DIR, String(result.seed));
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "case.json"), JSON.stringify({ model, viewport, actions }, null, 2) + "\n");
  const failures = await runCase(browser, model, viewport, actions, dir);

  throw new Error(
    [
      `Counterexample after ${result.numRuns} runs and ${result.numShrinks} shrinks:`,
      ...failures,
      "",
      `Artifacts: ${dir}`,
      `  case.json, tree/ (diagraph view --root ${join(dir, "tree")}), diagraph.html, screenshot.png, trace.zip (npx playwright show-trace)`,
      `Replay:    FUZZ_SEED=${result.seed} FUZZ_PATH=${result.counterexamplePath} npm run fuzz`,
      `Regression entry for e2e/regressions.ts:`,
      JSON.stringify([model, viewport, actions]),
    ].join("\n")
  );
});
