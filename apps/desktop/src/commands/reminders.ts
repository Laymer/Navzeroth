import type { DatasetRevision, SnapshotId } from "../../../../schemas/v1/exchange";
export interface ReminderItem {
  rule: string; priority: number; dismissal: "session" | "until_ruleset_changes" | "permanent";
  kind: { kind: "advice"; text: string } | { kind: "cannot_evaluate"; reasons: unknown[] };
  evidence: string[];
}
export interface ReminderView {
  profile_id: string; snapshot_id: SnapshotId | null; revision: DatasetRevision | null; fixture: boolean; enabled: boolean;
  plan: { mode: "normal" | "non_intrusive" | "suppressed"; queue: ReminderItem[]; held: ReminderItem[]; excluded: Record<string, { code: string }> } | null;
  dismissed: string[]; rules: {id: string; text: string; dismissal: ReminderItem["dismissal"]}[];
}
export type ReminderAction = {kind:"acknowledge";rules:string[]} | {kind:"dismiss" | "restore";rule:string} | {kind:"preferences";enabled:boolean};
export interface ReminderRequest {profile_id:string;snapshot_id:SnapshotId|null;revision:DatasetRevision|null;action:ReminderAction}
