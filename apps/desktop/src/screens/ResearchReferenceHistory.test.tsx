import {fireEvent,render,screen} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {ResearchReferenceHistory} from "./ResearchReferenceHistory";
it("verifies the retained reference against the current catalog before opening it",async()=>{
 const selected=vi.fn();const revision="a".repeat(64);let matching=false;
 const adapter:CommandAdapter={invoke(command){return Promise.resolve((command==="get_local_artifact_page"?{format_version:1,namespace:"deliveries",revision:"list",offset:0,total:1,rows:[revision],next_offset:null}:{format_version:1,catalog_sha256:matching?"current":"other"})as never);}};
 render(<ResearchReferenceHistory client={createClient(adapter)} catalog="current" disabled={false} onReference={selected}/>);
 fireEvent.click(screen.getByRole("button",{name:"Review retained research references"}));fireEvent.click(await screen.findByRole("button",{name:"Inspect reference aaaaaaaa"}));
 await screen.findByText(/belongs to another catalog/);expect(selected).not.toHaveBeenCalled();
 matching=true;fireEvent.click(screen.getByRole("button",{name:"Inspect reference aaaaaaaa"}));await vi.waitFor(()=>expect(selected).toHaveBeenCalledWith(revision));
});
