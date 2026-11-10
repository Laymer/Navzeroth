/* Generated from progression-api.schema.json by scripts/generate-contract-types.mjs.
 * Do not edit. Structural types only: Rust validation enforces semantic
 * invariants, and unknown members are omitted from these types. */

/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "UtcMillis".
 */
export type UtcMillis = number;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Sequence".
 */
export type Sequence = number;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Millis".
 */
export type Millis = number;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "SessionId".
 */
export type SessionId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "CharacterId".
 */
export type CharacterId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "BuildId".
 */
export type BuildId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "EditionId".
 */
export type EditionId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "InterfaceId".
 */
export type InterfaceId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "DatasetNamespace".
 */
export type DatasetNamespace = "demo" | "reference";
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "SnapshotId".
 */
export type SnapshotId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Sha256Hex".
 */
export type Sha256Hex = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "DatasetRevision".
 */
export type DatasetRevision = string;
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue3".
 */
export type ManualValue3 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: number;
    };
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue6".
 */
export type ManualValue6 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: Enumeration4;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Completeness".
 */
export type Completeness = "complete" | "partial";
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ItemId".
 */
export type ItemId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "VariantId".
 */
export type VariantId = string;
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue7".
 */
export type ManualValue7 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: ManualEquipment;
    };
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue8".
 */
export type ManualValue8 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: Enumeration5;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "EnhancementDuration".
 */
export type EnhancementDuration = "permanent" | "temporary";
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue2".
 */
export type ManualValue2 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: Copper;
    };
/**
 * Unsigned integer copper as a canonical base-10 string (Rust also rejects values above u64)
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Copper".
 */
export type Copper = string;
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue4".
 */
export type ManualValue4 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: Enumeration2;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "SourceId".
 */
export type SourceId = string;
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue5".
 */
export type ManualValue5 =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: Enumeration3;
    };
/**
 * Manual input has no caller-controlled provenance or observation metadata.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualValue".
 */
export type ManualValue =
  | {
      status: "unknown";
    }
  | {
      status: "known";
      value: Enumeration;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Freshness".
 */
export type Freshness =
  | {
      age_ms: Millis;
      status: "fresh";
    }
  | {
      age_ms: Millis;
      status: "stale";
    }
  | {
      reason: ClockUncertainty;
      status: "uncertain";
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ClockUncertainty".
 */
export type ClockUncertainty = "missing_or_untrusted_time" | "clock_reversed" | "age_out_of_range";
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_uint16".
 */
export type ObservationUint16 =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: number;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_Enumeration4".
 */
export type Observation_Enumeration4 =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: Enumeration9;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_EquipmentEnhancements".
 */
export type Observation_EquipmentEnhancements =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: EquipmentEnhancements;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_Enumeration5".
 */
export type Observation_Enumeration5 =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: Enumeration10;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_EnhancementDuration".
 */
export type Observation_EnhancementDuration =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: EnhancementDuration;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_Copper".
 */
export type Observation_Copper =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: Copper;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "SchemaVersion".
 */
export type SchemaVersion = 1;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_Enumeration2".
 */
export type Observation_Enumeration2 =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: Enumeration7;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_SourceId".
 */
export type Observation_SourceId =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: SourceId;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_Enumeration3".
 */
export type Observation_Enumeration3 =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: Enumeration8;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_boolean".
 */
export type ObservationBoolean =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: boolean;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_Enumeration".
 */
export type Observation_Enumeration =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: Enumeration6;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ClockTrust".
 */
export type ClockTrust = "trusted" | "uncertain";
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_UtcMillis".
 */
export type Observation_UtcMillis =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: UtcMillis;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Observation_uint32".
 */
export type ObservationUint32 =
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "known";
      value: number;
    }
  | {
      clock_trust?: "trusted" | "uncertain";
      observed_at_utc_ms?: UtcMillis | null;
      received_at_utc_ms?: UtcMillis | null;
      sequence?: Sequence | null;
      session_elapsed_ms?: Millis | null;
      session_id?: SessionId | null;
      source?:
        | "unrecorded"
        | "addon_snapshot"
        | "manual_entry"
        | "official_api"
        | "reference_data"
        | "synthetic_fixture";
      status: "unknown" | "unsupported" | "restricted" | "not_applicable";
      value?: never;
    };
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "SlotId".
 */
export type SlotId = string;
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "StateSource".
 */
export type StateSource =
  | "unrecorded"
  | "addon_snapshot"
  | "manual_entry"
  | "official_api"
  | "reference_data"
  | "synthetic_fixture";

/**
 * Schema generation root; this is not an executable command dispatcher.
 */
export interface ProgressionApi {
  import_request: ImportProgressionRequest;
  manual_request: ManualProgressionRequest;
  receipt: ProgressionReceipt;
  view: ProgressionView;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ImportProgressionRequest".
 */
export interface ImportProgressionRequest {
  expected: SnapshotBinding;
  profile_id: string;
  text: string;
}
/**
 * Exact predecessor identity for progression companions and saved advice.
 * The I/O layer supplies the persisted snapshot hash; this module does not hash
 * or normalize imported bytes and never accepts a nearest-build substitution.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "SnapshotBinding".
 */
export interface SnapshotBinding {
  capture: ObservationMetadata;
  character: CharacterId;
  context: DataContext;
  snapshot: SnapshotId;
  snapshot_sha256: Sha256Hex;
  source_revision: DatasetRevision;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ObservationMetadata".
 */
export interface ObservationMetadata {
  clock_trust?: "trusted" | "uncertain";
  observed_at_utc_ms?: UtcMillis | null;
  received_at_utc_ms?: UtcMillis | null;
  sequence?: Sequence | null;
  session_elapsed_ms?: Millis | null;
  session_id?: SessionId | null;
  source?:
    | "unrecorded"
    | "addon_snapshot"
    | "manual_entry"
    | "official_api"
    | "reference_data"
    | "synthetic_fixture";
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "DataContext".
 */
export interface DataContext {
  build: BuildKey;
  namespace: DatasetNamespace;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "BuildKey".
 */
export interface BuildKey {
  build: BuildId;
  edition: EditionId;
  interface: InterfaceId;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualProgressionRequest".
 */
export interface ManualProgressionRequest {
  available_talent_points: ManualValue3;
  bags: ManualValue6;
  bank: ManualValue6;
  equipment_enhancements: {
    [k: string]: ManualValue7 | undefined;
  };
  expected: SnapshotBinding;
  id: SnapshotId;
  money: ManualValue2;
  professions: ManualValue4;
  profile_id: string;
  recipes: ManualValue5;
  spells: ManualValue;
  talents: ManualValue;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration4".
 */
export interface Enumeration4 {
  completeness: Completeness;
  entries: {
    [k: string]: ManualInventory | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualInventory".
 */
export interface ManualInventory {
  item: ItemKey;
  quantity: number;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ItemKey".
 */
export interface ItemKey {
  context: DataContext;
  item_id: ItemId;
  variant: VariantId;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualEquipment".
 */
export interface ManualEquipment {
  enhancements: ManualValue8;
  item: ItemKey;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration5".
 */
export interface Enumeration5 {
  completeness: Completeness;
  entries: {
    [k: string]: ManualEnhancement | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualEnhancement".
 */
export interface ManualEnhancement {
  duration?: EnhancementDuration | null;
  expires_at_utc_ms?: UtcMillis | null;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration2".
 */
export interface Enumeration2 {
  completeness: Completeness;
  entries: {
    [k: string]: ManualProfession | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ManualProfession".
 */
export interface ManualProfession {
  maximum_skill?: number | null;
  rank?: SourceId | null;
  skill?: number | null;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration3".
 */
export interface Enumeration3 {
  completeness: Completeness;
  entries: {
    [k: string]: boolean | undefined;
  };
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration".
 */
export interface Enumeration {
  completeness: Completeness;
  entries: {
    [k: string]: number | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ProgressionReceipt".
 */
export interface ProgressionReceipt {
  companion_id: SnapshotId;
  fixture: boolean;
  profile_id: string;
  snapshot_id: SnapshotId;
  stored: boolean;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ProgressionView".
 */
export interface ProgressionView {
  binding: SnapshotBinding;
  fixture: boolean;
  imported: ProgressionRecord[];
  manual: ProgressionRecord[];
  profile_id: string;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ProgressionRecord".
 */
export interface ProgressionRecord {
  freshness: Freshness;
  snapshot: ProgressionSnapshot;
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ProgressionSnapshot".
 */
export interface ProgressionSnapshot {
  available_talent_points: ObservationUint16;
  /**
   * Bags and bank are separate enumerations and never silently combined.
   */
  bags:
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "known";
        value: Enumeration9;
      }
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "unknown" | "unsupported" | "restricted" | "not_applicable";
        value?: never;
      };
  bank: Observation_Enumeration4;
  binding: SnapshotBinding;
  capture: ObservationMetadata;
  equipment_enhancements: {
    [k: string]: Observation_EquipmentEnhancements | undefined;
  };
  id: SnapshotId;
  money: Observation_Copper;
  payload_version: SchemaVersion;
  professions: Observation_Enumeration2;
  recipes: Observation_Enumeration3;
  /**
   * Keys identify exact source spell records; unknown rank is not rank zero.
   */
  spells:
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "known";
        value: Enumeration6;
      }
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "unknown" | "unsupported" | "restricted" | "not_applicable";
        value?: never;
      };
  talents: Observation_Enumeration;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration9".
 */
export interface Enumeration9 {
  completeness: Completeness;
  entries: {
    [k: string]: InventoryQuantity | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "InventoryQuantity".
 */
export interface InventoryQuantity {
  item: ItemKey;
  /**
   * Aggregated quantity within this one explicit scope. Never a bag index.
   */
  quantity:
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "known";
        value: number;
      }
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "unknown" | "unsupported" | "restricted" | "not_applicable";
        value?: never;
      };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "EquipmentEnhancements".
 */
export interface EquipmentEnhancements {
  enhancements: Observation_Enumeration5;
  item: ItemKey;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration10".
 */
export interface Enumeration10 {
  completeness: Completeness;
  entries: {
    [k: string]: ObservedEnhancement | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ObservedEnhancement".
 */
export interface ObservedEnhancement {
  duration: Observation_EnhancementDuration;
  /**
   * Unreadable expiry remains unknown/restricted; no inferred duration.
   */
  expires_at_utc_ms:
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "known";
        value: UtcMillis;
      }
    | {
        clock_trust?: "trusted" | "uncertain";
        observed_at_utc_ms?: UtcMillis | null;
        received_at_utc_ms?: UtcMillis | null;
        sequence?: Sequence | null;
        session_elapsed_ms?: Millis | null;
        session_id?: SessionId | null;
        source?:
          | "unrecorded"
          | "addon_snapshot"
          | "manual_entry"
          | "official_api"
          | "reference_data"
          | "synthetic_fixture";
        status: "unknown" | "unsupported" | "restricted" | "not_applicable";
        value?: never;
      };
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration7".
 */
export interface Enumeration7 {
  completeness: Completeness;
  entries: {
    [k: string]: ProfessionState | undefined;
  };
}
/**
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "ProfessionState".
 */
export interface ProfessionState {
  maximum_skill: ObservationUint16;
  rank: Observation_SourceId;
  skill: ObservationUint16;
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration8".
 */
export interface Enumeration8 {
  completeness: Completeness;
  entries: {
    [k: string]: ObservationBoolean | undefined;
  };
}
/**
 * Availability and capture metadata live on the enclosing Observation. An
 * unavailable enumeration is never represented as an empty complete map.
 *
 * This interface was referenced by `ProgressionApi`'s JSON-Schema
 * via the `definition` "Enumeration6".
 */
export interface Enumeration6 {
  completeness: Completeness;
  entries: {
    [k: string]: ObservationUint16 | undefined;
  };
}
