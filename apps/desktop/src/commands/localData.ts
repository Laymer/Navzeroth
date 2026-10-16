export type LocalDataPreset = "small" | "balanced" | "large" | "max";
export interface LocalInstallation {
  selection_id: string;
  inspection: {format_version:number;metadata:{product:string;version:string;build:number;declared_locales:string[]};build_info_sha256:string;consistency:string;limitations:string[]};
}
export interface LocalDataPlan {
  preset:LocalDataPreset;limits:{workers:number;worker_memory_bytes:number;total_storage_bytes:number};
  required_storage_bytes:number|null;available_storage_bytes:number|null;extraction_ready:boolean;blockers:string[];
}
export interface LocalDataJob {
  job_id:string;state:"running"|"complete"|"failed"|"cancelled";completed:number;total:number;error:string|null;
  summary:{catalog_sha256:string;build:number;tables_processed:number;tables_decoded:number;actual_workers:number;gameplay_projection_available:boolean;activated:boolean}|null;
}
export interface LocalDataInventoryPage {
  revision:string;offset:number;total:number;next_offset:number|null;
  rows:{table:string;locale:string|null;file_data_id:number|null;disposition:string;reason:string;records:number|null;retained_sha256:string|null;decoded_sha256:string|null}[];
}
