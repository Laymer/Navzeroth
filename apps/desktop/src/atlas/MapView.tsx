import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { AtlasImage, AtlasLayer } from "../commands/contract";
import { APPROXIMATE_LABEL, describePin } from "./describe";
import type { Placement } from "./markerState";
import {
  ZOOM_STEP,
  applyKeyAction,
  clampViewport,
  fitViewport,
  frameToScreen,
  keyAction,
  panBy,
  zoomAt,
  type MapKeyAction,
  type Point,
  type Size,
  type Viewport,
} from "./viewport";

export interface MapViewProps {
  image: AtlasImage;
  /** The raster layer to draw; null selects the schematic fallback. */
  layer: AtlasLayer | null;
  onLayerError: (layerId: string) => void;
  marker: Placement | null;
  markerStale: boolean;
  selectedPin: string | null;
  /** Centre the view on this frame point whenever `nonce` changes. */
  focus: { point: Point; nonce: number } | null;
}

const DEFAULT_SIZE: Size = { w: 640, h: 480 };

/** A round grid step (1, 2 or 5 times a power of ten) giving about eight lines across. */
export function gridStep(extent: number): number {
  const rough = extent / 8;
  if (!(rough > 0)) return 1;
  const power = 10 ** Math.floor(Math.log10(rough));
  const unit = rough / power;
  return (unit < 1.5 ? 1 : unit < 3.5 ? 2 : unit < 7.5 ? 5 : 10) * power;
}

export function MapView({ image, layer, onLayerError, marker, markerStale, selectedPin, focus }: MapViewProps) {
  const frame = useMemo<Size>(() => ({ w: image.width_px, h: image.height_px }), [image.width_px, image.height_px]);
  const regionRef = useRef<HTMLDivElement>(null);
  const helpId = useId();
  const [size, setSize] = useState<Size>(DEFAULT_SIZE);
  const [view, setView] = useState<Viewport>(() => fitViewport(frame, DEFAULT_SIZE));
  // Until the user moves the map, resizing keeps it fitted.
  const touched = useRef(false);
  const drag = useRef<Point | null>(null);

  useEffect(() => {
    const el = regionRef.current;
    if (!el) return;
    const measure = () => {
      const rect = el.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        setSize((s) => (s.w === rect.width && s.h === rect.height ? s : { w: rect.width, h: rect.height }));
      }
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    setView((v) => (touched.current ? clampViewport(v, frame, size) : fitViewport(frame, size)));
  }, [frame, size]);

  useEffect(() => {
    const el = regionRef.current;
    if (!el) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = el.getBoundingClientRect();
      const anchor = { x: event.clientX - rect.left, y: event.clientY - rect.top };
      touched.current = true;
      setView((v) => zoomAt(v, event.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP, anchor, frame, size));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [frame, size]);

  useEffect(() => {
    if (focus === null) return;
    touched.current = true;
    const target = focus.point;
    setView((v) => applyKeyAction(v, { type: "centre" }, frame, size, target));
  }, [focus, frame, size]);

  const markerPoint = marker === null ? null : marker.pixel;
  const act = (action: MapKeyAction) => {
    touched.current = action.type !== "fit";
    setView((v) => applyKeyAction(v, action, frame, size, markerPoint));
  };

  const origin = frameToScreen(view, { x: 0, y: 0 });
  const far = frameToScreen(view, { x: frame.w, y: frame.h });
  const step = gridStep(Math.max(frame.w, frame.h));
  const gridX = Array.from({ length: Math.floor(frame.w / step) + 1 }, (_, i) => i * step);
  const gridY = Array.from({ length: Math.floor(frame.h / step) + 1 }, (_, i) => i * step);
  const markerScreen = marker === null ? null : frameToScreen(view, marker.pixel);

  return (
    <div className="map">
      <div className="map-toolbar" role="toolbar" aria-label="Map view controls">
        <button type="button" onClick={() => act({ type: "zoom", factor: ZOOM_STEP })}>
          Zoom in
        </button>
        <button type="button" onClick={() => act({ type: "zoom", factor: 1 / ZOOM_STEP })}>
          Zoom out
        </button>
        <button type="button" onClick={() => act({ type: "fit" })}>
          Fit to frame
        </button>
        <button type="button" disabled={marker === null} onClick={() => act({ type: "centre" })}>
          Centre on marker
        </button>
      </div>
      {layer === null && (
        <p className="map-note" role="status">
          Schematic view: no imagery could be shown, so only the frame outline, grid, pins and marker are drawn.
        </p>
      )}
      <div
        ref={regionRef}
        className="map-region"
        role="group"
        tabIndex={0}
        aria-label={`Atlas map of ${image.id}`}
        aria-describedby={helpId}
        onKeyDown={(event) => {
          if (event.ctrlKey || event.metaKey || event.altKey) return;
          const action = keyAction(event.key);
          if (action === null) return;
          event.preventDefault();
          act(action);
        }}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          drag.current = { x: event.clientX, y: event.clientY };
          event.currentTarget.setPointerCapture?.(event.pointerId);
        }}
        onPointerMove={(event) => {
          const last = drag.current;
          if (last === null) return;
          drag.current = { x: event.clientX, y: event.clientY };
          touched.current = true;
          setView((v) => panBy(v, event.clientX - last.x, event.clientY - last.y, frame, size));
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
      >
        {layer !== null && layer.url !== null && (
          <div
            className="map-layer"
            style={{
              width: frame.w,
              height: frame.h,
              transform: `translate(${view.pan.x}px, ${view.pan.y}px) scale(${view.zoom})`,
            }}
          >
            <img src={layer.url} alt="" draggable={false} onError={() => onLayerError(layer.id)} />
          </div>
        )}
        <svg className="map-overlay" width={size.w} height={size.h} role="img" aria-label={overlayLabel(image, marker)}>
          {layer === null && (
            <g className="schematic" data-testid="schematic">
              <rect x={origin.x} y={origin.y} width={far.x - origin.x} height={far.y - origin.y} className="schematic-fill" />
              {gridX.map((x) => {
                const sx = frameToScreen(view, { x, y: 0 }).x;
                return <line key={`x${x}`} x1={sx} y1={origin.y} x2={sx} y2={far.y} className="schematic-grid" />;
              })}
              {gridY.map((y) => {
                const sy = frameToScreen(view, { x: 0, y }).y;
                return <line key={`y${y}`} x1={origin.x} y1={sy} x2={far.x} y2={sy} className="schematic-grid" />;
              })}
            </g>
          )}
          <rect
            x={origin.x}
            y={origin.y}
            width={far.x - origin.x}
            height={far.y - origin.y}
            className={layer === null ? "frame frame-schematic" : "frame"}
          />
          {image.pins.map((pin) => {
            const at = frameToScreen(view, pin.pixel);
            return (
              <g key={pin.id} transform={`translate(${at.x} ${at.y})`} className="pin" data-selected={pin.id === selectedPin}>
                <title>{`${pin.label}: ${describePin(pin)}`}</title>
                {pin.id === selectedPin && <circle r={13} className="pin-selected" />}
                <polygon points="0,-9 8,6 -8,6" />
              </g>
            );
          })}
          {marker !== null && markerScreen !== null && (
            <g transform={`translate(${markerScreen.x} ${markerScreen.y})`} className="marker" data-stale={markerStale}>
              {marker.approximate && (
                <circle r={18} className="uncertainty-ring" data-testid="uncertainty-ring" />
              )}
              <circle r={6} className={markerStale ? "marker-dot marker-dot-stale" : "marker-dot"} />
              <path d="M-9 0H9M0 -9V9" className="marker-cross" />
              <text x={22} y={4} className="marker-text">
                {marker.approximate ? APPROXIMATE_LABEL : "calibrated"}
              </text>
            </g>
          )}
        </svg>
      </div>
      <p id={helpId} className="map-help">
        Keyboard (map focused): arrow keys pan, + and - zoom, 0 fits the whole frame, c centres on the marker. Drag to
        pan and use the mouse wheel to zoom.
      </p>
    </div>
  );
}

function overlayLabel(image: AtlasImage, marker: Placement | null): string {
  const pins = `${image.pins.length} reference pin${image.pins.length === 1 ? "" : "s"}`;
  return marker === null
    ? `Map overlay with ${pins} and no snapshot marker`
    : `Map overlay with ${pins} and the last imported snapshot position${marker.approximate ? `, ${APPROXIMATE_LABEL}` : ""}`;
}
