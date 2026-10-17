import { Button } from "../ui";
import { useMemo, useRef, useState } from "react";
import type { RouteInspection as Inspection } from "../commands/routes";
import { MapView } from "../atlas/MapView";
import type { Bounds } from "../atlas/viewport";
import { endpoints, routeBounds, ROUTE_PAGE_SIZE } from "./routeSegments";
/** A newly guarded response owns its own selection and projection state. */
export function RouteInspection({ value }: { value: Inspection }) {
  const owner = useRef(value); const epoch = useRef(0);
  if (owner.current !== value) { owner.current = value; epoch.current++; }
  return <RouteView key={epoch.current} value={value} />;
}
function RouteView({ value }: { value: Inspection }) {
  const [selected, setSelected] = useState<number | null>(value.hops.length ? 0 : null);
  const [page, setPage] = useState(0); const [frameIndex, setFrameIndex] = useState(() => Math.max(0, value.frames.findIndex((f) => value.hops[0] && endpoints(value, f, value.hops[0]) !== null)));
  const [fit, setFit] = useState<{ bounds: Bounds; nonce: number } | null>(null);
  const buttons = useRef(new Map<number, HTMLButtonElement>());
  const frame = value.frames[frameIndex];
  const start = page * ROUTE_PAGE_SIZE; const displayed = value.hops.slice(start, start + ROUTE_PAGE_SIZE);
  const fitted = useMemo(() => frame ? routeBounds(value, frame) : { bounds: null, count: 0 }, [value, frame]);
  const selectedBounds = frame && selected !== null ? routeBounds(value, frame, [selected]).bounds : null;
  const segments = frame ? displayed.flatMap((hop, offset) => { const points = endpoints(value, frame, hop); const index = start + offset; return points ? [{ index, from: points[0], to: points[1], selected: index === selected, label: `${hop.hop.from.id} to ${hop.hop.to.id}: ${hop.hop.mode}, ${hop.hop.cost_ms} ms` }] : []; }) : [];
  function choose(index: number, focus: boolean) {
    if (index < 0 || index >= value.hops.length) return;
    setSelected(index); setPage(Math.floor(index / ROUTE_PAGE_SIZE)); setFit(null);
    const hop = value.hops[index]; if (!hop) return; const destination = value.frames.findIndex((f) => endpoints(value, f, hop) !== null);
    if (destination >= 0) setFrameIndex(destination);
    if (focus) requestAnimationFrame(() => buttons.current.get(index)?.focus());
  }
  function movePage(next: number) { choose(Math.min(next * ROUTE_PAGE_SIZE, value.hops.length - 1), true); }
  return <section aria-label="Route inspection"><h3>Selected travel and final stopping route</h3>
    <p>Endpoint links are schematic connections. Road geometry is unavailable. Estimates include declared waits and service time.</p>
    <p>{value.context.build.edition}/{value.context.build.build}/{value.context.build.interface}, {value.context.namespace}. Snapshot {value.report.request.snapshot_id}; revision {value.report.request.revision}.</p>
    {value.fixture && <p className="play-note">SYNTHETIC — fixture route.</p>}
    <p>Planner termination: {value.report.result.termination}. Only returned hops are shown; missing or incomplete trace entries do not create connections.</p>
    {frame ? <figure><figcaption>{frame.id}: exact dataset projection; map {frame.map.map_id}, floor {frame.map.floor ?? "unrecorded"}, instance {frame.map.instance ?? "none"}; schematic links</figcaption>
      <p>Frame {frameIndex + 1} of {value.frames.length}. {segments.length} links displayed from this hop page; {fitted.count} same-frame links across all pages.</p>
      <Button variant="ghost" disabled={frameIndex === 0} onClick={() => { setFrameIndex((n) => n - 1); setFit(null); }}>Previous route frame</Button>
      <Button variant="ghost" disabled={frameIndex + 1 >= value.frames.length} onClick={() => { setFrameIndex((n) => n + 1); setFit(null); }}>Next route frame</Button>
      <Button disabled={fitted.bounds === null} onClick={() => { const bounds = fitted.bounds; if (bounds) setFit((old) => ({ bounds, nonce: (old?.nonce ?? 0) + 1 })); }}>Fit route endpoints in this frame</Button>
      <Button disabled={selectedBounds === null} onClick={() => { if (selectedBounds) setFit((old) => ({ bounds: selectedBounds, nonce: (old?.nonce ?? 0) + 1 })); }}>Fit selected hop endpoints</Button>
      <MapView key={frameIndex} image={{ ...frame, pins: [] }} layer={null} onLayerError={() => {}} marker={null} markerStale={false} selectedPin={null} focus={null} segments={segments} fit={fit} mapLabel={`Schematic route frame ${frame.id}`} />
    </figure> : <p>No route image frames. Ordered hops and placement gaps remain available below.</p>}
    {value.hops.length > 0 && <><p role="status">Hops {start + 1}–{start + displayed.length} of {value.hops.length}. Selected hop {selected === null ? "none" : selected + 1}.</p>
      <p>On a hop button: Up/Down selects the previous/next hop; Home/End selects the first/last; Page Up/Down moves by {ROUTE_PAGE_SIZE} hops.</p>
      <Button variant="ghost" disabled={page === 0} onClick={() => movePage(page - 1)}>Previous route hop page</Button>
      <Button variant="ghost" disabled={start + ROUTE_PAGE_SIZE >= value.hops.length} onClick={() => movePage(page + 1)}>Next route hop page</Button>
    </>}
    <ol className="route-hops" start={start + 1}>{displayed.map((h, offset) => { const index = start + offset; return <li key={index}>
      <Button ref={(el) => { if (el) buttons.current.set(index, el); else buttons.current.delete(index); }} aria-pressed={selected === index} onClick={() => choose(index, false)} onKeyDown={(event) => {
        if (event.ctrlKey || event.metaKey || event.altKey) return;
        const target = event.key === "ArrowUp" ? Math.max(0, index - 1) : event.key === "ArrowDown" ? Math.min(value.hops.length - 1, index + 1) : event.key === "Home" ? 0 : event.key === "End" ? value.hops.length - 1 : event.key === "PageUp" ? Math.max(0, index - ROUTE_PAGE_SIZE) : event.key === "PageDown" ? Math.min(value.hops.length - 1, index + ROUTE_PAGE_SIZE) : null;
        if (target !== null) { event.preventDefault(); choose(target, true); }
      }}>Select hop {index + 1}: {h.hop.from.id} → {h.hop.to.id}</Button>
      <p>{h.stopping ? "Final stopping route" : `Selected leg ${h.leg + 1}`}: {h.hop.from.id} → {h.hop.to.id}; {h.hop.mode}; {h.hop.cost_ms} ms; {h.hop.status}.</p>
      {h.geometry_gap && <p>{h.geometry_gap}</p>}
      {!h.geometry_gap && (!frame || endpoints(value, frame, h) === null) && <p>No schematic connection is displayed for this hop in the current frame.</p>}
      <details><summary>Endpoint placement, requirements and uncertainty</summary><pre>{JSON.stringify({ from: h.from, to: h.to, requirements: h.requirements, expected: h.hop.expected_ms, wait: h.hop.wait_ms, service: h.hop.service_ms, range: h.hop.range, stateful: h.hop.stateful, resource: h.hop.consumes_resource, cooldown: h.hop.cooldown_ms }, null, 2)}</pre></details>
    </li>; })}</ol>{value.hops.length === 0 && <p>No travel hops; inspect the reported stopping/trace result before inferring a connection.</p>}
    <details><summary>Original route decision trace and stopping result</summary><pre>{JSON.stringify({ termination: value.report.result.termination, alternatives: value.report.result.alternatives, trace: value.report.result.trace, stopping: value.report.result.stopping }, null, 2)}</pre></details>
  </section>;
}
