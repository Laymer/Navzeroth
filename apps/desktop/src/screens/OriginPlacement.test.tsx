// Origin markers in the existing catalog views (ADR 0007 §3): gear items, planner
// objectives, route hops and dungeon preparation records. The backend computes
// the marks; these tests only check placement, binding and silence on failure.
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { createClient, type CommandAdapter } from "../commands";
import { GearScreen } from "./GearScreen";
import { PlannerScreen } from "./PlannerScreen";
import { DungeonsScreen } from "./DungeonsScreen";
import { RouteInspection } from "./RouteInspection";
import { originKey, type OriginMark, type OriginRef } from "../commands/world";
import { syntheticRoute } from "./test-fixtures/route";
import dataset from "../../../../fixtures/contracts/v1/reference-dataset.json";
import planning from "../../../../fixtures/desktop/planning/synthetic-revision.json";
import content from "../../../../fixtures/contracts/v2/reference-content.json";
import preparation from "../../../../fixtures/desktop/preparation/report.json";
import type { PreparationReport } from "../commands/dungeonPreparation";

/** Answers every origin lookup from `marks` by collection; records each request. */
function origins(marks: Partial<Record<OriginRef["collection"], OriginMark[]>>, requests: unknown[]) {
  return (input: unknown) => {
    const request = JSON.parse((input as { text: string }).text) as { revision: string; records: OriginRef[] };
    requests.push(request);
    return Promise.resolve({ revision: request.revision, origins: request.records.map((r) => marks[r.collection] ?? []) } as never);
  };
}

it("marks gear candidates beside their legend with the exact catalog revision", async () => {
  const requests: unknown[] = [];
  const adapter: CommandAdapter = { invoke: vi.fn((command, input) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", data_mode: "stored" }] as never);
    if (command === "get_gear_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, context: dataset.payload.items[0]?.key.context, equipment: {}, items: dataset.payload.items, acquisitions: dataset.payload.acquisitions } as never);
    if (command === "get_authored_catalog") return Promise.resolve({ revision: "r", records: [], total: 0, next_offset: null } as never);
    if (command === "get_record_origins") return origins({ items: ["vanilla_1_12"] }, requests)(input);
    return Promise.reject(new Error("unavailable"));
  }) };
  render(<GearScreen client={createClient(adapter)} refreshKey={0} />);
  const item = dataset.payload.items[0]?.key;
  const legend = await screen.findByText(`${item?.item_id} (${item?.variant})`, { selector: "legend" });
  await waitFor(() => expect(legend.parentElement?.querySelector(".ui-origin")).toHaveTextContent("From 1.12 data"));
  expect(requests[0]).toMatchObject({ subject: { kind: "character", profile_id: "p" }, revision: "r", records: [{ collection: "items", id: item?.item_id, variant: item?.variant }] });
});

it("marks planner objectives and leaves Forever-origin objectives unmarked", async () => {
  const objectives = planning.envelope.payload.objectives;
  const adapter: CommandAdapter = { invoke: vi.fn((command, input) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", data_mode: "stored" }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, nodes: planning.envelope.payload.nodes, objectives } as never);
    if (command === "get_record_origins") return origins({ objectives: ["wow_classic_era", "inferred"] }, [])(input);
    return Promise.reject(new Error("unavailable"));
  }) };
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);
  const legend = await screen.findByText(objectives[0]?.id ?? "", { selector: "legend" });
  await waitFor(() => expect(legend.parentElement?.querySelector(".ui-origin")).toHaveTextContent(/From Classic Era data.*Inferred/));
});

it("marks inferred route hops from the origins it is given", () => {
  const value = syntheticRoute(2);
  const marks = new Map([[originKey({ collection: "edges", id: "e1" }), ["inferred"] as OriginMark[]]]);
  render(<RouteInspection value={value} origins={marks} />);
  const hops = screen.getAllByText(/walking; \d+ ms; estimated/);
  expect(hops[0]?.querySelector(".ui-origin")).toBeNull();
  expect(hops[1]).toHaveTextContent(/Inferred$/);
});

it("marks the authored dungeon, its quests, reward sources and travel edges", async () => {
  const edge = content.reference.payload.edges[0];
  const report = { ...preparation, authored: { revision: "demo-r1", id: "dungeon-1" }, authored_travel: { declaration: { observation: { status: "known", value: [edge?.id] }, evidence: [] }, edges: [edge] } } as unknown as PreparationReport;
  const requests: unknown[] = [];
  const client = {
    getAuthoredCatalog: vi.fn(async () => ({ ok: true, value: { revision: null, records: [], total: 0, next_offset: null } })),
    listProfiles: vi.fn(async () => ({ ok: true, value: [{ id: preparation.members[0]?.profile_id, display_name: "Synthetic member", data_mode: "stored" }] })),
    listParties: vi.fn(async () => ({ ok: true, value: [] })),
    getDungeonPreparation: vi.fn(async () => ({ ok: true, value: report })),
    getRecordOrigins: vi.fn(async (text: string) => ({ ok: true, value: await origins({ dungeons: ["vanilla_1_12"], quests: ["wow_classic_era"], acquisitions: ["vanilla_1_12"], edges: ["inferred"] }, requests)({ text }) })),
  };
  render(<DungeonsScreen client={client as never} refreshKey={0} />);
  await waitFor(() => expect(screen.getByRole("button", { name: "Inspect preparation gaps" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Inspect preparation gaps" }));
  const title = await screen.findByRole("heading", { level: 3, name: /^Synthetic preparation reference/ });
  await waitFor(() => expect(title).toHaveTextContent(/From 1\.12 data$/));
  expect(screen.getAllByText(/^demo-quest-1/, { selector: "li" }).some((li) => li.querySelector(".ui-origin")?.textContent?.includes("From Classic Era data"))).toBe(true);
  expect(screen.getAllByText(/via demo-acq-1/, { selector: "li" }).every((li) => li.querySelector(".ui-origin")?.textContent?.includes("From 1.12 data"))).toBe(true);
  expect(within(screen.getByRole("region", { name: "Authored travel dependencies" })).getByText(/demo-edge-1:/)).toHaveTextContent(/Inferred$/);
  const expected: OriginRef[] = [{ collection: "dungeons", id: "dungeon-1" }, { collection: "quests", id: "demo-quest-1" }, { collection: "acquisitions", id: "demo-acq-1" }, { collection: "edges", id: "demo-edge-1" }];
  expect(requests.length).toBeGreaterThan(0);
  for (const request of requests) expect(request).toMatchObject({ subject: { kind: "character", profile_id: preparation.members[0]?.profile_id }, revision: "demo-r1" });
  const requested = requests.flatMap(request => (request as { records: OriginRef[] }).records).map(originKey);
  expect([...new Set(requested)].sort()).toEqual(expected.map(originKey).sort());
});
