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
export type InteractionMode="interactive"|"pass_through";
export type OperationDispatch="not_attempted"|"accepted"|"failed";
export type ControlReason="main_not_focused"|"focus_query_failed"|"reference_focused"|"window_absent"|"create_failed"|"focusable_failed"|"pointer_failed"|"topmost_failed"|"visibility_failed"|"state_query_failed";
export interface ReferenceControlReceipt {schema_version:1;requested_visible:boolean|null;requested_interaction:InteractionMode;dispatch:"accepted"|"refused"|"partial_failure";reason:ControlReason|null;operations:{creation:OperationDispatch;focusability:OperationDispatch;pointer_events:OperationDispatch;topmost:OperationDispatch;visibility:OperationDispatch;state_query:OperationDispatch};window:ReferenceWindowState|null}
export interface ReferenceControlClient extends ReferenceClient {
 toggleReferenceWindow(alwaysOnTop:boolean,interactionMode:InteractionMode):Promise<Result<ReferenceControlReceipt>>;
 setReferenceInteraction(interactionMode:InteractionMode):Promise<Result<ReferenceControlReceipt>>;
}
