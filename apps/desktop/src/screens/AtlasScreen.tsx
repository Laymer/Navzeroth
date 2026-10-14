import { useEffect, useMemo, useRef, useState } from "react";
import type { AppError, AtlasView, CommandClient, SnapshotMarker } from "../commands";
import type { AtlasImage } from "../commands/contract";
import "../atlas/atlas.css";
import { MapView } from "../atlas/MapView";
import {
  APPROXIMATE_LABEL,
  DEMO_LABEL,
  FLIGHT_TOGGLE_LABEL,
  ROADS_TOGGLE_LABEL,
  describeAge,
  describeGap,
  describeLayerAvailability,
  describeNotice,
  describePin,
  describePinAccess,
  describeUncertainty,
  describeUtc,
} from "../atlas/describe";
import { chooseLayer, type LayerToggles } from "../atlas/layers";
import { MARKER_STATE_LABELS, markerState, placementOf } from "../atlas/markerState";
import type { Point } from "../atlas/viewport";

const PIN_PAGE_SIZE = 200;

type Load =
  | { phase: "loading" }
  | { phase: "error"; error: AppError }
  | { phase: "ready"; view: AtlasView; markers: SnapshotMarker[] };

export function AtlasScreen({ client, refreshKey = 0 }: { client: CommandClient; refreshKey?: number }) {
  const [refresh, setRefresh] = useState(0);
  const [load, setLoad] = useState<Load>({ phase: "loading" });
  const [refreshError, setRefreshError] = useState<AppError | null>(null);

  useEffect(() => {
    let live = true;
    void (async () => {
      const [view, markers] = await Promise.all([client.queryAtlas(null), client.getSnapshotMarkers()]);
      if (!live) return;
      const error = !view.ok ? view.error : !markers.ok ? markers.error : null;
      if (error !== null) {
        setRefreshError(error);
        setLoad((previous) => previous.phase === "ready" ? previous : { phase: "error", error });
        return;
      }
      if (!view.ok || !markers.ok) return;
      setRefreshError(null);
      setLoad({ phase: "ready", view: view.value, markers: markers.value });
    })();
    return () => {
      live = false;
    };
  }, [client, refreshKey, refresh]);

  return (
    <section aria-labelledby="atlas-heading" className="atlas">
      <h2 id="atlas-heading">Atlas</h2>
      <button type="button" onClick={() => setRefresh((n) => n + 1)}>Refresh atlas</button>
      {load.phase === "loading" && <p role="status">Loading the atlas…</p>}
      {load.phase === "error" && (
        <p role="alert">
          Atlas error {load.error.code}: {load.error.message}
          {load.error.retryable ? " (you can retry)" : ""}
        </p>
      )}
      {load.phase === "ready" && refreshError !== null && <p role="alert">
        Atlas refresh error {refreshError.code}: {refreshError.message} The previous atlas remains displayed. Refresh again to retry.
      </p>}
      {load.phase === "ready" && <AtlasReady view={load.view} markers={load.markers} />}
    </section>
  );
}

function AtlasReady({ view, markers }: { view: AtlasView; markers: SnapshotMarker[] }) {
  const [imageId, setImageId] = useState<string | null>(null);
  const image = view.images.find((i) => i.id === imageId) ?? view.images[0] ?? null;
  const marker = markers[0] ?? null;

  return (
    <>
      {view.data_mode === "demo" && (
        <p className="demo-label" role="note">
          {DEMO_LABEL}
        </p>
      )}
      <p className="dataset-line">
        Dataset {view.revision} ({view.namespace} namespace{view.fixture ? ", fixture" : ""})
      </p>
      {view.images.length > 1 && (
        <label>
          Image{" "}
          <select value={image?.id ?? ""} onChange={(event) => setImageId(event.target.value)}>
            {view.images.map((i) => (
              <option key={i.id} value={i.id}>
                {i.id}
              </option>
            ))}
          </select>
        </label>
      )}
      <MarkerPanel marker={marker} />
      {image === null ? (
        <p role="status">No atlas dataset is installed, so there is no map to show.</p>
      ) : (
        <ImageSection key={image.id} image={image} marker={marker} />
      )}
    </>
  );
}

function ImageSection({ image, marker }: { image: AtlasImage; marker: SnapshotMarker | null }) {
  const [toggles, setToggles] = useState<LayerToggles>({ roads: true, flight: false });
  const [failed, setFailed] = useState<ReadonlySet<string>>(new Set());
  const [selectedPin, setSelectedPin] = useState<string | null>(null);
  const [focus, setFocus] = useState<{ point: Point; nonce: number } | null>(null);

  const [search, setSearch] = useState("");
  const [kind, setKind] = useState("all");
  const [page, setPage] = useState(0);
  const [pendingFocus, setPendingFocus] = useState<string | null>(null);
  const pinButtons = useRef(new Map<string, HTMLButtonElement>());
  const pins = image.pins.filter((pin) =>
    (kind === "all" || pin.kind.kind === kind) &&
    `${pin.label} ${pin.id}`.toLowerCase().includes(search.trim().toLowerCase()),
  );
  const pageCount = Math.max(1, Math.ceil(pins.length / PIN_PAGE_SIZE));
  const currentPage = Math.min(page, pageCount - 1);
  const pageStart = currentPage * PIN_PAGE_SIZE;
  const pagePins = pins.slice(pageStart, pageStart + PIN_PAGE_SIZE);
  const filteredImage = { ...image, pins: pagePins };
  useEffect(() => {
    if (pendingFocus !== null) {
      pinButtons.current.get(pendingFocus)?.focus();
      setPendingFocus(null);
    }
  }, [pendingFocus, currentPage]);
  const layer = useMemo(() => chooseLayer(image, toggles, failed), [image, toggles, failed]);
  const placement = placementOf(marker);
  const drawn = placement !== null && placement.image === image.id ? placement : null;
  const stale = marker !== null && marker.freshness.status === "stale";

  return (
    <>
      <fieldset className="layers">
        <legend>Layers</legend>
        <label>
          <input
            type="checkbox"
            checked={toggles.roads}
            onChange={(event) => setToggles({ ...toggles, roads: event.target.checked })}
          />{" "}
          {ROADS_TOGGLE_LABEL}
        </label>
        <label>
          <input
            type="checkbox"
            checked={toggles.flight}
            onChange={(event) => setToggles({ ...toggles, flight: event.target.checked })}
          />{" "}
          {FLIGHT_TOGGLE_LABEL}
        </label>
        <p className="layer-note">
          {layer === null
            ? "Showing: schematic frame (no imagery)."
            : `Showing layer: ${layer.id} (${layer.kind.replace(/_/g, " ")}).`}{" "}
          Each layer is a full image; the flight network replaces the roads layer when both are on.
        </p>
        {image.layers
          .filter((l) => l.availability !== "available" || failed.has(l.id))
          .map((l) => (
            <p key={l.id} className="layer-note" role="note">
              Layer {l.id}: {l.availability !== "available" ? describeLayerAvailability(l) : "the image failed to load"}.
            </p>
          ))}
      </fieldset>
      <MapView
        image={filteredImage}
        layer={layer}
        onLayerError={(id) => setFailed((previous) => new Set(previous).add(id))}
        marker={drawn}
        markerStale={stale}
        selectedPin={selectedPin}
        focus={focus}
      />
      <section aria-labelledby="pins-heading">
        <h3 id="pins-heading">Reference pins</h3>
        <p>Reference geography; these pins do not indicate player unlocks.</p>
        <label>Search reference pins <input value={search} maxLength={256} onChange={(event) => { setSearch(event.target.value); setPage(0); }} /></label>
        <label>Point type <select value={kind} onChange={(event) => { setKind(event.target.value); setPage(0); }}>
          <option value="all">All types</option>
          {kind !== "all" && !image.pins.some((pin) => pin.kind.kind === kind) &&
            <option value={kind}>{kind.replace(/_/g, " ")} (absent in this revision)</option>}
          {[...new Set(image.pins.map((pin) => pin.kind.kind))].sort().map((value) => <option key={value} value={value}>{value.replace(/_/g, " ")}</option>)}
        </select></label>
        <p role="status">{pins.length} of {image.pins.length} reference pins shown.</p>
        {selectedPin !== null && !image.pins.some((pin) => pin.id === selectedPin)
          ? <p role="note">The selected pin is no longer present in this atlas revision.</p>
          : selectedPin !== null && !pins.some((pin) => pin.id === selectedPin)
            ? <p role="note">The selected pin is hidden by the current filters.</p> : null}
        {selectedPin !== null && pins.some((pin) => pin.id === selectedPin) && !pagePins.some((pin) => pin.id === selectedPin) &&
          <p role="note">The selected pin is on another page. Change pages or search to show it.</p>}
        {pageCount > 1 && <div className="pin-pagination" role="group" aria-label="Reference pin pages">
          <p role="status">Map and list display pins {pageStart + 1}–{Math.min(pageStart + PIN_PAGE_SIZE, pins.length)} of {pins.length} matches. Other reference pins are on the remaining pages.</p>
          <button type="button" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>Previous pin page</button>
          <span>Page {currentPage + 1} of {pageCount}</span>
          <button type="button" disabled={currentPage === pageCount - 1} onClick={() => setPage(currentPage + 1)}>Next pin page</button>
        </div>}
        {pins.length === 0 ? (
          <p>{image.pins.length === 0 ? "This image has no pins." : "No reference pins match these filters."}</p>
        ) : (
          <ul className="pins">
            {pagePins.map((pin, pageIndex) => (
              <li key={pin.id}>
                <strong>{pin.label}</strong>. {describePin(pin)}.
                {describePinAccess(pin) !== null && <> {describePinAccess(pin)}.</>}{" "}
                <button
                  type="button"
                  ref={(node) => { if (node) pinButtons.current.set(pin.id, node); else pinButtons.current.delete(pin.id); }}
                  onKeyDown={(event) => {
                    const index = pageStart + pageIndex;
                    const target = event.key === "ArrowDown" ? (index + 1) % pins.length
                      : event.key === "ArrowUp" ? (index + pins.length - 1) % pins.length
                      : event.key === "Home" ? 0 : event.key === "End" ? pins.length - 1 : null;
                    if (target !== null) {
                      event.preventDefault();
                      const targetPin = pins[target];
                      if (targetPin) {
                        setPage(Math.floor(target / PIN_PAGE_SIZE));
                        setPendingFocus(targetPin.id);
                      }
                    }
                  }}
                  aria-pressed={selectedPin === pin.id}
                  onClick={() => {
                    setSelectedPin(pin.id);
                    setFocus({ point: pin.pixel, nonce: (focus?.nonce ?? 0) + 1 });
                  }}
                >
                  Show {pin.label} on the map
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section aria-labelledby="coverage-heading">
        <h3 id="coverage-heading">Coverage and sources</h3>
        <p>
          Calibration: {image.calibration_status}
          {image.calibration_confidence === null ? "" : `, confidence ${image.calibration_confidence.replace(/_/g, " ")}`}
          {image.approximate ? ` (${APPROXIMATE_LABEL})` : ""}.
        </p>
        <p>Not verified or not shown: {image.gaps}</p>
        <p>{image.attribution}</p>
      </section>
    </>
  );
}

function MarkerPanel({ marker }: { marker: SnapshotMarker | null }) {
  const state = markerState(marker);
  const projection = marker?.projection ?? null;
  return (
    <section aria-labelledby="marker-heading" className="marker-panel">
      <h3 id="marker-heading">Snapshot position</h3>
      <p className="badge" role="status">
        {marker === null ? "No snapshot imported" : `Last imported: ${describeAge(marker.freshness)}`}
      </p>
      <p>{MARKER_STATE_LABELS[state]}</p>
      {marker !== null && marker.freshness.status === "stale" && (
        <p role="note" className="stale-note">
          Stale: this position is older than 30 minutes and may no longer be where the character is.
        </p>
      )}
      {marker !== null && marker.freshness.status === "uncertain" && (
        <p role="note">{describeUncertainty(marker.freshness.reason)}</p>
      )}
      {projection !== null && projection.outcome === "gap" && <p role="note">{describeGap(projection)}</p>}
      {projection !== null &&
        projection.outcome === "placed" &&
        projection.notices.map((notice) => (
          <p key={notice.notice} role="note">
            {describeNotice(notice)}
          </p>
        ))}
      {marker !== null && marker.observed_at_utc_ms !== null && (
        <p>Captured (UTC): {describeUtc(marker.observed_at_utc_ms)}</p>
      )}
    </section>
  );
}
