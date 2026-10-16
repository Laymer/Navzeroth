#!/usr/bin/env node
// Fails when the production bundle contains a `data:` URI or a remote asset, font or
// image URL. The native CSP allows neither (default-src 'self'; img-src 'self' atlas:),
// and Vite inlines assets under 4 KiB unless build.assetsInlineLimit is 0.
// Usage: node scripts/check-dist.mjs [dist-directory]
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const dist = resolve(process.argv[2] ?? join(root, "apps/desktop/dist"));

const TEXT = new Set([".html", ".css", ".js", ".mjs", ".json", ".svg", ".webmanifest", ".map"]);
const ASSET = "png|jpe?g|gif|svg|webp|avif|ico|woff2?|ttf|otf|eot|css|m?js|wasm";
const rules = [
  ["data: URI", /data:[a-z]+\/[a-z0-9.+-]+(?:;[a-z0-9=._-]+)*[;,]/gi],
  ["remote asset URL", new RegExp(`(?:https?:)?//[a-z0-9.-]+\\.[a-z]{2,}(?::\\d+)?/[^"'\`)\\s]*\\.(?:${ASSET})\\b`, "gi")],
  ["remote stylesheet or import", /@import\s+(?:url\()?["']?(?:https?:)?\/\//gi],
  ["remote url()", /url\(\s*["']?(?:https?:)?\/\/[a-z0-9]/gi],
  ["remote src/href attribute", /\b(?:src|href)=["'](?:https?:)?\/\/[a-z0-9]/gi],
];

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) yield* walk(path);
    else yield path;
  }
}

let files;
try {
  files = [...walk(dist)];
} catch {
  console.error(`check-dist: ${dist} does not exist; run the desktop build first.`);
  process.exit(2);
}
if (!files.some((file) => file.endsWith("index.html"))) {
  console.error(`check-dist: ${dist} has no index.html.`);
  process.exit(2);
}

const failures = [];
let scanned = 0;
for (const file of files) {
  if (!TEXT.has(extname(file).toLowerCase())) continue;
  scanned += 1;
  const text = readFileSync(file, "utf8");
  for (const [label, pattern] of rules) {
    for (const match of text.matchAll(pattern)) {
      failures.push(`${relative(dist, file)}: ${label}: ${match[0].slice(0, 80)}`);
    }
  }
}

if (failures.length > 0) {
  console.error(`check-dist: ${failures.length} problem(s) in ${dist}`);
  for (const line of failures) console.error(`  ${line}`);
  process.exit(1);
}
console.log(`check-dist: ${scanned} text files and ${files.length} files clean in ${relative(root, dist) || "."}`);
