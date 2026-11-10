import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { vi } from "vitest";
import { createClient, type CommandAdapter } from "../../commands";
import type { CommandMap, CommandName } from "../../commands/contract";
import type { ManualProgressionRequest, ProgressionReceipt, ProgressionView } from "../../../../../schemas/v1/progression-api";
import fixture from "../../../../../fixtures/contracts/v1/progression-snapshot.json";
import { ProgressionInputs } from "./ProgressionInputs";
import { ProgressionRecords } from "./ProgressionRecords";
import { inputError, manualRequest } from "./inputs";

const view: ProgressionView = { profile_id: "profile-one", fixture: true, binding: fixture.payload.binding, imported: [], manual: [] } as ProgressionView;
const receipt: ProgressionReceipt = { profile_id: view.profile_id, snapshot_id: view.binding.snapshot, companion_id: "saved-companion", stored: true, fixture: true };
const unavailable = { code: "PROVIDER_UNAVAILABLE", message: "Storage unavailable", retryable: true };
function deferred<T>() { let resolve!: (value: T) => void; let reject!: (reason: unknown) => void; const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; }
function harness() {
  const calls: { command: CommandName; input: unknown }[] = [];
  const get = vi.fn<() => Promise<ProgressionView>>().mockResolvedValue(view);
  const save = vi.fn<() => Promise<ProgressionReceipt>>().mockResolvedValue(receipt);
  const adapter: CommandAdapter = { invoke<C extends CommandName>(command: C, input: CommandMap[C]["input"]): Promise<CommandMap[C]["output"]> {
    calls.push({ command, input });
    return (command === "get_progression_view" ? get() : save()) as Promise<CommandMap[C]["output"]>;
  } };
  return { client: createClient(adapter), calls, get, save };
}
async function open() {
  fireEvent.click(screen.getByRole("button", { name: "Open progression inputs" }));
  await waitFor(() => expect(screen.queryByText("Loading progression observations…")).not.toBeInTheDocument());
}
function props(h: ReturnType<typeof harness>) { return { client: h.client, profileId: view.profile_id, snapshotId: view.binding.snapshot, disabled: false }; }

it("loads only on explicit open and saves zero with exact binding and other observations unknown", async () => {
  const h = harness(); render(<ProgressionInputs {...props(h)} />);
  expect(h.calls).toEqual([]); await open();
  expect(screen.getByText(/No imported progression/)).toBeInTheDocument();
  expect(screen.getByText(/No manual progression/)).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Save manual entry" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Money (copper)"), { target: { value: "0" } });
  fireEvent.change(screen.getByLabelText("Available talent points"), { target: { value: "0" } });
  fireEvent.click(screen.getByRole("button", { name: "Save manual entry" }));
  expect(await screen.findByText(/Saved progression capture: saved-companion \(synthetic fixture\)/)).toBeInTheDocument();
  const call = h.calls.find((v) => v.command === "save_manual_progression");
  if (call === undefined) throw new Error("Manual save was not invoked");
  const request = JSON.parse((call.input as { text: string }).text) as ManualProgressionRequest;
  expect(request).toMatchObject({ profile_id: view.profile_id, expected: view.binding, id: expect.stringMatching(/^manual-/), money: { status: "known", value: "0" }, available_talent_points: { status: "known", value: 0 }, spells: { status: "unknown" }, talents: { status: "unknown" }, professions: { status: "unknown" }, recipes: { status: "unknown" }, bags: { status: "unknown" }, bank: { status: "unknown" }, equipment_enhancements: {} });
  expect(screen.getByLabelText("Money (copper)")).toHaveValue("");
  expect(screen.getByLabelText("Available talent points")).toHaveValue("");
  expect(h.get).toHaveBeenCalledTimes(2);
});

it("rejects invalid numbers without rounding currency and preserves unknown separately from zero", () => {
  for (const value of ["-1", "1.1", "1e3", "01", " ", "18446744073709551616", "999999999999999999999"]) expect(inputError(value, "")).toMatch(/Money must/);
  for (const value of ["-1", "1.1", "1e3", "01", "65536", "100000"]) expect(inputError("", value)).toMatch(/talent points must/);
  expect(inputError("18446744073709551615", "65535")).toBeNull();
  expect(manualRequest(view, "18446744073709551615", "").money).toEqual({ status: "known", value: "18446744073709551615" });
  expect(manualRequest(view, "", "0").money).toEqual({ status: "unknown" });
  expect(manualRequest(view, "0", "").available_talent_points).toEqual({ status: "unknown" });
  expect(manualRequest(view, "0", "").id).not.toEqual(manualRequest(view, "0", "").id);
});

it("imports a companion explicitly and explains duplicate receipts", async () => {
  const h = harness(); h.save.mockResolvedValue({ ...receipt, stored: false, fixture: false });
  render(<ProgressionInputs {...props(h)} />); await open();
  fireEvent.change(screen.getByLabelText("Progression export text"), { target: { value: "ANAV1:companion" } });
  fireEvent.click(screen.getByRole("button", { name: "Import progression companion" }));
  expect(await screen.findByText(/Progression capture was already stored: saved-companion/)).toBeInTheDocument();
  expect(h.calls).toContainEqual({ command: "import_progression_text", input: { text: JSON.stringify({ profile_id: view.profile_id, expected: view.binding, text: "ANAV1:companion" }) } });
  expect(screen.getByLabelText("Progression export text")).toHaveValue("");
});

it("shows capture origins, stale/uncertain age, empty/partial collections and unavailable fields", () => {
  const base = fixture.payload as ProgressionView["imported"][number]["snapshot"];
  const { rerender } = render(<ProgressionRecords origin="Imported" records={[{ snapshot: base, freshness: { status: "stale", age_ms: 120000 } }]} />);
  expect(screen.getByText(/Imported capture.*stale/)).toBeInTheDocument();
  fireEvent.click(screen.getByText(/Imported capture/));
  expect(screen.getByText("Source: Synthetic fixture.")).toBeInTheDocument();
  expect(screen.getByText("0 copper")).toBeInTheDocument();
  expect(screen.getByText("0 recorded entries; complete enumeration")).toBeInTheDocument();
  expect(screen.getByText("1 recorded entries; partial enumeration, other entries unknown")).toBeInTheDocument();
  expect(screen.getByText("Unsupported by this client")).toBeInTheDocument();
  const manual = { ...base, capture: { ...base.capture, source: "manual_entry" as const }, money: { status: "restricted" as const }, recipes: { status: "not_applicable" as const }, bank: { status: "known" as const, value: { completeness: "partial" as const, entries: {} } } };
  rerender(<ProgressionRecords origin="Manual" records={[{ snapshot: manual, freshness: { status: "uncertain", reason: "missing_or_untrusted_time" } }]} />);
  expect(screen.getByText(/Manual capture.*age uncertain/)).toBeInTheDocument();
  expect(screen.getByText("Source: Manual entry.")).toBeInTheDocument();
  expect(screen.getByText("Restricted (hidden by the game)")).toBeInTheDocument();
  expect(screen.getByText("Not applicable")).toBeInTheDocument();
  expect(screen.getByText("0 recorded entries; partial enumeration, other entries unknown")).toBeInTheDocument();
  rerender(<ProgressionRecords origin="Imported" records={[{ snapshot: { ...base, capture: { ...base.capture, source: "addon_snapshot" } }, freshness: { status: "fresh", age_ms: 0 } }]} />);
  expect(screen.getByText("Source: Addon observation.")).toBeInTheDocument();
});

it.each(["profile", "snapshot"])("refuses a loaded view for a different %s", async (mismatch) => {
  const h = harness(); h.get.mockResolvedValue(mismatch === "profile" ? { ...view, profile_id: "other" } : { ...view, binding: { ...view.binding, snapshot: "other" } });
  render(<ProgressionInputs {...props(h)} />); await open();
  expect(screen.getByRole("alert")).toHaveTextContent("SNAPSHOT_OUTDATED");
  expect(screen.getByLabelText("Money (copper)")).toBeDisabled();
  expect(screen.queryByRole("region", { name: "Manual progression observations" })).not.toBeInTheDocument();
});

it("recovers read failures and preserves manual inputs through save failures", async () => {
  const h = harness(); h.get.mockRejectedValueOnce(unavailable);
  h.save.mockRejectedValueOnce({ code: "INVALID_INPUT", message: "Capture rejected", retryable: false });
  render(<ProgressionInputs {...props(h)} />); await open();
  expect(screen.getByRole("alert")).toHaveTextContent("PROVIDER_UNAVAILABLE");
  fireEvent.click(screen.getByRole("button", { name: "Refresh progression observations" }));
  await waitFor(() => expect(screen.getByLabelText("Money (copper)")).toBeEnabled());
  fireEvent.change(screen.getByLabelText("Money (copper)"), { target: { value: "-1" } });
  expect(screen.getByRole("button", { name: "Save manual entry" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Money (copper)"), { target: { value: "250" } });
  fireEvent.click(screen.getByRole("button", { name: "Save manual entry" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Capture rejected");
  expect(screen.getByLabelText("Money (copper)")).toHaveValue("250");
  fireEvent.click(screen.getByRole("button", { name: "Save manual entry" }));
  expect(await screen.findByText(/Saved progression capture/)).toBeInTheDocument();
});

it.each(["stale", "wrong receipt"])("invalidates the binding on %s during a write", async (kind) => {
  const h = harness();
  if (kind === "stale") h.save.mockRejectedValue({ code: "SNAPSHOT_OUTDATED", message: "Changed snapshot", retryable: false });
  else h.save.mockResolvedValue({ ...receipt, snapshot_id: "different" });
  render(<ProgressionInputs {...props(h)} />); await open();
  fireEvent.change(screen.getByLabelText("Available talent points"), { target: { value: "1" } });
  fireEvent.click(screen.getByRole("button", { name: "Save manual entry" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("SNAPSHOT_OUTDATED");
  expect(screen.getByLabelText("Available talent points")).toBeDisabled();
});

it.each(["client", "profile", "snapshot"])("resets entered data and ignores delayed reads when %s changes", async (kind) => {
  const h = harness(); const late = deferred<ProgressionView>(); h.get.mockReturnValueOnce(late.promise);
  const original = props(h); const { rerender } = render(<ProgressionInputs {...original} />);
  fireEvent.click(screen.getByRole("button", { name: "Open progression inputs" }));
  const next = kind === "client" ? { ...original, client: harness().client } : kind === "profile" ? { ...original, profileId: "other" } : { ...original, snapshotId: "other" };
  rerender(<ProgressionInputs {...next} />);
  await act(async () => { late.resolve(view); });
  expect(screen.getByRole("button", { name: "Open progression inputs" })).toBeInTheDocument();
  expect(screen.queryByLabelText("Money (copper)")).not.toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("ignores delayed write failures after closing and resets drafts when reopening", async () => {
  const h = harness(); const late = deferred<ProgressionReceipt>(); h.save.mockReturnValueOnce(late.promise);
  render(<ProgressionInputs {...props(h)} />); await open();
  fireEvent.change(screen.getByLabelText("Money (copper)"), { target: { value: "10" } });
  fireEvent.click(screen.getByRole("button", { name: "Save manual entry" }));
  expect(screen.getByLabelText("Money (copper)")).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Close progression inputs" }));
  await act(async () => { late.reject(unavailable); });
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  await open(); expect(screen.getByLabelText("Money (copper)")).toHaveValue("");
});

it("disables mutations during parent changes and refreshes ages without replacing drafts", async () => {
  vi.useFakeTimers({ toFake: ["setInterval", "clearInterval"] });
  try {
    const h = harness(); const original = props(h); const { rerender, unmount } = render(<ProgressionInputs {...original} />); await open();
    fireEvent.change(screen.getByLabelText("Money (copper)"), { target: { value: "10" } });
    rerender(<ProgressionInputs {...original} disabled />);
    expect(screen.getByLabelText("Money (copper)")).toBeDisabled();
    rerender(<ProgressionInputs {...original} />);
    h.get.mockResolvedValue({ ...view, manual: [{ snapshot: fixture.payload as ProgressionView["manual"][number]["snapshot"], freshness: { status: "stale", age_ms: 3600000 } }] });
    await act(async () => { await vi.advanceTimersByTimeAsync(60000); });
    expect(screen.getByLabelText("Money (copper)")).toHaveValue("10");
    expect(within(screen.getByRole("region", { name: "Manual progression observations" })).getByText(/stale/)).toBeInTheDocument();
    const pending = deferred<ProgressionReceipt>(); h.save.mockReturnValueOnce(pending.promise);
    fireEvent.click(screen.getByRole("button", { name: "Save manual entry" }));
    const reads = h.get.mock.calls.length;
    await act(async () => { await vi.advanceTimersByTimeAsync(60000); });
    expect(h.get).toHaveBeenCalledTimes(reads);
    unmount(); await act(async () => { pending.resolve(receipt); });
    await vi.advanceTimersByTimeAsync(60000); expect(h.get).toHaveBeenCalledTimes(reads);
  } finally { vi.useRealTimers(); }
});
