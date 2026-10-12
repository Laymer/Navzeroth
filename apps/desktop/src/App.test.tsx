import { render, screen } from "@testing-library/react";
import { App } from "./App";
import { createClient, type CommandAdapter } from "./commands";
import { demoAdapter } from "./commands/demoAdapter";
import { toAppError } from "./commands/client";
import { describeObservation } from "./observation";

describe("demo mode", () => {
  it("shows the Demo label and the synthetic snapshot", async () => {
    render(<App client={createClient(demoAdapter)} />);
    expect(await screen.findByText(/DEMO — synthetic data/)).toBeInTheDocument();
    expect(screen.getByText("demo-snapshot-001")).toBeInTheDocument();
    expect(screen.getByText("22")).toBeInTheDocument();
    // role is "unknown" in the fixture and must not render as a value
    expect(screen.getByText("Unknown (not observed)")).toBeInTheDocument();
  });

  it("rejects an unknown profile with a structured error", async () => {
    const r = await createClient(demoAdapter).getCharacterSnapshot("nope");
    expect(r).toEqual({ ok: false, error: { code: "INVALID_INPUT", message: "No such profile.", retryable: false } });
  });
});

describe("non-demo and failure states", () => {
  const stored: CommandAdapter = {
    invoke: (c) =>
      c === "get_app_status"
        ? Promise.resolve({ version: "0.1.0", data_mode: "stored", active_profile: null, capabilities: {} } as never)
        : Promise.reject(new Error("unexpected")),
  };
  it("does not label stored data as demo and reports an empty state", async () => {
    render(<App client={createClient(stored)} />);
    expect(await screen.findByText("No character snapshot has been imported.")).toBeInTheDocument();
    expect(screen.queryByText(/DEMO/)).not.toBeInTheDocument();
  });

  it("surfaces backend failures without leaking raw error text", async () => {
    const failing: CommandAdapter = { invoke: () => Promise.reject(new Error("secret path /home/x")) };
    render(<App client={createClient(failing)} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("INTERNAL");
    expect(screen.getByRole("alert")).not.toHaveTextContent("secret");
  });
});

describe("helpers", () => {
  it("keeps observation states distinct", () => {
    expect(describeObservation({ status: "known", value: 0 })).toBe("0");
    const labels = (["unknown", "unsupported", "restricted", "not_applicable"] as const).map((status) =>
      describeObservation({ status }),
    );
    expect(new Set(labels).size).toBe(4);
  });
  it("normalizes malformed errors", () => {
    expect(toAppError({ code: "BOGUS", message: "x" }).code).toBe("INTERNAL");
    expect(toAppError({ code: "STORAGE_ERROR", message: "m", retryable: true })).toEqual({
      code: "STORAGE_ERROR", message: "m", retryable: true,
    });
  });
});

it("labels stored synthetic snapshots and uncertain age explicitly", async () => {
  const snapshot = (await createClient(demoAdapter).getCharacterSnapshot("demo-character-paladin"));
  if (!snapshot.ok) throw new Error("Expected demo fixture");
  const adapter: CommandAdapter = {
    invoke: (command) => {
      if (command === "get_app_status") return Promise.resolve({ version: "0.1.0", data_mode: "stored", active_profile: "stored-full-context", capabilities: {} } as never);
      if (command === "get_stored_snapshot") return Promise.resolve({ envelope: snapshot.value, profile_id: "stored-full-context", data_mode: "stored", freshness: { status: "uncertain", reason: "missing_or_untrusted_time" } } as never);
      return Promise.reject(new Error("Unexpected command"));
    },
  };
  render(<App client={createClient(adapter)} />);
  expect(await screen.findByText(/SYNTHETIC — fixture or demo data/)).toBeInTheDocument();
  expect(screen.getByText(/Last imported:.*uncertain/)).toBeInTheDocument();
  expect(screen.getByText("demo-snapshot-001")).toBeInTheDocument();
});
