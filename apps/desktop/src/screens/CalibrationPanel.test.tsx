import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { createClient } from "../commands";
import type { CommandAdapter } from "../commands/contract";
import { CalibrationPanel } from "./CalibrationPanel";
const report = {methodology_version:1, fixture:true, sample_count:0, median_ms:null, minimum_ms:null, maximum_ms:null, blended_ms:600000, uncertainty:"no_complete_samples", request:{baseline_ms:600000,baseline_source:"planning assumption"}, comparisons:[{session_id:"unknown-time",exclusion:"activity_not_completely_observed",observed_ms:null,difference_ms:null,outlier:false}],methodology:"Retained observations only."};
test("comparison transmits explicit assumptions and exposes excluded sparse samples", async()=> {
  const calls: unknown[]=[];
  const adapter:CommandAdapter={invoke:async(command,args)=>{calls.push({command,args});return report as never;}};
  render(<CalibrationPanel client={createClient(adapter)} profile="profile-a" detail={null} refreshKey={0} disabled={false} />);
  fireEvent.change(screen.getByLabelText("Route or quest stage"),{target:{value:"stage-a"}});
  fireEvent.change(screen.getByLabelText("Declared role"),{target:{value:"healer"}});
  fireEvent.change(screen.getByLabelText("Baseline source or assumption"),{target:{value:"planning assumption"}});
  fireEvent.click(screen.getByRole("button",{name:"Compare retained sessions"}));
  expect(await screen.findByText(/Complete comparable samples: 0/)).toHaveTextContent("SYNTHETIC");
  expect(screen.getByText(/unknown-time: excluded/)).toHaveTextContent("activity not completely observed");
  expect(calls).toContainEqual({command:"get_calibration",args:{profile_id:"profile-a",request:{conditions:{route_stage:"stage-a",level:1,role:"healer",party_size:1},baseline_ms:600000,baseline_source:"planning assumption"}}});
});
test("a previous profile response cannot restore a comparison",async()=>{
  let resolve!:(value:unknown)=>void; const pending=new Promise<unknown>(done=>{resolve=done;});
  const adapter:CommandAdapter={invoke:async()=>await pending as never}; const client=createClient(adapter);
  const view=render(<CalibrationPanel client={client} profile="a" detail={null} refreshKey={0} disabled={false}/>);
  fireEvent.click(screen.getByRole("button",{name:"Compare retained sessions"}));
  view.rerender(<CalibrationPanel client={client} profile="b" detail={null} refreshKey={0} disabled={false}/>);
  resolve(report);
  await waitFor(()=>expect(screen.getByRole("button",{name:"Compare retained sessions"})).toBeEnabled());
  expect(screen.queryByText(/Complete comparable samples:/)).not.toBeInTheDocument();
});
test("comparing is the one primary action; saving an association is secondary (G5a #13)",()=>{
  const adapter:CommandAdapter={invoke:async()=>report as never};
  const detail={analytics:{summary:{session:"session-a"}}} as never;
  const {container}=render(<CalibrationPanel client={createClient(adapter)} profile="a" detail={detail} refreshKey={0} disabled={false}/>);
  expect(container.querySelectorAll(".ui-button--primary")).toHaveLength(1);
  expect(screen.getByRole("button",{name:"Compare retained sessions"})).toHaveClass("ui-button--primary");
  expect(screen.getByRole("button",{name:"Save association for session-a"})).toHaveClass("ui-button--secondary");
});
