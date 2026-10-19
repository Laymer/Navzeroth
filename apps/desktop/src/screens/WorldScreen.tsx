import "./play.css";
import "./world.css";
import { useEffect, useRef, useState } from "react";
import { Button, Field, OriginMarker } from "../ui";
import type { ProfileSummary } from "../commands/contract";
import type { EntityLink, EntityRow, LootRowView, LootTableView, Observed, Offer, QuestRow, SourceRow, TargetRow, WorldClient, WorldPage, WorldView } from "../commands/world";
import { describeObservation } from "../observation";

/** View tabs in display order; Trainers is hidden when the pack has no trainers. */
export const WORLD_VIEWS: { id: WorldView; label: string; noun: string }[] = [
  { id: "entities", label: "NPCs and objects", noun: "NPCs and objects" },
  { id: "vendors", label: "Vendors", noun: "vendors" },
  { id: "trainers", label: "Trainers", noun: "trainers" },
  { id: "quests", label: "Quests", noun: "quests" },
  { id: "loot", label: "Loot", noun: "loot sources" },
];
const SERVICE: Record<string, string> = { junction: "junction", vendor: "vendor", repair: "repair", trainer: "trainer", flight: "flight master", transport: "transport", dungeon: "dungeon", other: "other service" };
const percent = (value: number) => `${Math.round(value * 1000) / 10}`;
export const describeLevels = (levels: Observed<[number, number]>) => describeObservation(levels, ([lo, hi]) => lo === hi ? `${lo}` : `${lo}–${hi}`);
export function describePlace(place: { map_id: string; floor: string; x: number; y: number; node?: string }) {
  return `map ${place.map_id}, floor ${place.floor}, at ${percent(place.x)}, ${percent(place.y)}`;
}
/** An item by name when the pack records one, always with its id; `variant` only when known. */
export function describeItem(item: { item_id: string; variant?: string; name?: string }) {
  if (item.name) return `${item.name} (item ${item.item_id}${item.variant ? `, ${item.variant}` : ""})`;
  return item.variant ? `item ${item.item_id} (${item.variant})` : `item ${item.item_id}`;
}
const describeLink = (link: EntityLink) => link.name ? `${link.name} (${link.id})` : `${link.id} (not described by this pack)`;
function Links({ label, links, total }: { label: string; links: EntityLink[]; total: number }) {
  if (total === 0) return <p>{label}: not recorded.</p>;
  return <p>{label}: {links.map(describeLink).join("; ")}{total > links.length ? `; ${total - links.length} more not listed here` : ""}.</p>;
}
const LOOT_KIND: Record<string, string> = { creature: "creature loot", game_object: "object loot", item: "item contents", reference: "shared reference table", fishing: "fishing", pickpocketing: "pickpocketing", skinning: "skinning", disenchant: "disenchanting" };
function describeLootRow(row: LootRowView) {
  const what = row.item ? describeItem(row.item) : `table ${row.table ?? "unknown"}`;
  const count = row.min_count === row.max_count ? `${row.min_count}` : `${row.min_count}–${row.max_count}`;
  const chance = row.probability === null ? "chance unknown" : `chance ${percent(row.probability)}%`;
  return [what, row.table && row.item ? `table ${row.table}` : "", `count ${count}`, chance, row.group === null ? "" : `group ${row.group}`, row.quest_only ? "only while a quest needs it" : "", row.conditional ? "has a condition this app cannot express" : ""].filter(Boolean).join("; ");
}
function LootTables({ tables, total }: { tables: LootTableView[]; total: number }) {
  if (tables.length === 0) return null;
  return <>{tables.map((table) => <details key={table.id}><summary>{LOOT_KIND[table.kind] ?? table.kind} {table.id} ({table.rows_total} rows)<OriginMarker marks={table.origin} /></summary>
    <ul>{table.rows.map((row, n) => <li key={n}>{describeLootRow(row)}</li>)}</ul>
    <More shown={table.rows.length} total={table.rows_total} noun="rows" /></details>)}
    <More shown={tables.length} total={total} noun="loot tables" /></>;
}
function describeTarget(target: TargetRow) {
  if (target.kind === "quest_stage") return `quest stage ${target.stage}`;
  if (target.kind === "acquisition") return `obtain from ${target.source}`;
  return `visit ${target.node}`;
}
function describeCost(offer: Offer) {
  if (offer.cost.status === "known") return `${offer.cost.value} copper`;
  if (offer.costs.status === "known") return offer.costs.value.map((c) => c.kind === "copper" ? `${c.amount} copper` : c.kind === "item" ? `${c.quantity} × ${offer.cost_item_names[c.item.item_id] ? `${offer.cost_item_names[c.item.item_id]} (item ${c.item.item_id})` : `item ${c.item.item_id}`}` : `${c.quantity} × ${c.currency}`).join(", ") || "free";
  return describeObservation(offer.cost);
}
function More({ shown, total, noun }: { shown: number; total: number; noun: string }) {
  return total > shown ? <p>{total - shown} more {noun} are not listed here.</p> : null;
}
function OfferList({ offers, total, view }: { offers: Offer[]; total: number; view: WorldView }) {
  if (offers.length === 0) return <p>{view === "vendors" ? "No items recorded for this vendor." : "No rewards recorded."}</p>;
  return <><ul>{offers.map((offer) => <li key={offer.acquisition}>
    {offer.items.length ? offer.items.map(describeItem).join(", ") : `source ${offer.acquisition} (items unresolved)`}
    {view === "loot" ? `; chance ${describeObservation(offer.probability, (p) => `${percent(p)}%`)}` : `; cost ${describeCost(offer)}`}
    {offer.dungeon && `; in ${offer.dungeon}`}{offer.location && `; at ${offer.location}`}
    <OriginMarker marks={offer.origin} />
  </li>)}</ul><More shown={offers.length} total={total} noun="entries" /></>;
}
function EntityDetails({ entity }: { entity: Omit<EntityRow, "row"> }) {
  return <dl className="world-facts">
    <dt>Kind</dt><dd>{entity.kind === "npc" ? "NPC" : "Object"} ({entity.id})</dd>
    <dt>Subname</dt><dd>{describeObservation(entity.subname)}</dd>
    <dt>Levels</dt><dd>{describeLevels(entity.levels)}</dd>
    <dt>Reaction</dt><dd>{describeObservation(entity.reaction)}</dd>
    <dt>Services</dt><dd>{entity.services.length ? entity.services.map((s) => SERVICE[s] ?? s).join(", ") : "None recorded"}</dd>
    {entity.boss_of.length > 0 && <><dt>Boss of</dt><dd>{entity.boss_of.map((d) => d.title).join(", ")}</dd></>}
    <dt>Where</dt><dd>{entity.places.length === 0 ? "No mapped place in the travel graph" : <ul>{entity.places.map((p) => <li key={p.node}>{describePlace(p)} (node {p.node})</li>)}</ul>}
      <More shown={entity.places.length} total={entity.places_total} noun="places" /></dd>
  </dl>;
}
function EntityItem({ row }: { row: EntityRow }) {
  return <li className="world-row"><h3>{row.name}<OriginMarker marks={row.origin} /></h3><EntityDetails entity={row} /></li>;
}
function SourceItem({ row, view }: { row: SourceRow; view: WorldView }) {
  return <li className="world-row"><h3>{row.entity?.name ?? row.id}<OriginMarker marks={row.entity?.origin} /></h3>
    {row.entity ? <details><summary>About {row.entity.name}</summary><EntityDetails entity={row.entity} /></details> : <p>This pack names {row.id} but does not describe it.</p>}
    <h4>{view === "vendors" ? "Items for sale" : "Drops"}</h4>
    {(view !== "loot" || row.offers.length > 0 || row.tables.length === 0) && <OfferList offers={row.offers} total={row.offers_total} view={view} />}
    {view === "loot" && <LootTables tables={row.tables} total={row.tables_total} />}
  </li>;
}
function QuestItem({ row }: { row: QuestRow }) {
  return <li className="world-row"><h3>{row.title ?? row.id}<OriginMarker marks={row.origin} /></h3>
    {row.title && <p>Quest {row.id}.</p>}
    <Links label="Given by" links={row.givers} total={row.givers_total} /><Links label="Turned in to" links={row.enders} total={row.enders_total} />
    <p>Completion: {describeObservation(row.completion)}.</p>
    <h4>Stages</h4><p>{row.stages.length ? row.stages.join(", ") : "none recorded"}.</p>
    <h4>Objectives</h4>{row.objectives.length === 0 ? <p>No objective details recorded.</p>
      : <><ul>{row.objectives.map((o) => <li key={o.id}>{o.id}: {o.targets.map(describeTarget).join("; ")}<OriginMarker marks={o.origin} /></li>)}</ul><More shown={row.objectives.length} total={row.objectives_total} noun="objectives" /></>}
    <h4>Rewards</h4><p>{describeObservation(row.rewards, (items) => items.length ? items.map(describeItem).join(", ") : "No item rewards")}</p>
    {row.offers.length > 0 && <OfferList offers={row.offers} total={row.offers_total} view="quests" />}
  </li>;
}

type Load = { ticket: number; page: WorldPage } | null;
export function WorldScreen({ client, refreshKey }: { client: WorldClient; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const [view, setView] = useState<WorldView>("entities");
  const [draft, setDraft] = useState("");
  const [query, setQuery] = useState("");
  const [load, setLoad] = useState<Load>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const generation = useRef(0);
  useEffect(() => {
    let live = true;
    void client.listProfiles().then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      setProfiles(result.value); setProfile((old) => result.value.some((p) => p.id === old) ? old : result.value[0]?.id ?? "");
    });
    return () => { live = false; };
  }, [client, refreshKey]);
  async function fetchPage(offset: number, revision: string | null) {
    if (!profile) return;
    const ticket = ++generation.current; setBusy(true); setError("");
    const result = await client.getWorldPage(JSON.stringify({ subject: { kind: "character", profile_id: profile }, view, query, offset, ...(revision ? { revision } : {}) }));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) { setError(result.error.message); if (offset === 0) setLoad(null); return; }
    if (result.value.schema_version !== 1 || result.value.view !== view) { setError("World page changed; search again."); return; }
    setLoad({ ticket, page: result.value });
  }
  useEffect(() => {
    setLoad(null);
    void fetchPage(0, null);
    return () => { generation.current++; };
    // A new profile, view, search or refresh starts again at the first page.
  }, [client, profile, view, query, refreshKey]);
  const page = load?.page ?? null;
  const counts = page?.counts;
  const views = WORLD_VIEWS.filter((v) => v.id !== "trainers" || !counts || counts.trainers > 0 || view === "trainers");
  const current = WORLD_VIEWS.find((v) => v.id === view) ?? WORLD_VIEWS[0];
  const noun = current?.noun ?? "records";
  return <section aria-labelledby="world-heading" className="play-screen world-screen">
    <h2 id="world-heading">World browser</h2>
    <p>NPCs, objects, vendors, trainers, quests and loot from the active content. Search and paging run in the desktop backend, 50 rows at a time. Facts taken from 1.12 or Classic Era data, and inferred values, are marked. A fact without a marker has no recorded 1.12 or Classic Era origin; while origins load, or if they cannot be read, markers are absent.</p>
    {page?.fixture && <p role="note" className="play-note">SYNTHETIC — engineering fixture, not game data.</p>}
    <Field label="World profile" help="The imported character whose active content is browsed. Different builds can have different content.">{(c) => <select {...c} value={profile} onChange={(e) => setProfile(e.target.value)}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select>}</Field>
    <div role="group" aria-label="World views" className="world-views">
      {views.map((v) => <Button key={v.id} variant="ghost" aria-pressed={view === v.id} onClick={() => setView(v.id)}>{v.label}{counts ? ` (${counts[v.id]})` : ""}</Button>)}
    </div>
    {counts && counts.trainers === 0 && view !== "trainers" && <p>This content pack has no trainers.</p>}
    <form role="search" aria-label="World search" onSubmit={(e) => { e.preventDefault(); setQuery(draft); }}>
      <Field label={`Search ${noun}`} hint="Matches names, subnames and identifiers; at most 64 characters.">{(c) => <input {...c} type="search" maxLength={64} value={draft} onChange={(e) => setDraft(e.target.value)} />}</Field>
      <Button type="submit" variant="primary" disabled={busy || !profile}>Search</Button>
      {query && <Button variant="ghost" onClick={() => { setDraft(""); setQuery(""); }}>Clear search</Button>}
    </form>
    {error && <p role="alert">{error}</p>}
    <p role="status">{busy ? "Searching…" : !page ? "" : page.revision === null ? "No active content for this profile. Activate content first." : page.total === 0 ? `No ${noun} match${query ? ` “${query}”` : ""}.` : `${current?.label ?? "Records"}: ${page.offset + 1}–${page.offset + page.rows.length} of ${page.total}${query ? ` matching “${query}”` : ""}.`}</p>
    {page && page.rows.length > 0 && <ul className="world-rows">{page.rows.map((row) => row.row === "entity" ? <EntityItem key={row.id} row={row} /> : row.row === "source" ? <SourceItem key={row.id} row={row} view={page.view} /> : <QuestItem key={row.id} row={row} />)}</ul>}
    {page && page.total > page.page_size && <nav aria-label={`${noun} pages`}>
      <Button variant="ghost" disabled={busy || page.offset === 0} onClick={() => void fetchPage(Math.max(0, page.offset - page.page_size), page.revision)}>Previous page</Button>
      <Button variant="ghost" disabled={busy || page.next_offset === null} onClick={() => page.next_offset !== null && void fetchPage(page.next_offset, page.revision)}>Next page</Button>
    </nav>}
  </section>;
}
