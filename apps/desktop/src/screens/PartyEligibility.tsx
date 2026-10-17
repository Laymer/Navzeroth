import { Field, Panel } from "../ui";
import { CatalogPager, OffPageOptions } from "./CatalogPager";
import { useEffect, useState } from "react";
import type { CommandClient } from "../commands";
import type { AppError, CatchUp, PartyQuestCatalog, PartyQuestReport } from "../commands/contract";
import { describeAge } from "../atlas/describe";

const JOINT = {
  jointly_eligible: "All participants meet the declared acceptance conditions.",
  eligible_for_some: "Some participants need catch-up progress before acceptance.",
  all_behind: "All participants need catch-up progress before acceptance.",
  blocked: "At least one participant is blocked by a declared condition.",
  unknown: "Joint eligibility is unknown because required observations are unavailable.",
  missing_or_old_snapshots: "Joint advice is uncertain: a participant's snapshot is missing, stale or has uncertain age.",
};
export function describeCatchUp(value: CatchUp): string {
  switch (value.kind) {
    case "single": switch (value.requirement.kind) {
      case "level": return `Reach level ${value.requirement.level}`;
      case "quest": return `Quest ${value.requirement.quest}: ${value.requirement.state.replaceAll("_", " ")}`;
      case "other": return "Satisfy another condition";
    } break;
    case "all_of": return `All required: ${value.items.map(describeCatchUp).join("; ")}`;
    case "one_of": return `Choose an alternative: ${value.alternatives.map(describeCatchUp).join("; ")}`;
  }
}
export function PartyEligibility({ client, partyId, refreshKey }: { client: CommandClient; partyId: string; refreshKey: number }) {
  const [catalog, setCatalog] = useState<PartyQuestCatalog | null>(null);
  const [quest, setQuest] = useState("");
  const [report, setReport] = useState<PartyQuestReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    let active = true;
    void client.getPartyQuests(partyId).then((result) => {
      if (!active) return;
      setLoading(false); setReport(null);
      if (!result.ok) { setCatalog(null); setError(result.error); return; }
      setCatalog(result.value); setError(null);
      setQuest((old) => result.value.quests.includes(old) ? old : result.value.quests[0] ?? "");
    });
    return () => { active = false; };
  }, [client, partyId, refreshKey]);
  useEffect(() => {
    let active = true;
    if (!catalog?.revision || !quest) return;
    void client.getPartyEligibility(partyId, catalog.revision, quest).then((result) => {
      if (!active) return;
      setLoading(false);
      if (!result.ok) { setReport(null); setError(result.error); return; }
      setReport(result.value); setError(null);
    });
    return () => { active = false; };
  }, [client, partyId, catalog, quest]);
  return <Panel title="Quest eligibility" level={3}>
    {loading && <p role="status">Checking quest eligibility…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {catalog?.revision === null && <p>No active content matches this party. Install and activate a compatible content revision first.</p>}
    <CatalogPager client={client} subject={{kind:"party",party_id:partyId}} page={catalog?.pages?.quests} label="quests" onPage={p=>{setCatalog(old=>old?{...old,quests:p.records.map(r=>(r as {id:string}).id),pages:{...old.pages,quests:p}}:old);}}/>
{catalog?.revision && catalog.quests.length === 0 && <p>The active content contains no quest definitions.</p>}
    {catalog?.revision && catalog.quests.length > 0 && <>
      {catalog.fixture && <p className="demo-label play-note">SYNTHETIC — engineering quest rules, not verified gameplay advice</p>}
      <p>Content revision: {catalog.revision}. Evaluates declared quest predicates only. Sharing, class/race/faction restrictions and exclusive choices require separate verified data.</p>
      <Field label="Quest to check" help="Evaluates the quest's declared predicates for every member from imported snapshots. It does not check sharing or class restrictions.">{(c) => <select {...c} value={quest} onChange={(event) => { setQuest(event.target.value); setReport(null); setError(null); setLoading(true); }}>
        <OffPageOptions selected={[quest]} visible={catalog.quests}/>{catalog.quests.map((id) => <option key={id}>{id}</option>)}
      </select>}</Field>
    </>}
    {report && report.quest === quest && report.revision === catalog?.revision && <>
      <p>{JOINT[report.joint_status]}</p>
      <ul>{report.participants.map((participant) => <li key={participant.character}>
        <h4>{participant.character}</h4>
        {participant.freshness && <p>{describeAge(participant.freshness)}</p>}
        {participant.assessment === null ? <p>Missing imported snapshot; eligibility is unknown.</p> : <>
          <p>Acceptance: {participant.assessment.acceptance}. Progress: {participant.assessment.progress.replaceAll("_", " ")}. Completion: {participant.assessment.completion}. Turn-in: {participant.assessment.turn_in.replaceAll("_", " ")}.</p>
          {participant.assessment.catch_up && <p>{describeCatchUp(participant.assessment.catch_up)}</p>}
          <ul>{participant.assessment.reasons.map((reason, index) => <li key={index}>{reason.phase.replaceAll("_", " ")}: {reason.message}</li>)}</ul>
        </>}
      </li>)}</ul>
    </>}
  </Panel>;
}
