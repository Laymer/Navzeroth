// Structural strict-schema regression; Rust tests own graph invariants.
import { readFileSync } from "node:fs";
import { Ajv2020 } from "ajv/dist/2020.js";
const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const schema = read("../schemas/v2/reference-content.schema.json");
const fixture = read("../fixtures/contracts/v2/reference-content.json");
const ajv = new Ajv2020({ strict: true, allErrors: true });
const validate = ajv.compile(schema);
if (!validate(fixture)) throw new Error(JSON.stringify(validate.errors));
const reject = (edit) => {
  const value = structuredClone(fixture);
  edit(value);
  if (validate(value)) throw new Error("Invalid v2 fixture passed schema");
};
reject((v) => { v.schema_version = 1; });
reject((v) => { v.models[0].profile.untyped = true; });
reject((v) => { v.reference.payload.items[0].untyped = true; });
reject((v) => { v.quest_facts[0].shareable.value = null; });
reject((v) => { v.acquisition_facts[0].entitlement.value = false; });
reject((v) => { v.quest_facts[0].exclusive_groups = { status: "known", value: [{ quests: ["a", "b"] }] }; });
const withNodes = structuredClone(fixture);
withNodes.node_facts = [{ node: "demo-node-1", evidence: ["demo-evidence-1"] }];
if (!validate(withNodes)) throw new Error(JSON.stringify(validate.errors));
reject((v) => { v.node_facts = [{ node: "demo-node-1", evidence: [] }]; });
reject((v) => { v.node_facts = [{ node: "demo-node-1", evidence: ["demo-evidence-1", "demo-evidence-1"] }]; });
reject((v) => { v.node_facts = [{ node: "demo-node-1", evidence: ["demo-evidence-1"], unknown: true }]; });
// Committed authored slice catalog (task V2.4), original authoring only.
const authored = read("../content/authored/slice-v1/catalog.json");
if (!validate(authored)) throw new Error(JSON.stringify(validate.errors));
console.log("V2 schema: legacy and node-fact fixtures, the authored slice catalog and nine strict rejection cases passed");
