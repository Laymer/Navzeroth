import type { ContentInspection } from "./contract";
import type { DataContext } from "../../../../schemas/v1/exchange";
export type Reported<T> = { status: "known"; value: T } | { status: "unknown" | "unsupported" | "restricted" | "stale" | "not_applicable" };
export type LocalPayload = { kind: "quest_state"; quest: string; state: Reported<"not_started" | "accepted" | "completed" | "turned_in"> } | { kind: "price"; item: string; copper: Reported<number> } | { kind: "acquisition_outcome"; item: string; acquired: Reported<boolean>; opportunities: Reported<number>; successes: Reported<number>; group_eligible: Reported<boolean>; personally_received: Reported<boolean> };
export interface LocalObservation { schema_version: 1; context: DataContext; fixture: boolean; base_revision: string; source: string; evidence: string; observed_at_utc_ms: number | null; payload: LocalPayload }
export interface SubmissionGuard { id: string; revision: number }
export type SubmissionState = "pending" | "accepted" | "rejected" | "withdrawn" | "revoked";
export type SubmissionAction = "accept" | "reject" | "withdraw" | "revoke" | "erase_revoked";
export interface SubmissionRecord { guard: SubmissionGuard; base: ContentInspection["key"]; source: string; state: SubmissionState; decision_code: string; document: LocalObservation | null; created_at_utc_ms: number | null; changed_at_utc_ms: number | null }
export interface SubmissionPage { rows: SubmissionRecord[]; next: string | null; total: number }
export interface SubmitReceipt { duplicate: boolean; record: SubmissionRecord }
export interface LocalSubmissionExport { format: "azeroth-local-observation"; schema_version: 1; text: string; byte_count: number; sha256: string }
