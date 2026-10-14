import type {DataContext,Position} from "../../../../schemas/v1/exchange";
import type {Freshness,Result} from "./contract";
import type {GearObservation} from "./gear";
export interface ReferenceViewData {
 schema_version:1;app_version:string;current_plan?:import("./today").CurrentPlanView|null;accessibility?:import("./today").AppSettings["accessibility"]|null;snapshot:null|{character:string;snapshot_id:string;context:DataContext;fixture:boolean;level:GearObservation<number>;role:GearObservation<string>;location:GearObservation<Position>;capture_freshness:Freshness;level_freshness:Freshness;role_freshness:Freshness;location_freshness:Freshness};
}
export interface ReferenceWindowState {schema_version:1;exists:boolean;visible:boolean;always_on_top:boolean}
export interface ReferenceClient {
 getReferenceView():Promise<Result<ReferenceViewData>>;
 getReferenceWindowState():Promise<Result<ReferenceWindowState>>;
 setReferenceWindow(visible:boolean,alwaysOnTop:boolean):Promise<Result<ReferenceWindowState>>;
}
