// Contract checks for TypeScript consumers. Run with: pnpm contracts:check
// (tsc type-checks and emits to node_modules/.cache/contract-check; Node runs
// the emitted JavaScript, so no runtime TypeScript support is required).
// 1. Ajv validates every manifest fixture against the generated JSON Schema.
// 2. Schema-valid fixtures are narrowed through the generated TypeScript types
//    with exhaustive switches; display categories and safe numbers are asserted.
// Rust remains authoritative for semantic rules (semantic_invalid fixtures).
import { existsSync, readFileSync } from "node:fs";
import { Ajv2020 } from "ajv/dist/2020.js";
import type {
  AzerothNavigatorExchangeV1 as Exchange,
  CharacterSnapshot,
  CharacterSnapshotEnvelope,
  Copper,
  Observation_Level,
  Predicate,
} from "../../schemas/v1/exchange.ts";

/** The repository root is the nearest ancestor holding pnpm-workspace.yaml. */
function findRoot(): URL {
  for (let dir = new URL("./", import.meta.url); ; dir = new URL("../", dir)) {
    if (existsSync(new URL("pnpm-workspace.yaml", dir))) return dir;
    if (dir.pathname === "/" || new URL("../", dir).href === dir.href) throw new Error("repository root not found");
  }
}
const root = findRoot();
const readJson = (path: string): unknown => JSON.parse(readFileSync(new URL(path, root), "utf8"));

type Expectation = "valid" | "schema_invalid" | "semantic_invalid";
interface ManifestEntry {
  file: string;
  kind: Exchange["kind"];
  expect: Expectation;
  error_code?: string;
}
const manifest = readJson("fixtures/contracts/v1/manifest.json") as { fixtures: ManifestEntry[] };

const envelopeDefs: Record<Exchange["kind"], string> = {
  character_snapshot: "CharacterSnapshotEnvelope",
  party_snapshot: "PartySnapshotEnvelope",
  route_plan: "RoutePlanEnvelope",
  evidence_bundle: "EvidenceBundleEnvelope",
  reference_dataset: "ReferenceDatasetEnvelope",
  gear_recommendations: "GearRecommendationSetEnvelope",
};

function assertNever(value: never, what: string): never {
  throw new Error(`Unhandled ${what}: ${JSON.stringify(value)}`);
}
function check(condition: boolean, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

// ---- Display categories ------------------------------------------------------
type Category = "value" | "unknown" | "unsupported" | "restricted" | "not_applicable";
type ObservationLike =
  | { status: "known"; value: unknown }
  | { status: "unknown" | "unsupported" | "restricted" | "not_applicable"; value?: never };
// Generated observation types must stay assignable to the display union.
const observationShape = (o: Observation_Level): ObservationLike => o;

function category(o: ObservationLike): Category {
  switch (o.status) {
    case "known":
      check(o.value !== undefined && o.value !== null, "known observation without value");
      return "value";
    case "unknown":
    case "unsupported":
    case "restricted":
    case "not_applicable":
      check(!("value" in o), `${o.status} observation carries a value`);
      return o.status;
    default:
      return assertNever(o, "observation status");
  }
}

/** Record maps use strict index signatures: absent keys read as undefined. */
function entries<T>(map: { [key: string]: T | undefined }): T[] {
  return Object.values(map).filter((v): v is T => v !== undefined);
}

function snapshotCategories(s: CharacterSnapshot): Category[] {
  const fields: ObservationLike[] = [observationShape(s.level), s.role, s.location];
  fields.push(...entries(s.preferences), ...entries(s.equipment), ...entries(s.objectives));
  for (const quest of entries(s.quests)) {
    fields.push(quest.accepted, quest.completed, quest.turned_in, ...entries(quest.objectives));
  }
  return fields.map(category);
}

function predicateDepth(p: Predicate): number {
  switch (p.op) {
    case "all":
    case "any":
      return 1 + Math.max(0, ...p.predicates.map(predicateDepth));
    case "not":
    case "participant_state":
      return 1 + predicateDepth(p.predicate);
    case "known":
    case "equals":
    case "greater_than_or_equal":
    case "quest_state":
    case "objective_state":
    case "level_at_least":
    case "equipment_matches":
    case "role_is":
    case "location_in_supported_area":
    case "user_preference":
      return 1;
    default:
      return assertNever(p, "predicate operator");
  }
}

function demoLabel(doc: Exchange): "Demo" | "Fixture" | "Reference" {
  switch (doc.namespace) {
    case "demo":
      return "Demo";
    case "reference":
      return doc.fixture ? "Fixture" : "Reference";
    default:
      return assertNever(doc.namespace, "namespace");
  }
}

function copperIsCanonical(c: Copper): boolean {
  return typeof c === "string" && /^(?:0|[1-9][0-9]{0,19})$/.test(c) && BigInt(c) < 2n ** 64n;
}

/** Every JSON number is finite; integers are within the safe integer range. */
function numbersAreSafe(value: unknown): boolean {
  if (typeof value === "number") return Number.isFinite(value) && (!Number.isInteger(value) || Number.isSafeInteger(value));
  if (Array.isArray(value)) return value.every(numbersAreSafe);
  if (value !== null && typeof value === "object") return Object.values(value).every(numbersAreSafe);
  return true;
}

function summarize(doc: Exchange): string {
  check(doc.format === "azeroth-navigator" && doc.schema_version === 1, "header");
  check(doc.namespace !== "demo" || doc.fixture, "demo data must be labeled fixture");
  const label = demoLabel(doc);
  switch (doc.kind) {
    case "character_snapshot": {
      const categories = new Set(snapshotCategories(doc.payload));
      return `${label} snapshot: ${[...categories].sort().join(",")}`;
    }
    case "party_snapshot": {
      const times = doc.payload.participants.map((p) => p.capture.observed_at_utc_ms ?? null);
      check(new Set(times).size === times.length, "participants keep individual capture times");
      doc.payload.participants.forEach(snapshotCategories);
      return `${label} party of ${doc.payload.participants.length}`;
    }
    case "route_plan": {
      const depths = doc.payload.steps.flatMap((s) =>
        [s.prerequisite, s.invalidation].map((o) => (o.status === "known" ? predicateDepth(o.value) : 0)),
      );
      return `${label} plan: ${doc.payload.steps.length} steps, rule depth ${Math.max(0, ...depths)}`;
    }
    case "evidence_bundle":
      return `${label} evidence: ${doc.payload.evidence.length} records`;
    case "reference_dataset": {
      for (const a of doc.payload.acquisitions) {
        if (a.cost.status === "known") check(copperIsCanonical(a.cost.value), "copper stays a canonical string");
      }
      return `${label} dataset ${doc.payload.revision}`;
    }
    case "gear_recommendations": {
      const scores = doc.payload.recommendations.flatMap((r) => [category(r.baseline_score), category(r.proposed_score)]);
      return `${label} gear: ${scores.join(",")}`;
    }
    default:
      return assertNever(doc, "exchange kind");
  }
}

// ---- Compile-time contract checks (type-checked by tsc, never executed) ------
export function compileTimeChecks(): void {
  // @ts-expect-error known observations require a value
  const missing: Observation_Level = { status: "known" };
  // @ts-expect-error unavailable observations cannot carry a value
  const leaked: Observation_Level = { status: "restricted", value: 3 };
  // @ts-expect-error copper is a decimal string, never a number
  const cost: Copper = 100;
  // @ts-expect-error the kind literal is tied to the envelope type
  const kind: CharacterSnapshotEnvelope["kind"] = "route_plan";
  // @ts-expect-error unknown predicate operators are not representable
  const op: Predicate = { op: "eval", code: "return true" };
  void [missing, leaked, cost, kind, op];
}

// ---- Run --------------------------------------------------------------------
const schema = readJson("schemas/v1/exchange.schema.json") as { $id: string };
const ajv = new Ajv2020({ allErrors: false, strict: true });
ajv.addSchema(schema);
const rootValidator = ajv.getSchema(schema.$id);
check(rootValidator !== undefined, "root schema compiles");

let failures = 0;
const summaries: string[] = [];
for (const entry of manifest.fixtures) {
  const doc = readJson(`fixtures/contracts/v1/${entry.file}`);
  const kindValidator = ajv.getSchema(`${schema.$id}#/$defs/${envelopeDefs[entry.kind]}`);
  check(kindValidator !== undefined, `schema for ${entry.kind}`);
  const accepted = kindValidator(doc) && rootValidator(doc);
  const expected = entry.expect !== "schema_invalid";
  if (accepted !== expected) {
    failures++;
    console.error(`FAIL ${entry.file}: schema ${accepted ? "accepted" : "rejected"}, expected ${entry.expect}`);
    continue;
  }
  if (entry.expect === "valid") {
    check(numbersAreSafe(doc), `${entry.file}: unsafe number`);
    summaries.push(`${entry.file}: ${summarize(doc as Exchange)}`);
  }
}
const snapshot = summaries.find((s) => s.startsWith("character-snapshot.json"));
check(
  snapshot?.endsWith("not_applicable,restricted,unknown,unsupported,value") === true,
  "character snapshot exercises every observation category",
);
check(
  summaries.some((s) => s.startsWith("route-plan-max-depth.json") && s.endsWith("rule depth 12")),
  "max-depth plan reaches rule depth 12",
);
summaries.forEach((s) => console.log(s));
if (failures > 0) {
  console.error(`${failures} fixture expectation(s) failed`);
  process.exit(1);
}
console.log(`OK: ${manifest.fixtures.length} fixtures checked against schema and TypeScript handling`);
