/** Versioned explanations; exported plan steps retain their original IDs. */
export interface ClusterChoice {
  members: string[]; node: string; participant_utilities: number[]; utility: number;
  completion_ms: number; service_ms: number; inbound_ms: number; stopping_ms: number;
  incremental_ms: number; expected_session_ms: number;
}
export interface ClusteringMetadata {
  schema_version: 1;
  selected_clusters: ClusterChoice[];
  cluster_alternatives: ClusterChoice[][];
  assessments: unknown[];
  variant_trace: unknown[];
  peak_variant_count: number;
  variants_evaluated: number;
}
