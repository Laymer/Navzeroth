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
