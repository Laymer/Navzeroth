import type { RouteInspection as Inspection } from "../commands/routes";
export function RouteInspection({ value }: { value: Inspection }) {
  return <section aria-label="Route inspection"><h3>Selected travel and final stopping route</h3>
    <p>Endpoint links are schematic connections. Road geometry is unavailable. Estimates include declared waits and service time.</p>
    {value.fixture && <p>SYNTHETIC — fixture route.</p>}
    {value.frames.map((frame) => <figure key={frame.id}><figcaption>{frame.id}: exact dataset projection; schematic links</figcaption>
      <svg role="img" aria-label={`Schematic route on ${frame.id}`} viewBox={`0 0 ${frame.width_px} ${frame.height_px}`} style={{ width: "100%", maxHeight: 360, background: "#152536" }}>
        {value.hops.map((h, index) => h.schematic_connection && h.from.outcome === "placed" && h.to.outcome === "placed" && h.from.image === frame.id && h.to.image === frame.id
          ? <g key={index}><title>{h.hop.from.id} to {h.hop.to.id}: {h.hop.mode}, {h.hop.cost_ms} ms, schematic</title><line x1={h.from.pixel.x} y1={h.from.pixel.y} x2={h.to.pixel.x} y2={h.to.pixel.y} stroke="#ffce63" strokeWidth={3} strokeDasharray="8 5" /><circle cx={h.to.pixel.x} cy={h.to.pixel.y} r={5} fill="#ffce63" /></g> : null)}
      </svg></figure>)}
    <ol>{value.hops.map((h, index) => <li key={index}>{h.stopping ? "Final stopping route" : `Selected leg ${h.leg + 1}`}: {h.hop.from.id} → {h.hop.to.id}; {h.hop.mode}; {h.hop.cost_ms} ms; {h.hop.status}.
      {h.geometry_gap && <p>{h.geometry_gap}</p>}
      <details><summary>Endpoint placement, requirements and uncertainty</summary><pre>{JSON.stringify({ from: h.from, to: h.to, requirements: h.requirements, expected: h.hop.expected_ms, wait: h.hop.wait_ms, range: h.hop.range, resource: h.hop.consumes_resource, cooldown: h.hop.cooldown_ms }, null, 2)}</pre></details>
    </li>)}</ol>{value.hops.length === 0 && <p>No travel hops; origin already at stopping node.</p>}
  </section>;
}
