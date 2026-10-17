// Pure helpers for the WoW Forever build watch (task V4.1). No I/O here: the
// workflow script fetches Blizzard's public version list and passes text in.
// Reading the list never admits a build; admission stays a reviewed change.

/** Products whose builds are reported on every run. */
export const WATCHED = [
  // The WoW Forever beta the owner's client runs (1.60.1.x).
  "wow_classic_beta",
  // Lists 1.60.1.x builds and is the likely Forever live product; unconfirmed.
  "wowf",
];

/** `wow*` product codes known on 2026-10-17; any other code is reported as new. */
export const KNOWN_WOW_PRODUCTS = [
  "wow", "wow_anniversary", "wow_beta", "wow_classic", "wow_classic_beta",
  "wow_classic_era", "wow_classic_era_beta", "wow_classic_era_ptr", "wow_classic_ptr",
  "wow_classic_titan", "wow_cn_beta", "wowdev", "wowdev2", "wowdev3", "wowdev4",
  "wowdev5", "wowdev6", "wowe1", "wowe2", "wowe3", "wowf", "wowlivetest",
  "wowlivetest2", "wownev1", "wownev10", "wownev11", "wownev12", "wownev2", "wownev3",
  "wownev4", "wownev5", "wownev6", "wownev7", "wownev8", "wownev9", "wowt", "wowv",
  "wowv10", "wowv2", "wowv3", "wowv4", "wowv5", "wowv6", "wowv7", "wowv8", "wowv9",
  "wowxptr", "wowz",
];

export const MAX_RESPONSE_BYTES = 256 * 1024;
/** More changes than this in one run become a single aggregate issue. */
export const MAX_ISSUES_PER_RUN = 5;

/**
 * Parses Blizzard's pipe-separated version tables: a `Name!TYPE:n|...` header,
 * `## seqn = N` comments and data rows. Refuses anything else instead of guessing.
 */
export function parseTable(text) {
  if (typeof text !== "string" || text.length > MAX_RESPONSE_BYTES) throw new Error("Version table is missing or too large");
  const lines = text.split(/\r?\n/).filter((line) => line.trim() !== "");
  const header = lines.find((line) => !line.startsWith("##"));
  if (!header || !header.includes("!")) throw new Error("Version table has no header");
  const columns = header.split("|").map((column) => column.split("!")[0]);
  if (new Set(columns).size !== columns.length || columns.some((name) => !/^[A-Za-z][A-Za-z0-9 ]*$/.test(name))) {
    throw new Error("Version table header is invalid");
  }
  const seqnLine = lines.find((line) => /^## seqn = \d+$/.test(line));
  const rows = lines
    .filter((line) => line !== header && !line.startsWith("##"))
    .map((line) => {
      const cells = line.split("|");
      if (cells.length !== columns.length) throw new Error("Version table row has the wrong column count");
      return Object.fromEntries(columns.map((name, i) => [name, cells[i]]));
    });
  return { seqn: seqnLine ? Number(seqnLine.slice("## seqn = ".length)) : null, rows };
}

/** Distinct (version, build, build config) per product, with the regions reporting each. */
export function summarizeVersions(product, text) {
  const { seqn, rows } = parseTable(text);
  // A watched product with no rows is a change worth failing on, not "nothing new".
  if (rows.length === 0) throw new Error(`No version rows for ${product}`);
  const builds = new Map();
  for (const row of rows) {
    const build = row["BuildId"];
    const version = row["VersionsName"];
    // Every value that reaches an issue is checked; nothing else is passed on.
    if (!/^[a-z]{2,8}$/.test(row["Region"] ?? "") || !/^\d{1,9}$/.test(build ?? "") || !/^\d+\.\d+\.\d+\.\d+$/.test(version ?? "") || !version.endsWith(`.${build}`) || !/^[0-9a-f]{32}$/.test(row["BuildConfig"] ?? "")) {
      throw new Error(`Unexpected version row for ${product}`);
    }
    const key = `${version}|${row["BuildConfig"]}`;
    const entry = builds.get(key) ?? { version, build: Number(build), build_config: row["BuildConfig"], regions: [] };
    entry.regions.push(row["Region"]);
    builds.set(key, entry);
  }
  const list = [...builds.values()].sort((a, b) => b.build - a.build || a.version.localeCompare(b.version));
  for (const entry of list) entry.regions.sort();
  return { product, seqn, builds: list, regions_agree: list.length <= 1 };
}

/** `wow*` product codes in the summary that aren't in the known list. */
export function newProducts(summaryText, known = KNOWN_WOW_PRODUCTS) {
  const { rows } = parseTable(summaryText);
  const seen = new Set(rows.map((row) => row["Product"]).filter((code) => /^wow[a-z0-9_]*$/.test(code ?? "")));
  return [...seen].filter((code) => !known.includes(code)).sort();
}

/** Stable issue titles: one issue per product build and per new product code. */
export function buildIssueTitle(product, entry) {
  return `Build watch: ${product} is ${entry.version} (build ${entry.build})`;
}
export function productIssueTitle(code) {
  return `Build watch: new WoW product code ${code}`;
}

/**
 * Bounds one run: more than MAX_ISSUES_PER_RUN changes (unseen titles) become one
 * aggregate issue, so a malformed or hostile response can't open many issues.
 */
export function capIssues(issues, existingTitles, date) {
  const fresh = issues.filter((issue) => !existingTitles.has(issue.title));
  if (fresh.length <= MAX_ISSUES_PER_RUN) return fresh;
  return [{
    title: `Build watch: ${fresh.length} changes on ${date} need manual review`,
    body: `The build watch saw ${fresh.length} builds or product codes it hadn't recorded, more than the per-run limit of ${MAX_ISSUES_PER_RUN}. No individual issues were opened. Check Blizzard's version list by hand (see docs/compatibility/BUILD_WATCH.md).`,
  }];
}
