import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { DungeonsScreen } from "./DungeonsScreen";
import type { CommandClient } from "../commands";
import type { PreparationReport } from "../commands/dungeonPreparation";
import fixture from "../../../../fixtures/desktop/preparation/report.json";
const report=fixture as PreparationReport;
function fixtureMember(){const member=report.members[0];if(!member)throw Error("Synthetic wirefixture needs a member");return member;}
const firstMember=fixtureMember();
function mock(mode:"stored"|"demo"="stored") {
  return {listProfiles:vi.fn(async()=>({ok:true,value:[{id:firstMember.profile_id,display_name:"Synthetic member",data_mode:mode}]})),
    listParties:vi.fn(async()=>({ok:true,value:["preparation-party"]})),
    getDungeonPreparation:vi.fn(async(text:string)=>{expect(JSON.parse(text).schema_version).toBe(1);return {ok:true,value:report};})};
}
async function ready() {await waitFor(()=>expect(screen.getByRole("button",{name:"Inspect preparation gaps"})).toBeEnabled());}
test("browser demo cannot claim imported preparation state or invoke evaluation",async()=>{
  const client=mock("demo");render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);
  await screen.findByText(/Browser demo profiles cannot supply/);
  fireEvent.click(screen.getByText("Advanced local reference editor"));
  fireEvent.click(screen.getByRole("button",{name:"Load synthetic preparation reference"}));
  expect(screen.getByRole("button",{name:"Inspect preparation gaps"})).toBeDisabled();expect(client.getDungeonPreparation).not.toHaveBeenCalled();
});
test("explicit local party inspection exposes evidence gaps, false and zero without promoting readiness",async()=>{
  const client=mock();render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);await ready();
  fireEvent.change(screen.getByLabelText("Preparation members"),{target:{value:"party"}});
  fireEvent.click(screen.getByText("Advanced local reference editor"));
  fireEvent.click(screen.getByRole("button",{name:"Load synthetic preparation reference"}));
  fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  await screen.findByText(/Declared group formation: 0 to 0 ms/);
  expect(screen.getByText(/member-1 — capture stale/)).toBeVisible();
  expect(screen.getAllByText("Known declared requirement; current assessment unavailable").length).toBeGreaterThan(0);
  expect(screen.getByText(/condition false/)).toBeInTheDocument();
  expect(screen.getByText(/limited_sample; source demo-source-1/)).toBeVisible();
  expect(screen.getByText(/local package unverified/)).toBeVisible();
  const sent=client.getDungeonPreparation.mock.calls[0]?.[0];if(!sent)throw Error("Expected explicit preparation request");
  expect(JSON.parse(sent).subject).toEqual({kind:"party",party_id:"preparation-party"});
  const checklist=screen.getByRole("checkbox",{name:/Review missing or stale participant/});fireEvent.click(checklist);expect(checklist).toBeChecked();
  expect(screen.getAllByText(/Manual local checklist/)).toHaveLength(2);
});
test("identity refresh permits newer work and late old result cannot clear busy or replace report",async()=>{
  const client=mock();let oldResolve:((result:{ok:true;value:PreparationReport})=>void)|undefined;
  let newResolve:typeof oldResolve;
  client.getDungeonPreparation.mockImplementationOnce(()=>new Promise(resolve=>{oldResolve=resolve;})).mockImplementationOnce(()=>new Promise(resolve=>{newResolve=resolve;}));
  const view=render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);await ready();
  fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  expect(screen.getByRole("button",{name:"Inspect preparation gaps"})).toBeDisabled();
  view.rerender(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={1}/>);await ready();
  fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  await act(async()=>{oldResolve?.({ok:true,value:{...report,revision:"old-source"}});});
  expect(screen.getByRole("button",{name:"Inspect preparation gaps"})).toBeDisabled();expect(screen.queryByText(/old-source/)).toBeNull();
  await act(async()=>{newResolve?.({ok:true,value:{...report,revision:"new-source"}});});
  await screen.findByText(/Content: new-source/);expect(screen.getByRole("button",{name:"Inspect preparation gaps"})).toBeEnabled();
});
test("changing local input discards pending results",async()=>{
  const client=mock();let resolvePending:((result:{ok:true;value:PreparationReport})=>void)|undefined;
  client.getDungeonPreparation.mockImplementationOnce(()=>new Promise(resolve=>{resolvePending=resolve;}));
  const view=render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);await ready();
  fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  fireEvent.click(screen.getByText("Advanced local reference editor"));
  fireEvent.change(screen.getByLabelText("Manually declared shared origin node"),{target:{value:"changed-origin"}});
  await act(async()=>{resolvePending?.({ok:true,value:report});});
  expect(screen.queryByText(/Imported member assessments/)).toBeNull();
  view.unmount();expect(client.getDungeonPreparation).toHaveBeenCalledTimes(1);
});
