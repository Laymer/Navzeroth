import { act, fireEvent, render, screen } from "@testing-library/react";
import { createClient, type CommandAdapter } from "../commands";
import type { CompatibilityReport } from "../commands/compatibility";
import { CompatibilityPanel } from "./CompatibilityPanel";
function report(): CompatibilityReport {
  return { report_version: 1, app_version: "test", selected: null, active: null, recorded_builds: [{ build: { edition: "wow_forever", build: "70338", interface: "16001" }, version: "1.60.1", basis: "reported_flow", reported_on: "2026-10-13", addon_version: "0.1.0", matches_selected_build: null, limitations: "One manual sample; restricted contexts untested", capabilities: [{ capability: "C_Map.GetPlayerMapPosition", state: "function_present_only", statement: "Present, no readable position was observed", source: "sanitized-record" }] }, { build: { edition: "wow_forever", build: "1.60.1.70205", interface: "16001" }, version: "1.60.1", basis: "assumed_reference", reported_on: "2026-10-12", addon_version: null, matches_selected_build: null, limitations: "No client session", capabilities: [] }], official_api: "Official API enablement unverified; this does not establish absence", unverified_checks: [{ check: "native_interaction", required_evidence: "Recorded native window checks" }], limitations: "Historical finite evidence" };
}
it("separates reported function presence, assumed build and missing active scope", async () => {
  const adapter: CommandAdapter = { invoke(command, input) { expect(command).toBe("get_compatibility_report"); expect(input).toEqual({ profile_id: null }); return Promise.resolve(report() as never); } };
  render(<CompatibilityPanel client={createClient(adapter)} profileId={null} refreshKey={0} />);
  fireEvent.click(screen.getByRole("button", { name: "Inspect compatibility and coverage" }));
  await screen.findByText(/No selected profile: active dataset coverage is unavailable, not zero/);
  expect(screen.getByText(/Function present; usability untested/)).toBeInTheDocument();
  expect(screen.getByText(/Reference assumption; no client session/)).toBeInTheDocument();
  expect(screen.getByText(/Official API enablement unverified/)).toBeInTheDocument();
  expect(screen.getByText(/native interaction: Recorded native window checks/)).toBeInTheDocument();
});
it("discards deferred report after selection refresh", async () => {
  let finish: ((value: never) => void) | undefined;
  const adapter: CommandAdapter = { invoke() { return new Promise<never>((resolve) => { finish = resolve; }); } };
  const client = createClient(adapter); const view = render(<CompatibilityPanel client={client} profileId="old" refreshKey={0} />);
  fireEvent.click(screen.getByRole("button", { name: "Inspect compatibility and coverage" }));
  view.rerender(<CompatibilityPanel client={client} profileId="new" refreshKey={1} />);
  await act(async () => finish?.(report() as never));
  expect(screen.queryByText("Historical finite evidence")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Inspect compatibility and coverage" })).toBeEnabled();
});
it("shows finite collection zeros and source negative declarations distinctly", async () => {
  const value = report(); value.active = { context: { build: { edition: "wow_forever", build: "70338", interface: "16001" }, namespace: "demo" }, fixture: true, revision: "finite-r1", structurally_valid: true, finite_scope: "Only installed records counted", counts: { collections: { quests: { total: 0, evidence: { records_with_evidence: 0, records_without_evidence: 0, links_by_confidence: {} }, fields: {} } }, evidence_records: { by_confidence: {}, records_with_conflicts: 0, conflict_links: 0 }, edge_status: { verified: 0, unverified: 2 }, quest_graph: {}, atlas: { positions_checked: 0, projected: 0, gaps: {} } }, errors_by_code: {}, gaps_by_code: { unverified_geometry: 2 }, sources: { registered: 1, referenced_distinct: 2, unregistered_distinct: 1, evidence_references: 1, image_references: 1, policy_statuses: {}, redistribution_values: { permitted: 0, not_permitted: 1, conditional: 0 }, offline_values: { known_true: 0, known_false: 1 }, editions_include_selected: 0, editions_exclude_selected: 0, verified_revision_matches: 0, verified_revision_differs: 0, declared_conflicts: 0 } };
  const adapter: CommandAdapter = { invoke() { return Promise.resolve(value as never); } };
  render(<CompatibilityPanel client={createClient(adapter)} profileId="one" refreshKey={0} />);
  fireEvent.click(screen.getByRole("button", { name: "Inspect compatibility and coverage" }));
  await screen.findByText(/Revision finite-r1/);
  expect(screen.getByRole("row", { name: "quests 0 0" })).toBeInTheDocument();
  expect(screen.getByText(/not permitted 1, conditional 0/)).toBeInTheDocument();
  expect(screen.getByText(/known true 0, known false 1/)).toBeInTheDocument();
  expect(screen.getByText(/unregistered source references/)).toHaveTextContent("1 unregistered");
});
it("exposes only safe error code when report fails", async () => {
  const adapter: CommandAdapter = { invoke() { return Promise.reject({ code: "STORAGE_ERROR", message: "private-path", retryable: false }); } };
  render(<CompatibilityPanel client={createClient(adapter)} profileId="one" refreshKey={0} />);
  fireEvent.click(screen.getByRole("button", { name: "Inspect compatibility and coverage" }));
  await screen.findByRole("alert"); expect(screen.getByRole("alert")).toHaveTextContent("STORAGE_ERROR"); expect(screen.queryByText(/private-path/)).not.toBeInTheDocument();
});
