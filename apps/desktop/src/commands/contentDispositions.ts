import type {ContentInspection} from "./contract";
export type ContentRevisionKey=ContentInspection["key"];
export type DispositionState="eligible"|"quarantined"|"revoked";
export type DispositionReason="initial_eligibility"|"operator_review"|"invalid_content"|"source_permission"|"context_mismatch"|"withdrawn_contribution";
export interface DispositionGuard {key:ContentRevisionKey;content_sha256:string;state:DispositionState;state_revision:number}
export interface ContentDisposition extends DispositionGuard {reason_code:DispositionReason;changed_at_utc_ms:number|null;last_activated_order:number|null}
export interface ContentGovernance {schema_version:1;scope:ContentRevisionKey["scope"];active:ContentRevisionKey|null;revisions:ContentDisposition[]}
export interface ContentDecision {schema_version:1;outcome:{disposition:ContentDisposition;active:ContentRevisionKey|null;fallback_applied:boolean}}
