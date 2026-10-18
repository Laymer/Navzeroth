// Selected invalid application requests, never file dialogs or private inputs.
// These exercise Rust handlers through the real renderer/native serialization.
export const COMMAND_BOUNDARIES = [
 ...['update_app_settings','mutate_reminders','plan_joint_session','propose_party_plan','accept_party_plan','change_content_disposition','export_plan','export_agreed_party_plan','inspect_plan_route','plan_session','get_content_inspection_page','stage_content','get_catalog_page','get_authored_catalog','get_dungeon_preparation','compare_gear','preview_acquisition_route','assess_route_acquisition','change_local_submission','export_local_submission'].map(command=>({command,args:{text:'{"truncated":'},codes:['INVALID_INPUT']})),
 {command:'preview_local_data',args:{selection_id:'missing-synthetic-selection',preset:'small'}},
 {command:'start_local_data',args:{selection_id:'missing-synthetic-selection',preset:'small',locales:['enUS']}},
 {command:'start_local_data',args:{selection_id:'missing-synthetic-selection',preset:'small',locales:null}},
 {command:'save_local_workflow_choices',args:{selection_id:'missing-synthetic-selection',expected:0,locales:['enUS'],preset:'small',detect:false}},
 {command:'acknowledge_local_build_change',args:{selection_id:'missing-synthetic-selection',expected:0,candidate:'invalid'}},
 {command:'stage_selected_local_input',args:{catalog:'invalid',id:'missing-synthetic-material',locale:'enUS',preset:'small'}},
 {command:'get_local_input_page',args:{revision:'invalid',offset:0,limit:201}},
 {command:'discover_local_media',args:{catalog:'invalid',locale:'enUS',offset:0,limit:201}},
 {command:'extract_local_media',args:{selection_id:'missing-synthetic-selection',catalog:'invalid',locale:'enUS',ids:[0],preset:'small'}},
 {command:'get_local_media_page',args:{revision:'invalid',offset:0,limit:201}},
 {command:'get_local_media_manifest',args:{revision:'invalid'}},
 {command:'get_local_artifact_page',args:{namespace:'../outside',offset:0,limit:201}},
 {command:'get_research_delivery_manifest',args:{revision:'invalid'}},
 {command:'build_local_inventory',args:{delivery:'invalid',authored:null,preset:'small'}},
 {command:'get_local_inventory_summary',args:{revision:'invalid',authored:null}},
 {command:'get_local_inventory_page',args:{revision:'invalid',domain:'invalid',offset:0,limit:201,authored:null}},
 {command:'get_local_tooltip',args:{projection:'invalid',spell:0}},
 {command:'restore_local_installation',args:{selection_id:'missing-synthetic-selection'}},
 {command:'resume_local_data',args:{job_id:'missing-synthetic-job'}},
 {command:'get_local_data_activation',args:{revision:'invalid'}},
 {command:'activate_local_data',args:{revision:'invalid',expected:0,rollback:false}},
 {command:'get_local_data_inventory',args:{revision:'invalid',offset:0,limit:201}},
 {command:'prepare_research_delivery',args:{catalog:'invalid',locale:'enUS',bindings:null}},
 {command:'get_research_delivery_page',args:{revision:'invalid',domain:'invalid',offset:0,limit:201}},
 {command:'get_research_delivery_activation',args:{revision:'invalid'}},
 {command:'activate_research_delivery',args:{revision:'invalid',expected:0,rollback:false}},
 {command:'diff_research_delivery',args:{before:'invalid',after:'invalid'}},
];

export function validateRefusal(test, outcome) {
 if(outcome.ok)throw new Error(`${test.command} accepted a deliberately invalid request`);
 const error=outcome.error;
 const codes=test.codes??['INVALID_INPUT','STORAGE_ERROR'];
 if(!error||!codes.includes(error.code)||typeof error.message!=='string'||typeof error.retryable!=='boolean')throw new Error(`${test.command} did not return its structured application refusal: ${JSON.stringify(error)}`);
}
