import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { WorldScreen, describeItem, describeLevels, describePlace } from "./WorldScreen";
import type { EntityRow, LootTableView, Offer, QuestRow, SourceRow, WorldPage, WorldView } from "../commands/world";
import pages from "../../../../fixtures/desktop/world/pages.json";

const golden = pages as unknown as Record<"entities" | "entities_hog" | "vendors" | "trainers" | "quests" | "loot", WorldPage>;
type Reply = { ok: true; value: WorldPage } | { ok: false; error: { code: "INVALID_INPUT"; message: string; retryable: false } };
function client(reply?: (request: { view: WorldView; query: string; offset: number; revision?: string }) => Reply) {
  return {
    listProfiles: vi.fn(async () => ({ ok: true as const, value: [{ id: "p", display_name: "Synthetic profile", data_mode: "stored" as const }] })),
    getWorldPage: vi.fn(async (text: string) => {
      const request = JSON.parse(text);
      if (reply) return reply(request);
      const page = request.view === "entities" && request.query ? golden.entities_hog : golden[request.view as WorldView];
      return { ok: true as const, value: page };
    }),
  };
}
const heading = (name: string) => screen.findByRole("heading", { level: 3, name: new RegExp(`^${name}`) });

it("searches on the backend and marks Hogger as 1.12 data with his map position", async () => {
  const mock = client();
  render(<WorldScreen client={mock as never} refreshKey={0} />);
  expect(await heading("Synthetic Chest")).toBeInTheDocument();
  expect(screen.getByRole("note")).toHaveTextContent("SYNTHETIC");
  expect(screen.getByRole("status")).toHaveTextContent("NPCs and objects: 1–4 of 4.");
  expect(screen.getByRole("button", { name: "Trainers (1)" })).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Search NPCs and objects"), { target: { value: "hog" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  const hogger = await heading("Hogger");
  expect(hogger).toHaveTextContent(/^Hogger.*From 1\.12 data$/);
  expect(screen.getByText("map demo-map-1, floor 0, at 27, 87 (node world-node-hogger)")).toBeInTheDocument();
  expect(screen.getByRole("status")).toHaveTextContent("1–1 of 1 matching “hog”");
  expect(JSON.parse(mock.getWorldPage.mock.calls.at(-1)?.[0] ?? "")).toEqual({ subject: { kind: "character", profile_id: "p" }, view: "entities", query: "hog", offset: 0 });
  fireEvent.click(screen.getByRole("button", { name: "Clear search" }));
  expect(await heading("Synthetic Chest")).toBeInTheDocument();
});

it("lists vendor items with cost, loot with chance and quests with objectives, each with origins", async () => {
  render(<WorldScreen client={client() as never} refreshKey={0} />);
  await heading("Hogger");
  fireEvent.click(screen.getByRole("button", { name: "Vendors (1)" }));
  const vendor = (await heading("Synthetic Quartermaster")).closest("li") as HTMLElement;
  expect(within(vendor).getByText(/Synthetic Sword \(item demo-item-1, base\); cost 250 copper; at world-node-vendor/)).toHaveTextContent("From Classic Era data");
  expect(within(vendor).getByText("vendor, repair")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Loot (2)" }));
  const loot = (await heading("Hogger")).closest("li") as HTMLElement;
  expect(within(loot).getByText(/chance 25%/)).toHaveTextContent("From 1.12 data");
  fireEvent.click(screen.getByRole("button", { name: "Trainers (1)" }));
  expect((await heading("Synthetic Trainer"))).toHaveTextContent("From Classic Era data");
  fireEvent.click(screen.getByRole("button", { name: "Quests (1)" }));
  const quest = (await heading("Synthetic Quest")).closest("li") as HTMLElement;
  expect(quest.querySelector("h3")).toHaveTextContent("From 1.12 data");
  expect(quest.querySelector("h3")).toHaveTextContent(/^Synthetic Quest/);
  expect(within(quest).getByText("Quest demo-quest-1.")).toBeInTheDocument();
  expect(within(quest).getByText(/Given by: Synthetic Quartermaster \(npc:1198\); Synthetic Sword \(item:demo-item-1\); object:404 \(not described by this pack\)\./)).toBeInTheDocument();
  expect(within(quest).getByText("Turned in to: Hogger (npc:448).")).toBeInTheDocument();
  expect(within(quest).getByText("Completion: unrepeatable.")).toBeInTheDocument();
  expect(within(quest).getByText("demo-objective-1.")).toBeInTheDocument();
  expect(within(quest).getByText(/demo-objective-1: quest stage demo-objective-1; obtain from demo-acq-1; visit demo-node-1/)).toBeInTheDocument();
  expect(within(quest).getByText("Synthetic Sword (item demo-item-1, base)")).toBeInTheDocument();
  expect(within(quest).getByText(/cost Not applicable/)).toHaveTextContent("From 1.12 data");
  expect(screen.getByRole("button", { name: "Quests (1)" })).toHaveAttribute("aria-pressed", "true");
});

const base = golden.entities;
const entity = base.rows[0] as EntityRow;
const offer = (golden.vendors.rows[0] as SourceRow).offers[0] as Offer;
it("pages by offset with the bound revision and keeps unknowns, partial lists and gaps explicit", async () => {
  const many: EntityRow = { ...entity, name: "Gnoll", places_total: 3, levels: { status: "known", value: [5, 7] }, services: ["flight", "other"] };
  const first: WorldPage = { ...base, view: "entities", total: 120, next_offset: 50, rows: [many], counts: { ...base.counts, trainers: 0 } };
  const second: WorldPage = { ...first, offset: 50, next_offset: 100 };
  const mock = client((r) => ({ ok: true, value: r.offset === 50 ? second : first }));
  render(<WorldScreen client={mock as never} refreshKey={0} />);
  const gnoll = (await heading("Gnoll")).closest("li") as HTMLElement;
  expect(within(gnoll).getByText("5–7")).toBeInTheDocument();
  expect(within(gnoll).getByText("flight master, other service")).toBeInTheDocument();
  expect(within(gnoll).getByText("2 more places are not listed here.")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: /Trainers/ })).toBeNull();
  expect(screen.getByText("This content pack has no trainers.")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Previous page" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Next page" }));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("51–51 of 120"));
  expect(JSON.parse(mock.getWorldPage.mock.calls.at(-1)?.[0] ?? "")).toMatchObject({ offset: 50, revision: "demo-r1" });
  fireEvent.click(screen.getByRole("button", { name: "Previous page" }));
  await waitFor(() => expect(JSON.parse(mock.getWorldPage.mock.calls.at(-1)?.[0] ?? "")).toMatchObject({ offset: 0, revision: "demo-r1" }));
});

it("describes unresolved sources, alternative costs and missing objective details", async () => {
  const costs: Offer = { ...offer, items: [], cost_item_names: { token: "Synthetic Token" }, cost: { status: "unknown" }, costs: { status: "known", value: [{ kind: "copper", amount: "5" }, { kind: "item", item: { item_id: "token", variant: "base" }, quantity: 2 }, { kind: "currency", currency: "honor", quantity: 3 }] } };
  const free: Offer = { ...offer, acquisition: "free", costs: { status: "known", value: [] }, cost: { status: "unknown" } };
  const unknown: Offer = { ...offer, acquisition: "unknown", cost: { status: "restricted" }, costs: { status: "unknown" }, dungeon: "deadmines", location: null };
  const source: SourceRow = { row: "source", id: "npc:404", entity: null, offers: [costs, free, unknown], offers_total: 4, tables: [], tables_total: 0 };
  const empty: SourceRow = { ...source, id: "npc:405", offers: [], offers_total: 0 };
  const quest: QuestRow = { ...(golden.quests.rows[0] as QuestRow), id: "bare-quest", objectives: [], stages: [], rewards: { status: "unknown" }, offers: [], origin: [] };
  const mock = client((r) => ({ ok: true, value: { ...base, view: r.view, rows: r.view === "quests" ? [quest] : r.view === "loot" ? [{ ...empty, id: "npc:406" }] : [source, empty] } }));
  render(<WorldScreen client={mock as never} refreshKey={0} />);
  await heading("Hogger").catch(() => undefined);
  fireEvent.click(await screen.findByRole("button", { name: /^Vendors/ }));
  expect(await screen.findByText("This pack names npc:404 but does not describe it.")).toBeInTheDocument();
  expect(screen.getByText(/source world-acq-vendor \(items unresolved\); cost 5 copper, 2 × Synthetic Token \(item token\), 3 × honor/)).toBeInTheDocument();
  expect(screen.getByText(/cost free/)).toBeInTheDocument();
  expect(screen.getByText(/cost Restricted \(hidden by the game\); in deadmines/)).toBeInTheDocument();
  expect(screen.getByText("1 more entries are not listed here.")).toBeInTheDocument();
  expect(screen.getByText("No items recorded for this vendor.")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /^Loot/ }));
  expect(await screen.findByText("No rewards recorded.")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /^Quests/ }));
  expect(await screen.findByText("No objective details recorded.")).toBeInTheDocument();
  expect(screen.getByText("none recorded.")).toBeInTheDocument();
  expect(screen.getByText("Unknown (not observed)")).toBeInTheDocument();
});

it("reports errors, empty results, absent content and schema drift without stale rows", async () => {
  let reply: Reply = { ok: false, error: { code: "INVALID_INPUT", message: "World data changed; search again", retryable: false } };
  const mock = client(() => reply);
  const view = render(<WorldScreen client={mock as never} refreshKey={0} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("World data changed; search again");
  reply = { ok: true, value: { ...base, revision: null, total: 0, rows: [], fixture: false } };
  view.rerender(<WorldScreen client={mock as never} refreshKey={1} />);
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("No active content for this profile. Activate content first."));
  reply = { ok: true, value: { ...base, total: 0, rows: [] } };
  fireEvent.change(screen.getByLabelText("Search NPCs and objects"), { target: { value: "nobody" } });
  fireEvent.click(screen.getByRole("button", { name: "Search" }));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("No NPCs and objects match “nobody”."));
  reply = { ok: true, value: { ...base, view: "loot" } };
  view.rerender(<WorldScreen client={mock as never} refreshKey={2} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("World page changed; search again.");
  reply = { ok: true, value: { ...base, total: 0, rows: [] } };
  fireEvent.click(screen.getByRole("button", { name: "Clear search" }));
  await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("No NPCs and objects match."));
});

it("refuses to browse without profiles and ignores late responses from an old view", async () => {
  const none = { listProfiles: vi.fn(async () => ({ ok: false as const, error: { code: "INTERNAL" as const, message: "Profiles unavailable", retryable: false } })), getWorldPage: vi.fn() };
  const view = render(<WorldScreen client={none as never} refreshKey={0} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("Profiles unavailable");
  expect(none.getWorldPage).not.toHaveBeenCalled();
  view.unmount();
  let release: ((r: Reply) => void) | undefined;
  const slow = client();
  slow.getWorldPage.mockImplementationOnce(() => new Promise((resolve) => { release = resolve as never; }) as never);
  render(<WorldScreen client={slow as never} refreshKey={0} />);
  await waitFor(() => expect(release).toBeDefined());
  fireEvent.click(await screen.findByRole("button", { name: "Loot" }));
  await act(async () => { release?.({ ok: true, value: golden.entities }); });
  expect(await heading("Hogger")).toBeInTheDocument();
  expect(screen.queryByText("Synthetic Chest")).toBeNull();
});

it("formats levels and places for screen readers", () => {
  expect(describeLevels({ status: "known", value: [11, 11] })).toBe("11");
  expect(describeLevels({ status: "unsupported" })).toBe("Unsupported by this client");
  expect(describePlace({ map_id: "m", floor: "1", x: 0.123, y: 1 })).toBe("map m, floor 1, at 12.3, 100");
});

it("shows loot templates with unknown chances, groups, quest-only rows and reference tables", async () => {
  render(<WorldScreen client={client() as never} refreshKey={0} />);
  await heading("Hogger");
  fireEvent.click(screen.getByRole("button", { name: "Loot (2)" }));
  const hogger = (await heading("Hogger")).closest("li") as HTMLElement;
  const summary = within(hogger).getByText(/creature loot loot:creature:448 \(4 rows\)/);
  expect(summary).toHaveTextContent("From 1.12 data");
  const rows = within(hogger).getAllByRole("listitem").map((li) => li.textContent);
  expect(rows).toContain("Synthetic Sword (item demo-item-1); count 1; chance 40%");
  expect(rows).toContain("item 9001; count 1–2; chance unknown; group 1; only while a quest needs it");
  expect(rows).toContain("item 9002; count 1; chance 0%; has a condition this app cannot express");
  expect(rows).toContain("table loot:reference:9; count 1–3; chance 5%");
  const reference = (await heading("loot:reference:9")).closest("li") as HTMLElement;
  expect(within(reference).getByText("This pack names loot:reference:9 but does not describe it.")).toBeInTheDocument();
  expect(within(reference).getByText(/shared reference table loot:reference:9/)).toBeInTheDocument();
});

it("names bosses, caps loot tables and explains quest links the pack does not record", async () => {
  const boss: EntityRow = { ...entity, boss_of: [{ id: "dungeon:the-deadmines", title: "The Deadmines" }] };
  const table: LootTableView = { id: "loot:creature:1", kind: "creature", origin: [], rows_total: 45, rows: [{ item: null, table: "loot:reference:1", probability: null, min_count: 1, max_count: 1, group: null, quest_only: false, conditional: false }] };
  const loot: SourceRow = { row: "source", id: "npc:1", entity: (() => { const rest: Partial<EntityRow> = { ...boss }; delete rest.row; return rest as Omit<EntityRow, "row">; })(), offers: [], offers_total: 0, tables: [table], tables_total: 8 };
  const quest: QuestRow = { ...(golden.quests.rows[0] as QuestRow), givers: [], givers_total: 0, enders: [{ id: "npc:2" }], enders_total: 3 };
  delete quest.title;
  const mock = client((r) => ({ ok: true, value: { ...base, view: r.view, rows: r.view === "quests" ? [quest] : r.view === "loot" ? [loot] : [boss] } }));
  render(<WorldScreen client={mock as never} refreshKey={0} />);
  expect(await screen.findByText("The Deadmines")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /^Loot/ }));
  expect(await screen.findByText("44 more rows are not listed here.")).toBeInTheDocument();
  expect(screen.getByText("7 more loot tables are not listed here.")).toBeInTheDocument();
  expect(screen.getByText(/table loot:reference:1; count 1; chance unknown/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: /^Quests/ }));
  expect(await screen.findByText("Given by: not recorded.")).toBeInTheDocument();
  expect(screen.getByText("Turned in to: npc:2 (not described by this pack); 2 more not listed here.")).toBeInTheDocument();
  expect(screen.getByRole("heading", { level: 3, name: /^demo-quest-1/ })).toBeInTheDocument();
});

it("describes items by name with the id and variant only when known", () => {
  expect(describeItem({ item_id: "1", variant: "base", name: "Sword" })).toBe("Sword (item 1, base)");
  expect(describeItem({ item_id: "1", name: "Sword" })).toBe("Sword (item 1)");
  expect(describeItem({ item_id: "1", variant: "base" })).toBe("item 1 (base)");
  expect(describeItem({ item_id: "1" })).toBe("item 1");
});
