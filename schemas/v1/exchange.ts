/* Generated from exchange.schema.json by scripts/generate-contract-types.mjs.
 * Do not edit. Structural types only: Rust validation enforces semantic
 * invariants, and unknown members are omitted from these types. */

/**
 * Azeroth Navigator exchange envelopes, schema version 1. Structural only: Rust validation enforces semantic invariants, and structural validity never establishes in-game truth.
 */
export type AzerothNavigatorExchangeV1 =
  | CharacterSnapshotEnvelope
  | PartySnapshotEnvelope
  | RoutePlanEnvelope
  | EvidenceBundleEnvelope
  | ReferenceDatasetEnvelope
  | GearRecommendationSetEnvelope;
export type DatasetRevision = string;
export type RulesetVersion = string;
export type ScoreModelVersion = string;
export type CharacterId = string;
export type BuildId = string;
export type EditionId = string;
export type InterfaceId = string;
export type DatasetNamespace = "demo" | "reference";
export type SnapshotId = string;
export type Locale = string;
export type UtcMillis = number;
export type Sequence = number;
export type Millis = number;
export type SessionId = string;
export type Observation_EquipmentSlot =
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
      value: EquipmentSlot;
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
export type EquipmentSlot =
  | {
      state: "empty";
    }
  | {
      item: EquippedItem;
      state: "item";
    };
export type Observation_InstanceId =
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
      value: InstanceId;
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
export type InstanceId = string;
export type ItemId = string;
export type Observation_RecordMap_AttributeId_FiniteNumber =
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
      value: {
        [k: string]: FiniteNumber | undefined;
      };
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
export type FiniteNumber = number;
export type Observation_VariantId =
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
      value: VariantId;
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
export type VariantId = string;
export type Observation_Level =
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
      value: Level;
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
export type Level = number;
export type Observation_Position =
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
      value: Position;
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
export type FloorId = string;
export type MapId = string;
export type Normalized = number;
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
export type QuestId = string;
export type Observation_RoleId =
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
      value: RoleId;
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
export type RoleId = string;
export type SchemaVersion = 1;
export type ProducerComponent = "addon" | "desktop" | "cli" | "test_fixture";
export type ClockTrust = "trusted" | "uncertain";
export type StateSource =
  | "unrecorded"
  | "addon_snapshot"
  | "manual_entry"
  | "official_api"
  | "reference_data"
  | "synthetic_fixture";
export type PartyId = string;
export type CapabilityStatus =
  "verified_available" | "verified_unavailable" | "restricted" | "failed" | "untested";
export type EvidenceId = string;
export type ObjectiveId = string;
export type Text = string;
export type PlanId = string;
/**
 * Events after which a plan must be revalidated. Expiry and these triggers
 * never prove that the underlying game state changed.
 */
export type RevalidationTrigger =
  "dataset_revision_changed" | "build_changed" | "snapshot_superseded" | "ruleset_changed";
export type Observation_DurationRange =
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
      value: DurationRange;
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
export type Observation_Millis =
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
      value: Millis;
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
export type StepId = string;
export type Observation_Predicate =
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
      value: Predicate;
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
export type Predicate =
  | {
      op: "all";
      predicates: Predicate[];
    }
  | {
      op: "any";
      predicates: Predicate[];
    }
  | {
      op: "not";
      predicate: Predicate;
    }
  | {
      field: Field;
      op: "known";
    }
  | {
      field: Field;
      op: "equals";
      value: Scalar;
    }
  | {
      field: Field;
      op: "greater_than_or_equal";
      value: Scalar;
    }
  | {
      op: "quest_state";
      quest: QuestId;
      state: QuestState;
      value: boolean;
    }
  | {
      completed: boolean;
      objective: ObjectiveId;
      op: "objective_state";
    }
  | {
      level: Level;
      op: "level_at_least";
    }
  | {
      item: ItemKey;
      modifiers: {
        [k: string]: FiniteNumber | undefined;
      };
      op: "equipment_matches";
      slot: SlotId;
    }
  | {
      op: "role_is";
      role: RoleId;
    }
  | {
      area: SupportedArea;
      op: "location_in_supported_area";
    }
  | {
      key: PreferenceId;
      op: "user_preference";
      value: boolean;
    }
  | {
      op: "participant_state";
      participant: CharacterId;
      predicate: Predicate;
    };
export type Field =
  | {
      field: "level";
    }
  | {
      field: "role";
    }
  | {
      field: "preference";
      key: PreferenceId;
    }
  | {
      field: "quest";
      quest: QuestId;
      state: QuestState;
    }
  | {
      field: "objective";
      objective: ObjectiveId;
    };
export type PreferenceId = string;
export type QuestState = "accepted" | "completed" | "turned_in";
export type Scalar =
  | {
      type: "bool";
      value: boolean;
    }
  | {
      type: "integer";
      value: SafeInteger;
    }
  | {
      type: "number";
      value: FiniteNumber;
    }
  | {
      type: "copper";
      value: Copper;
    }
  | {
      type: "text";
      value: Text;
    };
export type SafeInteger = number;
/**
 * Unsigned integer copper as a canonical base-10 string (Rust also rejects values above u64)
 */
export type Copper = string;
export type SlotId = string;
export type NodeId = string;
export type Confidence =
  | "verified_on_build"
  | "publisher_documented"
  | "limited_sample"
  | "cross_edition_unverified"
  | "inferred"
  | "disputed_or_stale";
export type Observation_Text =
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
      value: Text;
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
export type SourceId = string;
export type Observation_RecordMap_CapabilityId_CapabilityCheck =
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
      value: {
        [k: string]: CapabilityCheck | undefined;
      };
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
export type ProviderId = string;
export type ProviderStatus = "available" | "unavailable" | "unverified" | "restricted";
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
export type AcquisitionId = string;
export type AcquisitionKind = "quest" | "vendor" | "dungeon" | "crafting" | "other";
export type Observation_Normalized =
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
      value: Normalized;
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
export type EdgeId = string;
export type TravelMode = "walking" | "flight" | "boat" | "tram" | "teleport" | "return";
export type EdgeStatus = "verified" | "estimated" | "temporarily_unavailable" | "unverified";
export type Observation_ArrayOf_Text =
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
      value: Text[];
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
export type Observation_CoordinateTransform =
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
      value: CoordinateTransform;
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
export type Observation_ServiceKind =
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
      value: ServiceKind;
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
export type ServiceKind =
  "junction" | "vendor" | "repair" | "trainer" | "flight" | "transport" | "dungeon" | "other";
export type ObjectiveTarget =
  | {
      kind: "quest_stage";
      quest: QuestId;
      stage: ObjectiveId;
    }
  | {
      kind: "acquisition";
      source: AcquisitionId;
    }
  | {
      kind: "service";
      node: NodeRef;
    };
export type Observation_CompletionSemantics =
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
      value: CompletionSemantics;
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
export type CompletionSemantics = "unrepeatable" | "repeatable" | "reset_bound";
export type Observation_ArrayOf_Position =
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
      value: Position[];
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
export type Observation_ArrayOf_ItemKey =
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
      value: ItemKey[];
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
export type RuleId = string;
export type DismissalPolicy = "session" | "until_ruleset_changes" | "permanent";
export type RuleTrigger =
  | "snapshot_changed"
  | "level_changed"
  | "equipment_changed"
  | "location_changed"
  | "preference_changed"
  | "manual";
export type Observation_FiniteNumber =
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
      value: FiniteNumber;
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
export type Ranking = "theoretical" | "obtainable" | "practical";

export interface CharacterSnapshotEnvelope {
  dependencies?: Dependencies;
  /**
   * True for synthetic engineering data. Production planning rejects it.
   */
  fixture: boolean;
  format: "azeroth-navigator";
  game: GameContext;
  kind: "character_snapshot";
  namespace: DatasetNamespace;
  payload: CharacterSnapshot;
  producer: Producer;
  provenance: Provenance;
  schema_version: SchemaVersion;
}
/**
 * Versions and inputs the payload was produced from. Absent fields and an
 * empty snapshot list mean "no such dependency"; each kind fixes which apply.
 */
export interface Dependencies {
  dataset_revision?: DatasetRevision | null;
  ruleset_version?: RulesetVersion | null;
  score_model_version?: ScoreModelVersion | null;
  snapshots?: SnapshotRef[];
}
export interface SnapshotRef {
  character: CharacterId;
  context: DataContext;
  id: SnapshotId;
}
export interface DataContext {
  build: BuildKey;
  namespace: DatasetNamespace;
}
export interface BuildKey {
  build: BuildId;
  edition: EditionId;
  interface: InterfaceId;
}
export interface GameContext {
  build: BuildId;
  edition: EditionId;
  interface: InterfaceId;
  locale: Locale;
}
export interface CharacterSnapshot {
  capture: ObservationMetadata;
  character: CharacterId;
  context: DataContext;
  equipment: {
    [k: string]: Observation_EquipmentSlot | undefined;
  };
  id: SnapshotId;
  level: Observation_Level;
  location: Observation_Position;
  objectives: {
    [k: string]: ObservationBoolean | undefined;
  };
  preferences: {
    [k: string]: ObservationBoolean | undefined;
  };
  quests: {
    [k: string]: QuestProgress | undefined;
  };
  role: Observation_RoleId;
  schema_version: SchemaVersion;
  source_revision: DatasetRevision;
}
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
export interface EquippedItem {
  context: DataContext;
  instance: Observation_InstanceId;
  item_id: ItemId;
  modifiers: Observation_RecordMap_AttributeId_FiniteNumber;
  variant: Observation_VariantId;
}
export interface Position {
  map: MapKey;
  x: Normalized;
  y: Normalized;
}
export interface MapKey {
  context: DataContext;
  floor: FloorId;
  instance?: InstanceId | null;
  map_id: MapId;
}
export interface QuestProgress {
  accepted: ObservationBoolean;
  completed: ObservationBoolean;
  context: DataContext;
  objectives: {
    [k: string]: ObservationBoolean | undefined;
  };
  quest: QuestId;
  turned_in: ObservationBoolean;
}
export interface Producer {
  component: ProducerComponent;
  sequence?: Sequence | null;
  session_id?: SessionId | null;
  /**
   * Producer application version; independent of schema and dataset versions.
   */
  version: string;
}
export interface Provenance {
  clock_trust: ClockTrust;
  /**
   * Absent means the producer recorded no creation time.
   */
  created_at_utc_ms?: UtcMillis | null;
  source: StateSource;
}
export interface PartySnapshotEnvelope {
  dependencies?: Dependencies;
  /**
   * True for synthetic engineering data. Production planning rejects it.
   */
  fixture: boolean;
  format: "azeroth-navigator";
  game: GameContext;
  kind: "party_snapshot";
  namespace: DatasetNamespace;
  payload: PartySnapshot;
  producer: Producer;
  provenance: Provenance;
  schema_version: SchemaVersion;
}
export interface PartySnapshot {
  context: DataContext;
  id: PartyId;
  participants: CharacterSnapshot[];
}
export interface RoutePlanEnvelope {
  dependencies?: Dependencies;
  /**
   * True for synthetic engineering data. Production planning rejects it.
   */
  fixture: boolean;
  format: "azeroth-navigator";
  game: GameContext;
  kind: "route_plan";
  namespace: DatasetNamespace;
  payload: RoutePlan;
  producer: Producer;
  provenance: Provenance;
  schema_version: SchemaVersion;
}
export interface RoutePlan {
  capability_assumptions: {
    [k: string]: CapabilityStatus | undefined;
  };
  context: DataContext;
  dataset_revision: DatasetRevision;
  evidence: EvidenceId[];
  exclusions: ObjectiveId[];
  expires_at_utc_ms?: UtcMillis | null;
  generated_at_utc_ms: UtcMillis;
  goal: Text;
  id: PlanId;
  inputs: SnapshotRef[];
  manual_assumptions: Text[];
  revalidation: RevalidationTrigger[];
  ruleset_version: RulesetVersion;
  schema_version: SchemaVersion;
  score_model_version: ScoreModelVersion;
  steps: PlanStep[];
  time_budget_ms: Millis;
  travel: TravelSegment[];
}
export interface PlanStep {
  alternatives: ObjectiveId[];
  estimate: Observation_DurationRange;
  evidence: EvidenceId[];
  expected_ms: Observation_Millis;
  explanation: Text;
  id: StepId;
  invalidation: Observation_Predicate;
  objective: ObjectiveId;
  participants: CharacterId[];
  prerequisite: Observation_Predicate;
  required_fields: Text[];
}
export interface DurationRange {
  max_ms: Millis;
  min_ms: Millis;
}
export interface ItemKey {
  context: DataContext;
  item_id: ItemId;
  variant: VariantId;
}
export interface SupportedArea {
  bounds?: Bounds | null;
  map: MapKey;
}
export interface Bounds {
  max_x: Normalized;
  max_y: Normalized;
  min_x: Normalized;
  min_y: Normalized;
}
export interface TravelSegment {
  expected_ms: Observation_Millis;
  from: NodeRef;
  range: Observation_DurationRange;
  to: NodeRef;
}
export interface NodeRef {
  id: NodeId;
  map: MapKey;
}
export interface EvidenceBundleEnvelope {
  dependencies?: Dependencies;
  /**
   * True for synthetic engineering data. Production planning rejects it.
   */
  fixture: boolean;
  format: "azeroth-navigator";
  game: GameContext;
  kind: "evidence_bundle";
  namespace: DatasetNamespace;
  payload: EvidenceBundle;
  producer: Producer;
  provenance: Provenance;
  schema_version: SchemaVersion;
}
export interface EvidenceBundle {
  evidence: EvidenceRecord[];
  game_build: GameBuild;
  providers: ProviderCapability[];
}
export interface EvidenceRecord {
  confidence: Confidence;
  conflicts: EvidenceId[];
  context: DataContext;
  id: EvidenceId;
  license: Observation_Text;
  notes: Text;
  observed_at_utc_ms: Observation_UtcMillis;
  source: SourceId;
}
export interface GameBuild {
  capabilities: Observation_RecordMap_CapabilityId_CapabilityCheck;
  key: BuildKey;
  locale: Locale;
}
export interface CapabilityCheck {
  evidence: EvidenceId[];
  status: CapabilityStatus;
}
export interface ProviderCapability {
  context: DataContext;
  datasets: DatasetRevision[];
  endpoints: Text[];
  evidence: EvidenceId[];
  freshness: ObservationMetadata;
  provider: ProviderId;
  status: ProviderStatus;
}
export interface ReferenceDatasetEnvelope {
  dependencies?: Dependencies;
  /**
   * True for synthetic engineering data. Production planning rejects it.
   */
  fixture: boolean;
  format: "azeroth-navigator";
  game: GameContext;
  kind: "reference_dataset";
  namespace: DatasetNamespace;
  payload: ReferenceDataset;
  producer: Producer;
  provenance: Provenance;
  schema_version: SchemaVersion;
}
export interface ReferenceDataset {
  acquisitions: AcquisitionSource[];
  edges: TravelEdge[];
  evidence: EvidenceRecord[];
  items: ItemDefinition[];
  map_spaces: MapSpace[];
  nodes: TravelNode[];
  objectives: Objective[];
  quests: QuestDefinition[];
  revision: DatasetRevision;
  rules: ContextRule[];
}
export interface AcquisitionSource {
  conditions: Observation_Predicate;
  context: DataContext;
  cost: Observation_Copper;
  evidence: EvidenceId[];
  id: AcquisitionId;
  items: ItemKey[];
  kind: AcquisitionKind;
  probability: Observation_Normalized;
}
export interface TravelEdge {
  consumes_resource: ObservationBoolean;
  context: DataContext;
  cooldown_ms: Observation_Millis;
  evidence: EvidenceId[];
  expected_ms: Observation_Millis;
  from: NodeRef;
  id: EdgeId;
  mode: TravelMode;
  requirements: Observation_Predicate;
  status: EdgeStatus;
  to: NodeRef;
  uncertainty: Observation_DurationRange;
  wait_ms: Observation_Millis;
}
export interface ItemDefinition {
  attributes: Observation_RecordMap_AttributeId_FiniteNumber;
  effects: Observation_ArrayOf_Text;
  evidence: EvidenceId[];
  key: ItemKey;
  requirements: Observation_Predicate;
  slots: SlotId[];
  two_handed: ObservationBoolean;
  unique_group: Observation_Text;
}
export interface MapSpace {
  evidence: EvidenceId[];
  key: MapKey;
  transform: Observation_CoordinateTransform;
}
export interface CoordinateTransform {
  offset_x: FiniteNumber;
  offset_y: FiniteNumber;
  scale_x: FiniteNumber;
  scale_y: FiniteNumber;
  units: Text;
}
export interface TravelNode {
  id: NodeId;
  position: Position;
  service: Observation_ServiceKind;
}
export interface Objective {
  context: DataContext;
  evidence: EvidenceId[];
  expected_ms: Observation_Millis;
  id: ObjectiveId;
  participants: CharacterId[];
  requirements: Observation_Predicate;
  targets: ObjectiveTarget[];
}
export interface QuestDefinition {
  completion: Observation_CompletionSemantics;
  context: DataContext;
  eligibility: Observation_Predicate;
  evidence: EvidenceId[];
  id: QuestId;
  locations: Observation_ArrayOf_Position;
  prerequisites: Observation_Predicate;
  rewards: Observation_ArrayOf_ItemKey;
  shared_progress: ObservationBoolean;
  stages: ObjectiveId[];
}
export interface ContextRule {
  context: DataContext;
  cooldown_ms: Millis;
  deduplication_key: RuleId;
  dismissal: DismissalPolicy;
  evidence: EvidenceId[];
  expires_at_utc_ms?: UtcMillis | null;
  id: RuleId;
  predicate: Predicate;
  priority: number;
  result_template: Text;
  trigger: RuleTrigger;
}
export interface GearRecommendationSetEnvelope {
  dependencies?: Dependencies;
  /**
   * True for synthetic engineering data. Production planning rejects it.
   */
  fixture: boolean;
  format: "azeroth-navigator";
  game: GameContext;
  kind: "gear_recommendations";
  namespace: DatasetNamespace;
  payload: GearRecommendationSet;
  producer: Producer;
  provenance: Provenance;
  schema_version: SchemaVersion;
}
export interface GearRecommendationSet {
  dataset_revision: DatasetRevision;
  /**
   * Empty means no recommendation was produced, not that none exists.
   */
  recommendations: GearRecommendation[];
  score_model_version: ScoreModelVersion;
  snapshot: SnapshotRef;
}
export interface GearRecommendation {
  acquisitions: AcquisitionId[];
  assumptions: Text[];
  baseline: {
    [k: string]: ConfigurationItem | undefined;
  };
  baseline_score: Observation_FiniteNumber;
  context: DataContext;
  dataset_revision: DatasetRevision;
  eligibility: ObservationBoolean;
  evidence: EvidenceId[];
  explanation: Text;
  proposed: {
    [k: string]: ConfigurationItem | undefined;
  };
  proposed_score: Observation_FiniteNumber;
  ranking: Ranking;
  scenario: RoleId;
  score_model_version: ScoreModelVersion;
  snapshot: SnapshotRef;
  unmodeled_effects: Text[];
}
export interface ConfigurationItem {
  item: ItemKey;
  occupied_slots: SlotId[];
  unique_group: Observation_Text;
}
