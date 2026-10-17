// Daily WoW Forever build watch (task V4.1). Reads Blizzard's public,
// unauthenticated version list and prints a JSON report of the watched
// products' builds and any new `wow*` product code. It writes nothing; the
// workflow turns the report into GitHub issues.
import { readFileSync } from "node:fs";
import { MAX_RESPONSE_BYTES, WATCHED, buildIssueTitle, capIssues, newProducts, productIssueTitle, summarizeVersions } from "./lib.mjs";

const BASE = "https://us.version.battle.net/v2";

async function fetchText(path) {
  const response = await fetch(`${BASE}${path}`, { signal: AbortSignal.timeout(20000), redirect: "error" });
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  // Stream with a byte bound: a missing or false content-length can't make us buffer more.
  const chunks = [];
  let bytes = 0;
  for await (const chunk of response.body) {
    bytes += chunk.length;
    if (bytes > MAX_RESPONSE_BYTES) throw new Error(`${path}: response too large`);
    chunks.push(chunk);
  }
  return new TextDecoder("utf-8", { fatal: true }).decode(Buffer.concat(chunks));
}

const report = { checked_at: new Date().toISOString(), source: BASE, products: [], new_product_codes: [], issues: [] };
for (const product of WATCHED) {
  const summary = summarizeVersions(product, await fetchText(`/products/${product}/versions`));
  report.products.push(summary);
  for (const entry of summary.builds) {
    report.issues.push({
      title: buildIssueTitle(product, entry),
      body: [
        `Blizzard's public version list reports \`${product}\` at **${entry.version}** (build ${entry.build}, build config \`${entry.build_config}\`) in regions ${entry.regions.join(", ")}.`,
        summary.regions_agree ? "" : "Regions disagree: more than one build is live for this product.",
        "",
        "This is information only: no build is admitted automatically. If this is a new Forever build, see `docs/compatibility/BUILD_WATCH.md` for the follow-up (local inspect, admission as development input, re-extraction; after launch, V4.1/V4.2 target freeze).",
        "Close this issue to acknowledge it; a later run never reopens a closed title.",
      ].filter((line, i, all) => line !== "" || all[i - 1] !== "").join("\n"),
    });
  }
}
report.new_product_codes = newProducts(await fetchText("/summary"));
for (const code of report.new_product_codes) {
  report.issues.push({
    title: productIssueTitle(code),
    body: `Blizzard's public product summary lists a new product code \`${code}\` that the build watch doesn't know. After the WoW Forever launch this may be the live product; check it, then add it to \`KNOWN_WOW_PRODUCTS\` (and \`WATCHED\` if relevant) in \`scripts/build-watch/lib.mjs\`.`,
  });
}
// Titles already recorded (open or closed), read by the workflow beforehand.
const existing = new Set(process.argv[2] ? JSON.parse(readFileSync(process.argv[2], "utf8")) : []);
report.issues = capIssues(report.issues, existing, report.checked_at.slice(0, 10));
console.log(JSON.stringify(report, null, 2));
