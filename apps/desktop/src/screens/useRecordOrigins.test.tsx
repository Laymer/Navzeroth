import { render, screen, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { createClient, type CommandAdapter } from "../commands";
import { originKey, type OriginClient, type OriginRef } from "../commands/world";
import type { CatalogSubject } from "../commands/catalogPages";
import { ORIGIN_BATCH, useRecordOrigins } from "./useRecordOrigins";
import pages from "../../../../fixtures/desktop/world/pages.json";

const subject: CatalogSubject = { kind: "character", profile_id: "p" };
const refs: OriginRef[] = [
  { collection: "items", id: "demo-item-1", variant: "base" },
  { collection: "edges", id: "world-edge-inferred" },
  { collection: "quests", id: "demo-quest-1" },
];
function Probe({ client, revision = "demo-r1", records = refs }: { client?: Partial<OriginClient> | undefined; revision?: string | null; records?: OriginRef[] }) {
  const origins = useRecordOrigins(client, subject, revision, records);
  return <ul>{records.map((r, i) => <li key={`${i}-${originKey(r)}`}>{r.id}: {origins.get(originKey(r))?.join(",") ?? "none"}</li>)}</ul>;
}
it("binds one deduplicated lookup to the revision and maps marks by record", async () => {
  const getRecordOrigins = vi.fn(async (text: string) => { void text; return { ok: true as const, value: pages.origins as { revision: string; origins: (("vanilla_1_12" | "wow_classic_era" | "inferred")[] | null)[] } }; });
  render(<Probe client={{ getRecordOrigins }} records={[...refs, refs[0] as OriginRef]} />);
  expect(await screen.findByText("world-edge-inferred: inferred")).toBeInTheDocument();
  expect(screen.getByText("demo-quest-1: vanilla_1_12")).toBeInTheDocument();
  expect(screen.getAllByText(/^demo-item-1:\s*$/)).toHaveLength(2);
  expect(getRecordOrigins).toHaveBeenCalledOnce();
  expect(JSON.parse(getRecordOrigins.mock.calls[0]?.[0] ?? "")).toEqual({ subject, revision: "demo-r1", records: refs });
});
it("shows no markers for failures, stale revisions, missing clients or revisions", async () => {
  const failing = vi.fn(async () => ({ ok: false as const, error: { code: "INVALID_INPUT" as const, message: "no", retryable: false } }));
  const stale = vi.fn(async () => ({ ok: true as const, value: { ...pages.origins, revision: "other" } as never }));
  const nulls = vi.fn(async () => ({ ok: true as const, value: { revision: "demo-r1", origins: [null, null, null] } }));
  for (const lookup of [failing, stale, nulls]) {
    const view = render(<Probe client={{ getRecordOrigins: lookup }} />);
    await waitFor(() => expect(lookup).toHaveBeenCalled());
    expect(screen.queryByText(/: (inferred|vanilla)/)).toBeNull();
    view.unmount();
  }
  const unused = vi.fn();
  render(<Probe client={{ getRecordOrigins: unused }} revision={null} />);
  render(<Probe client={undefined} />);
  render(<Probe client={{ getRecordOrigins: unused }} records={[]} />);
  expect(unused).not.toHaveBeenCalled();
});
it("caps one lookup at a catalog page and goes through the typed command", async () => {
  const invoke = vi.fn(async (command: string, input: unknown) => {
    expect(command).toBe("get_record_origins");
    return { revision: "demo-r1", origins: JSON.parse((input as { text: string }).text).records.map(() => []) };
  });
  const client = createClient({ invoke } as unknown as CommandAdapter);
  const many: OriginRef[] = Array.from({ length: ORIGIN_BATCH + 5 }, (_, i) => ({ collection: "edges", id: `edge-${i}` }));
  render(<Probe client={client} records={many} />);
  await waitFor(() => expect(invoke).toHaveBeenCalledOnce());
  expect(JSON.parse((invoke.mock.calls[0]?.[1] as { text: string }).text).records).toHaveLength(ORIGIN_BATCH);
});
