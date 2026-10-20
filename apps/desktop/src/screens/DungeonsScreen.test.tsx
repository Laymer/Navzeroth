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
  return {getAuthoredCatalog:vi.fn(async()=>({ok:true,value:{revision:null,records:[],total:0,next_offset:null}})),listProfiles:vi.fn(async()=>({ok:true,value:[{id:firstMember.profile_id,display_name:"Synthetic member",data_mode:mode}]})),
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
  expect(screen.getAllByText(/Known declared requirement; current assessment unavailable/).length).toBeGreaterThan(0);
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
test.each(["restricted","unknown","known"] as const)("authored travel displays %s without inventing an ordered route",async status=>{
  const client=mock();client.getDungeonPreparation.mockResolvedValue({ok:true,value:{...report,authored_travel:{declaration:{observation:status==="known"?{status,value:[]}:{status},evidence:["travel-evidence"]},edges:[]}}} as {ok:true;value:PreparationReport});
  render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);await ready();fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  expect(await screen.findByRole("region",{name:"Authored travel dependencies"})).toHaveTextContent(`Availability: ${status}`);
  if(status==="known")expect(screen.getByText('The authored list is explicitly empty.')).toBeInTheDocument();
  else expect(screen.queryByText('The authored list is explicitly empty.')).not.toBeInTheDocument();
});

test("large source lists retain all references and page visible rows with bounded origin lookups",async()=>{
  const client={...mock(),getRecordOrigins:vi.fn(async(text:string)=>({ok:true,value:{revision:JSON.parse(text).revision,origins:JSON.parse(text).records.map(()=>[])}}))};
  const quest=firstMember.quests[0];const reward=firstMember.rewards[0];if(!quest||!reward)throw Error("Fixture requires quest and reward");
  const member={...firstMember,quests:Array.from({length:91},(_,i)=>({...quest,quest:`source-quest-${i}`,assessment:null})),rewards:Array.from({length:1024},(_,i)=>({...reward,source:`source-reward-${i}`}))};
  client.getDungeonPreparation.mockResolvedValue({ok:true,value:{...report,members:[member]}});
  render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);await ready();
  fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  expect(await screen.findByText(/Showing 1–24 of 1024 rewards/)).toBeInTheDocument();
  expect(screen.getByText(/source-reward-23/)).toBeInTheDocument();
  expect(screen.queryByText(/source-reward-24;/)).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Next rewards"}));
  expect(screen.getByText(/Showing 25–48 of 1024 rewards/)).toBeInTheDocument();
  expect(screen.getByText(/source-reward-24;/)).toBeInTheDocument();
  expect(screen.queryByText(/source-reward-0;/)).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Next quests"}));
  expect(screen.getByText(/Showing 25–48 of 91 quests/)).toBeInTheDocument();
  for(let i=0;i<41;i++)fireEvent.click(screen.getByRole("button",{name:"Next rewards"}));
  expect(screen.getByText(/Showing 1009–1024 of 1024 rewards/)).toBeInTheDocument();
  expect(screen.getByText(/source-reward-1023;/)).toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Next rewards"})).toBeDisabled();
  fireEvent.click(screen.getByRole("button",{name:"Previous rewards"}));
  expect(screen.getByText(/Showing 985–1008 of 1024 rewards/)).toBeInTheDocument();
  await waitFor(()=>expect(client.getRecordOrigins).toHaveBeenCalled());
  for(const [text] of client.getRecordOrigins.mock.calls){const refs=JSON.parse(text).records;expect(refs.length).toBeLessThanOrEqual(48);}
});

test("schema3 source guidance renders complete UTF8 derivation text with unknown readiness",async()=>{
  const value=structuredClone(report);const pkg=value.package;if(!pkg||pkg.checks.observation.status!=="known"||!pkg.checks.observation.value[0])throw Error("Fixture requires a declared check");
  const label="é".repeat(256);pkg.schema_version=3;pkg.checks.observation.value[0].label=label;
  for(const member of value.members)for(const check of member.checks)check.condition={availability:"unknown",evaluation:null};
  const client=mock();client.getDungeonPreparation.mockResolvedValue({ok:true,value});
  render(<DungeonsScreen client={client as unknown as CommandClient} refreshKey={0}/>);await ready();
  fireEvent.click(screen.getByRole("button",{name:"Inspect preparation gaps"}));
  const labels=await screen.findAllByText(label,{exact:false});
  expect(labels.length).toBeGreaterThan(0);for(const row of labels){expect(row).toHaveTextContent(label);expect(row).toHaveTextContent("unknown");}
});
