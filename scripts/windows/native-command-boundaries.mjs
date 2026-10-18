// Selected invalid application requests, never file dialogs or private inputs.
// These exercise Rust handlers through the real renderer/native serialization.
import requests from "../../fixtures/desktop/native-boundaries/requests.json" with {type:"json"};
export const COMMAND_BOUNDARIES = requests;

export function validateRefusal(test, outcome) {
 if(outcome.ok)throw new Error(`${test.command} accepted a deliberately invalid request`);
 const error=outcome.error;
 const codes=test.codes??['INVALID_INPUT','STORAGE_ERROR'];
 if(!error||!codes.includes(error.code)||typeof error.message!=='string'||typeof error.retryable!=='boolean')throw new Error(`${test.command} did not return its structured application refusal: ${JSON.stringify(error)}`);
}
