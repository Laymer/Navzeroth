import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { PlannerScreen } from "./PlannerScreen";
import { createClient, type CommandAdapter } from "../commands";
import fixture from "../../../../fixtures/desktop/planning/synthetic-revision.json";

it("ignores a response after its inputs change and clears results on refresh", async () => {
  let finish: (value: unknown) => void = () => {};
  const adapter: CommandAdapter = { invoke: vi.fn((command) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, nodes: fixture.envelope.payload.nodes, objectives: [] } as never);
    if (command === "plan_session") return new Promise<never>((resolve) => { finish = (value) => resolve(value as never); });
    return Promise.reject(new Error("unexpected"));
  }) };
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.change(screen.getByLabelText("Origin"), { target: { value: "demo-node-1" } });
  fireEvent.change(screen.getByLabelText("Stopping node"), { target: { value: "demo-node-1" } });
  fireEvent.click(screen.getByRole("button", { name: "Plan session" }));
  fireEvent.change(screen.getByLabelText("Budget (ms)"), { target: { value: "5000" } });
  finish({});
  await Promise.resolve();
  expect(screen.queryByRole("region", { name: "Planning result" })).not.toBeInTheDocument();
});

it("discards route responses when the planning inputs change", async () => {
  let finish: (value: unknown) => void = () => {};
  const adapter: CommandAdapter = { invoke: vi.fn((command, input) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, nodes: fixture.envelope.payload.nodes, objectives: [] } as never);
    if (command === "plan_session") return Promise.resolve({ request: JSON.parse((input as {text:string}).text), result: {selected: [], stopping: {total_ms:0}, expected_total_ms:0}, assumptions: [], filtered: [] } as never);
    if (command === "inspect_plan_route") return new Promise<never>((resolve) => { finish = (value) => resolve(value as never); });
    return Promise.reject(new Error("unexpected"));
  }) };
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.change(screen.getByLabelText("Origin"), { target: { value: "demo-node-1" } });
  fireEvent.change(screen.getByLabelText("Stopping node"), { target: { value: "demo-node-1" } });
  fireEvent.click(screen.getByRole("button", { name: "Plan session" }));
  fireEvent.click(await screen.findByRole("button", { name: "Inspect route" }));
  fireEvent.change(screen.getByLabelText("Budget (ms)"), { target: { value: "5000" } });
  finish({});
  expect(screen.queryByRole("region", { name: "Route inspection" })).toBeNull();
});

it("preserves same-profile manual planning inputs during routine refresh", async () => {
  const client = createClient({ invoke: vi.fn((command) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ snapshot_id: "s", revision: "r", fixture: true, nodes: fixture.envelope.payload.nodes, objectives: [] } as never);
    return Promise.reject(new Error("unexpected"));
  }) });
  const view = render(<PlannerScreen client={client} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.change(screen.getByLabelText("Origin"), { target: { value: "demo-node-1" } });
  fireEvent.change(screen.getByLabelText("Stopping node"), { target: { value: "demo-node-2" } });
  fireEvent.change(screen.getByLabelText("Budget (ms)"), { target: { value: "5000" } });
  view.rerender(<PlannerScreen client={client} refreshKey={1} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  expect(screen.getByLabelText("Origin")).toHaveValue("demo-node-1");
  expect(screen.getByLabelText("Stopping node")).toHaveValue("demo-node-2");
  expect(screen.getByLabelText("Budget (ms)")).toHaveValue(5000);
});
