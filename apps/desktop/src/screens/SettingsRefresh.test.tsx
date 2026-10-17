import { act, fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { createClient, type CommandAdapter } from "../commands";
import { defaultSettings } from "../commands/today";
import { SettingsScreen } from "./SettingsScreen";

function adapter(overrides: Record<string, (input: unknown) => unknown>): CommandAdapter {
  return { invoke(command, input) { const call = overrides[command]; if (!call) return Promise.reject(new Error("unexpected")); return Promise.resolve(call(input)) as never; } };
}

it("the refresh tick keeps unsaved edits and the revision they were made against", async () => {
  let revision = 1;
  const reads = vi.fn();
  const saved = vi.fn((input: unknown) => ({ revision: revision + 1, settings: JSON.parse((input as { text: string }).text).settings }));
  const client = createClient(adapter({
    get_app_settings: () => { reads(); return { revision, settings: { ...defaultSettings(), selected_installation: revision === 1 ? null : "changed-elsewhere" } }; },
    list_profiles: () => [], list_parties: () => [], update_app_settings: saved,
  }));
  const ui = render(<SettingsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  const field = await screen.findByLabelText("Manual goal identifier");
  fireEvent.change(field, { target: { value: "unsaved-goal" } });
  revision = 2;
  ui.rerender(<SettingsScreen client={client} refreshKey={1} onChanged={() => {}} />);
  await act(async () => { await Promise.resolve(); });
  expect(reads).toHaveBeenCalledTimes(2);
  expect(screen.getByLabelText("Manual goal identifier")).toHaveValue("unsaved-goal");
  expect(screen.queryByDisplayValue("changed-elsewhere")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Save local settings" }));
  await screen.findByText(/Local settings saved/);
  expect(JSON.parse((saved.mock.calls[0]?.[0] as { text: string }).text).expected_revision).toBe(1);
});

it("the refresh tick replaces a clean draft with the newer stored value", async () => {
  let installation: string | null = null;
  const client = createClient(adapter({
    get_app_settings: () => ({ revision: 1, settings: { ...defaultSettings(), selected_installation: installation } }),
    list_profiles: () => [], list_parties: () => [],
  }));
  const ui = render(<SettingsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  await screen.findByLabelText("Chosen game installation path");
  installation = "/newer";
  ui.rerender(<SettingsScreen client={client} refreshKey={1} onChanged={() => {}} />);
  expect(await screen.findByDisplayValue("/newer")).toBeVisible();
});

/** A store that enforces the revision check the real backend enforces. */
function store(initial = defaultSettings()) {
  const state = { revision: 1, settings: initial, saves: [] as number[], hold: null as null | (() => void) };
  const client = createClient(adapter({
    get_app_settings: () => ({ revision: state.revision, settings: state.settings }),
    list_profiles: () => [], list_parties: () => [],
    update_app_settings: async (input: unknown) => {
      const request = JSON.parse((input as { text: string }).text);
      state.saves.push(request.expected_revision);
      if (state.hold) await new Promise<void>((r) => { state.hold = r; });
      if (request.expected_revision !== state.revision) throw { code: "INVALID_INPUT", message: "Stale settings", retryable: false };
      state.revision += 1;
      state.settings = request.settings;
      return { revision: state.revision, settings: state.settings };
    },
  }));
  return { state, client };
}
const tick = async (ui: ReturnType<typeof render>, client: ReturnType<typeof createClient>, key: number) => {
  ui.rerender(<SettingsScreen client={client} refreshKey={key} onChanged={() => {}} />);
  await act(async () => { await Promise.resolve(); await Promise.resolve(); });
};

it("a stale save is rejected, the conflict stays through later ticks, and Keep my edits recovers", async () => {
  const { state, client } = store();
  const ui = render(<SettingsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  fireEvent.change(await screen.findByLabelText("Manual goal identifier"), { target: { value: "mine" } });
  state.revision = 2; state.settings = { ...defaultSettings(), selected_installation: "elsewhere" };
  fireEvent.click(screen.getByRole("button", { name: "Save local settings" }));
  expect(await screen.findByText("Stale settings")).toBeVisible();
  expect(await screen.findByText(/Settings changed elsewhere/)).toBeVisible();
  await tick(ui, client, 1); await tick(ui, client, 2);
  expect(screen.getByText("Stale settings")).toBeVisible();
  expect(screen.getByText(/Settings changed elsewhere/)).toBeVisible();
  expect(screen.getByLabelText("Manual goal identifier")).toHaveValue("mine");
  fireEvent.click(screen.getByRole("button", { name: "Keep my edits and save over the newer settings" }));
  fireEvent.click(screen.getByRole("button", { name: "Save local settings" }));
  await screen.findByText(/Local settings saved/);
  expect(state.saves).toEqual([1, 2]);
  expect(state.settings.manual_planning_defaults.goal_profile).toBe("mine");
});

it("Discard my edits takes the stored values and clears the conflict", async () => {
  const { state, client } = store();
  const ui = render(<SettingsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  fireEvent.change(await screen.findByLabelText("Manual goal identifier"), { target: { value: "mine" } });
  state.revision = 2; state.settings = { ...defaultSettings(), selected_installation: "elsewhere" };
  await tick(ui, client, 1);
  expect(screen.getByText(/Settings changed elsewhere/)).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Discard my edits and load stored settings" }));
  expect(screen.getByDisplayValue("elsewhere")).toBeVisible();
  expect(screen.getByLabelText("Manual goal identifier")).toHaveValue("");
  expect(screen.queryByText(/Settings changed elsewhere/)).toBeNull();
  expect(state.saves).toEqual([]);
});

it("a refresh tick during an in-flight save does not drop the save result", async () => {
  const { state, client } = store();
  const onChanged = vi.fn();
  const ui = render(<SettingsScreen client={client} refreshKey={0} onChanged={onChanged} />);
  fireEvent.change(await screen.findByLabelText("Manual goal identifier"), { target: { value: "mine" } });
  state.hold = () => {};
  fireEvent.click(screen.getByRole("button", { name: "Save local settings" }));
  await act(async () => { await Promise.resolve(); });
  await tick(ui, client, 1);
  const release = state.hold as () => void; state.hold = null;
  await act(async () => { release(); await Promise.resolve(); });
  await screen.findByText(/Local settings saved/);
  expect(onChanged).toHaveBeenCalledTimes(1);
  await tick(ui, client, 2);
  fireEvent.change(screen.getByLabelText("Manual goal identifier"), { target: { value: "again" } });
  fireEvent.click(screen.getByRole("button", { name: "Save local settings" }));
  await screen.findByText(/Local settings saved/);
  expect(state.saves).toEqual([1, 2]);
});

it("typing back to the stored value makes the draft clean again", async () => {
  const { state, client } = store();
  const ui = render(<SettingsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  const field = await screen.findByLabelText("Manual goal identifier");
  fireEvent.change(field, { target: { value: "x" } });
  fireEvent.change(field, { target: { value: "" } });
  state.revision = 2; state.settings = { ...defaultSettings(), selected_installation: "elsewhere" };
  await tick(ui, client, 1);
  expect(screen.getByDisplayValue("elsewhere")).toBeVisible();
  expect(screen.queryByText(/Settings changed elsewhere/)).toBeNull();
});

it("switching client while dirty shows nothing from the old client", async () => {
  const a = store({ ...defaultSettings(), selected_installation: "a-path" });
  const b = store({ ...defaultSettings(), selected_installation: "b-path" });
  const ui = render(<SettingsScreen client={a.client} refreshKey={0} onChanged={() => {}} />);
  fireEvent.change(await screen.findByLabelText("Manual goal identifier"), { target: { value: "dirty-a" } });
  ui.rerender(<SettingsScreen client={b.client} refreshKey={0} onChanged={() => {}} />);
  expect(await screen.findByDisplayValue("b-path")).toBeVisible();
  expect(screen.queryByDisplayValue("dirty-a")).toBeNull();
  expect(screen.getByLabelText("Manual goal identifier")).toHaveValue("");
});
