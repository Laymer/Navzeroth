import { fireEvent, render, screen, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { GearScreen } from "./GearScreen";
import { PlannerScreen } from "./PlannerScreen";
import { createClient } from "../commands";
import dataset from "../../../../fixtures/contracts/v1/reference-dataset.json";
import planning from "../../../../fixtures/desktop/planning/synthetic-revision.json";

// V3.1: Help texts named by the G5a review (#5-#7) and Help on the per-item labels.

/** The text a Help trigger describes: the bubble its aria-describedby points at. */
function helpText(name: string): string {
  const trigger = screen.getByRole("button", { name });
  const id = trigger.getAttribute("aria-describedby") ?? "";
  return document.getElementById(id)?.textContent ?? "";
}

it("puts Help on the Gear per-item labels without renaming the controls", async () => {
  const catalog = { snapshot_id: "s", revision: "r", fixture: true, context: dataset.payload.items[0]?.key.context, equipment: {}, items: dataset.payload.items, acquisitions: dataset.payload.acquisitions };
  const client = createClient({ invoke: vi.fn((command: string) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic profile", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, nodes: [], objectives: [] } as never);
    if (command === "get_authored_catalog") return Promise.resolve({ revision: "r", records: [], total: 0, next_offset: null } as never);
    if (command === "get_gear_catalog") return Promise.resolve(catalog as never);
    return Promise.reject(new Error("unexpected command"));
  }) });
  render(<GearScreen client={client} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.click(screen.getByRole("button", { name: "Load labelled synthetic example" }));
  const item = dataset.payload.items[0]?.key.item_id ?? "";
  expect(screen.getByLabelText(`Category status for ${item}`).tagName).toBe("SELECT");
  expect(screen.getByLabelText(`Acquisition source for ${item}`).tagName).toBe("SELECT");
  expect(helpText(`Help: Category status for ${item}`)).toMatch(/unknown, unsupported and restricted stay distinct/);
  expect(helpText(`Help: Acquisition source for ${item}`)).toMatch(/nothing is chosen for you/);
  // G5a #7: the manual-model tips say what an authored model replaces, and that class is still checked.
  for (const label of ["Role identifier", "Attribute weights (JSON, integer thousandths)", "Class proficiency map (JSON allowed/disallowed categories)", "Main-hand slot", "Off-hand slot"]) {
    expect(helpText(`Help: ${label}`), label).toMatch(/authored model/);
  }
  expect(helpText("Help: Assumed class identifier")).toMatch(/must match that model's class/);
});

it("states what the Planner profile change and Clear discard, and helps on each objective node", async () => {
  const objectives = planning.envelope.payload.objectives;
  const client = createClient({ invoke: vi.fn((command: string) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, nodes: planning.envelope.payload.nodes, objectives } as never);
    return Promise.reject(new Error("unexpected"));
  }) });
  render(<PlannerScreen client={client} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  expect(helpText("Help: Planning profile")).toMatch(/considered objectives with their numbers, the exclusions, and the chosen origin and stopping node/);
  const clear = helpText("Help: Clear objective selections");
  expect(clear).toMatch(/every exclusion/);
  expect(clear).toMatch(/node, utility, completion and service values/);
  const first = objectives[0]?.id ?? "";
  const group = screen.getByRole("group", { name: first });
  fireEvent.click(within(group).getByLabelText("Consider objective"));
  expect(within(group).getByLabelText("Objective node").tagName).toBe("SELECT");
  expect(helpText(`Help: Objective node for ${first}`)).toMatch(/unselected node makes planning fail/);
});
