import type { CommandClient } from "./client";
// World browser and origin markers (ADR 0007 §3 and §9). Rust searches and pages;
// the UI only renders one bounded page at a time.
export type Observed<T> = { status: "known"; value: T } | { status: "unknown" | "unsupported" | "restricted" | "not_applicable"; value?: never };
/** A fact's non-Forever origin. No mark means Forever-origin, non-inferred or unattributed. */
export type OriginMark = "vanilla_1_12" | "wow_classic_era" | "inferred";
export type WorldView = "entities" | "vendors" | "trainers" | "quests" | "loot";
export type ServiceKind = "junction" | "vendor" | "repair" | "trainer" | "flight" | "transport" | "dungeon" | "other";
export interface ItemRef { item_id: string; variant: string }
export interface Place { node: string; map_id: string; floor: string; x: number; y: number }
export interface EntityRow {
  row: "entity"; id: string; kind: "npc" | "game_object"; name: string; subname: Observed<string>; levels: Observed<[number, number]>;
  services: ServiceKind[]; reaction: Observed<string>; places: Place[]; places_total: number; origin: OriginMark[];
}
export type Cost = { kind: "copper"; amount: string } | { kind: "item"; item: ItemRef & { context?: unknown }; quantity: number } | { kind: "currency"; currency: string; quantity: number };
export interface Reward { item: ItemRef & { context?: unknown }; quantity: number; choice_group: string | null }
export interface Offer {
  acquisition: string; items: ItemRef[]; cost: Observed<string>; costs: Observed<Cost[]>; probability: Observed<number>;
  rewards: Observed<Reward[]>; location: string | null; dungeon: string | null; origin: OriginMark[];
}
/** A vendor or a loot source, grouped by entity. `entity` is null when the pack names an entity it does not describe. */
export interface SourceRow { row: "source"; id: string; entity: Omit<EntityRow, "row"> | null; offers: Offer[]; offers_total: number }
export type TargetRow = { kind: "quest_stage"; stage: string } | { kind: "acquisition"; source: string } | { kind: "service"; node: string };
export interface QuestRow {
  row: "quest"; id: string; stages: string[]; objectives: { id: string; targets: TargetRow[]; origin: OriginMark[] }[]; objectives_total: number;
  rewards: Observed<ItemRef[]>; offers: Offer[]; offers_total: number; completion: Observed<string>; origin: OriginMark[];
}
export type WorldRow = EntityRow | SourceRow | QuestRow;
export interface WorldCounts { entities: number; vendors: number; trainers: number; quests: number; loot: number }
export interface WorldPage {
  schema_version: 1; revision: string | null; fixture: boolean; view: WorldView; query: string;
  offset: number; total: number; next_offset: number | null; page_size: number; counts: WorldCounts; rows: WorldRow[];
}
export type OriginCollection = "items" | "quests" | "acquisitions" | "edges" | "objectives" | "nodes" | "dungeons" | "entities";
export interface OriginRef { collection: OriginCollection; id: string; variant?: string }
export interface RecordOrigins { revision: string; origins: (OriginMark[] | null)[] }
export type WorldSubject = import("./catalogPages").CatalogSubject;
export type WorldClient = Pick<CommandClient, "getWorldPage" | "listProfiles">;
export type OriginClient = Pick<CommandClient, "getRecordOrigins">;
/** Stable key of an origin reference, for lookups in UI maps. */
export const originKey = (ref: OriginRef) => JSON.stringify([ref.collection, ref.id, ref.variant ?? null]);
