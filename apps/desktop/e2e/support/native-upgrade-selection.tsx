// Test-only dev entry: actual ContentScreen, synthetic IPC; no native/CSP claims.
import { createRoot } from "react-dom/client";
import { ContentScreen } from "../../src/screens/ContentScreen";
import { createClient, type CommandAdapter } from "../../src/commands";
import type { ContentInspection } from "../../src/commands/contract";
import golden from "../../../../fixtures/desktop/content-governance/synthetic-flow.json";
const first = golden.profiles[0];
if (!first) throw Error("Missing Rust-written synthetic profile");
const second = { ...first, id: `stored-${"b".repeat(64)}`, display_name: "other-synthetic-character" };
const empty = { ...first, id: `stored-${"c".repeat(64)}`, display_name: "empty-synthetic-character" };
const calls: string[] = [];
const adapter: CommandAdapter = { invoke(command, input) {
  calls.push(command === "get_active_content" ? `${command}:${(input as {profile_id:string}).profile_id}` : command);
  switch (command) {
    case "get_app_status": return Promise.resolve({ data_mode: "stored" } as never);
    case "list_profiles": return Promise.resolve([first, second, empty] as never);
    case "list_local_submissions": return Promise.resolve({rows:[],next:null,total:0} as never);
    case "get_content_dispositions": return Promise.resolve({schema_version:1,scope:golden.inspection.key.scope,active:null,revisions:[]} as never);
    case "get_active_content": {
      const id = (input as {profile_id:string}).profile_id;
      const content: ContentInspection | null = id === empty.id ? null : {
        ...golden.inspection, key: { ...golden.inspection.key, revision: id === second.id ? "other-r1" : golden.inspection.key.revision },
      } as ContentInspection;
      return new Promise((resolve) => setTimeout(() => resolve(content as never), 100));
    }
    default: return Promise.reject(Error(`Unexpected synthetic command ${command}`));
  }
} };
Object.defineProperty(window, "__selectionRegression", { value: { calls } });
const root = document.getElementById("root");
if (!root) throw Error("Missing fixture root");
createRoot(root).render(<ContentScreen client={createClient(adapter)} onChanged={()=>{}} refreshKey={0}/>);
