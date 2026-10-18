export type LocalDataPreset = "small" | "balanced" | "large" | "max";
export interface LocalInstallation {
  selection_id: string;
  preset?: LocalDataPreset;
  inspection: {format_version:number;metadata:{product:string;version:string;build:number;declared_locales:string[]};build_info_sha256:string;consistency:string;limitations:string[]};
}
export interface LocalDataPlan {
  preset:LocalDataPreset;limits:{workers:number;worker_memory_bytes:number;total_storage_bytes:number};
  required_storage_bytes:number|null;available_storage_bytes:number|null;extraction_ready:boolean;blockers:string[];
}
export interface LocalDataJob {
  job_id:string;state:"preparing"|"running"|"complete"|"failed"|"cancelled"|"interrupted";completed:number;total:number;error:string|null;
  summary:{catalog_sha256:string;build:number;tables_processed:number;tables_decoded:number;actual_workers:number;gameplay_projection_available:boolean;activated:boolean}|null;
}
export interface LocalDataInventoryPage {
  revision:string;offset:number;total:number;next_offset:number|null;
  rows:{table:string;locale:string|null;file_data_id:number|null;disposition:string;reason:string;records:number|null;retained_sha256:string|null;decoded_sha256:string|null}[];
}

export interface SavedLocalInstallation {selection_id:string;product:string;preset:LocalDataPreset}
export interface LocalDataActivation {format_version:1;scope:string;generation:number;catalog_sha256:string|null;verified_for_planning:false}

export interface ResearchDeliveryActivation {format_version:1;scope:string;generation:number;delivery_sha256:string|null;verified_for_planning:false}
export interface ResearchDeliveryDiff {before:string;after:string;changed:string[];same_scope:boolean;actionable_advice:false}
export interface ResearchDeliveryPage {revision:string;domain:string;offset:number;total:number|null;sources:{table:string;state:"projected"|"gap";reason:string}[];gaps:string[];next_offset:number|null;rows:{domain:string;id:number;facts:unknown}[];guard:{development_match:"unconfigured"|"matched"|"different";actionable_advice:false;acceptance_ready:false;limitations:string[]}}

export type LocalStartupMode = "later" | "snapshots" | "local_research";
export interface LocalWorkflowRecord {
  format_version:1;revision:number;
  preferences:{format_version:1;first_run_completed:boolean;startup_mode:LocalStartupMode;selections:Record<string,{locales:string[];preset:LocalDataPreset;change_detection:boolean;baseline_sha256:string|null}>};
}
export interface LocalChangeDetector {format_version:1;selection_id:string;status:string;candidate_sha256:string|null;auto_activation:false}

export type LocalMaterialKind = "hotfix_cache"|"quest_cache"|"creature_cache"|"game_object_cache"|"item_cache"|"snapshot"|"session"|"guide"|"authored_pack";
export interface LocalMaterial {id:string;kind:LocalMaterialKind}
export interface LocalStagedInput {input_sha256:string;manifest:{format_version:number;catalog_sha256:string;kind:string;fixture:boolean;unsupported:string|null;record_count:number|null;conflict_count:number|null;locale_evidence:string;gaps:string[]}}
export interface LocalInputPage {revision:string;offset:number;total:number;next_offset:number|null;rows:{push_id:number;unique_id:number|null;region_id:number|null;table_hash:number;record_id:number;opcode:number;operation:string;payload_offset:number;payload_size:number}[]}

export interface LocalMediaDiscovery {revision:string;catalog_sha256:string;locale:string;total:number;offset:number;next_offset:number|null;gaps:string[];candidates:{table:string;row:number;field:string;file_data_id:number}[]}
export interface LocalMediaPage {revision:string;offset:number;total:number;next_offset:number|null;assets:{id:string;width:number;height:number;url:string}[]}
export interface LocalMediaManifest {format_version:1;catalog_sha256:string;locale:string;fixture:boolean;local_only:true;calibrated:false;verified_for_gameplay:false;assets:{file_data_id:number;disposition:string;read_refusal:string|null;width:number|null;height:number|null}[]}
