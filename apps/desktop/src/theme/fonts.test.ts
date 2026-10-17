import { describe, expect, it } from "vitest";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

// G4 review #9: nothing else verifies the bundled font binaries and licence texts against the
// sha256 values that THIRD_PARTY.md records, so this test does.

const fontsDir = join(__dirname, "fonts");
const notices = readFileSync(join(__dirname, "..", "..", "THIRD_PARTY.md"), "utf8");

/** Rows like: | ... | `src/theme/fonts/NAME` | ... | `<64 hex>` |  (the hash is the last backticked cell). */
function recordedHashes(): Map<string, string> {
  const result = new Map<string, string>();
  for (const line of notices.split("\n")) {
    const file = /`src\/theme\/fonts\/([^`]+)`/.exec(line)?.[1];
    const hashes = [...line.matchAll(/`([0-9a-f]{64})`/g)].map((m) => m[1] ?? "");
    const last = hashes[hashes.length - 1];
    if (file !== undefined && last !== undefined) result.set(file, last);
  }
  return result;
}

const sha256 = (name: string) => createHash("sha256").update(readFileSync(join(fontsDir, name))).digest("hex");

describe("bundled fonts", () => {
  const recorded = recordedHashes();
  const present = readdirSync(fontsDir).filter((name) => !name.endsWith(".css"));

  it("records a hash for every file in src/theme/fonts", () => {
    expect([...recorded.keys()].sort()).toEqual([...present].sort());
    expect(present.length).toBe(4);
  });

  for (const name of present) {
    it(`${name} matches the sha256 in THIRD_PARTY.md`, () => {
      expect(sha256(name)).toBe(recorded.get(name));
    });
  }
});
