import type { ProgressionRecord } from "../../../../../schemas/v1/progression-api";
import { describeAge } from "../../atlas/describe";
import { describeObservation } from "../../observation";
import { describeEnumeration } from "./inputs";

export function ProgressionRecords({ records, origin }: { records: ProgressionRecord[]; origin: "Imported" | "Manual" }) {
  return <section aria-label={`${origin} progression observations`}>
    <h3>{origin} observations</h3>
    {records.length === 0 ? <p>No {origin.toLowerCase()} progression observations for this snapshot.</p> : <>
      <p>Each capture is separate. These records have not been merged or selected as the latest state.</p>
      {records.map(({ snapshot, freshness }) => <details key={snapshot.id}>
        <summary>{origin} capture {snapshot.id} — capture age: {describeAge(freshness)}</summary>
        <p>Individual observations may be older than this capture.</p>
        <p>Source: {snapshot.capture.source === "synthetic_fixture" ? "Synthetic fixture" : origin === "Manual" ? "Manual entry" : "Addon observation"}.</p>
        <dl>
          <dt>Money</dt><dd>{describeObservation(snapshot.money, (value) => `${value} copper`)}</dd>
          <dt>Available talent points</dt><dd>{describeObservation(snapshot.available_talent_points)}</dd>
          <dt>Spells and ranks</dt><dd>{describeObservation(snapshot.spells, describeEnumeration)}</dd>
          <dt>Allocated talents</dt><dd>{describeObservation(snapshot.talents, describeEnumeration)}</dd>
          <dt>Professions</dt><dd>{describeObservation(snapshot.professions, describeEnumeration)}</dd>
          <dt>Recipe observations</dt><dd>{describeObservation(snapshot.recipes, describeEnumeration)}</dd>
          <dt>Bag inventory</dt><dd>{describeObservation(snapshot.bags, describeEnumeration)}</dd>
          <dt>Bank inventory</dt><dd>{describeObservation(snapshot.bank, describeEnumeration)}</dd>
          <dt>Equipment enhancements</dt><dd>{Object.keys(snapshot.equipment_enhancements).length} slots recorded; unlisted slots unknown</dd>
        </dl>
      </details>)}
    </>}
  </section>;
}
