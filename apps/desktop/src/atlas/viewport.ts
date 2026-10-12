// Pure viewport math for the atlas (plan section 12.2). Frame space is the
// image's full-frame pixels; screen space is CSS pixels inside the map region.
// screen = frame * zoom + pan. No DOM access here.

export interface Size {
  w: number;
  h: number;
}
export interface Point {
  x: number;
  y: number;
}
export interface Viewport {
  zoom: number;
  pan: Point;
}

/** Zoom may go this far below the fit zoom, and this many times above it. */
export const MIN_ZOOM_FACTOR = 0.5;
export const MAX_ZOOM_FACTOR = 16;
export const KEY_PAN_STEP_PX = 48;
export const ZOOM_STEP = 1.25;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** The zoom at which the whole frame is visible. Never zero or negative. */
export function fitZoom(frame: Size, container: Size): number {
  if (frame.w <= 0 || frame.h <= 0 || container.w <= 0 || container.h <= 0) return 1;
  return Math.min(container.w / frame.w, container.h / frame.h);
}

export function clampZoom(zoom: number, frame: Size, container: Size): number {
  const fit = fitZoom(frame, container);
  return clamp(Number.isFinite(zoom) ? zoom : fit, fit * MIN_ZOOM_FACTOR, fit * MAX_ZOOM_FACTOR);
}

function clampAxis(pan: number, frameLen: number, containerLen: number, zoom: number): number {
  const scaled = frameLen * zoom;
  // A frame smaller than the region is centred; a larger one cannot leave a gap.
  if (scaled <= containerLen) return (containerLen - scaled) / 2;
  return clamp(pan, containerLen - scaled, 0);
}

/** Keep zoom in range and the frame covering the region (or centred when smaller). */
export function clampViewport(v: Viewport, frame: Size, container: Size): Viewport {
  const zoom = clampZoom(v.zoom, frame, container);
  return {
    zoom,
    pan: {
      x: clampAxis(v.pan.x, frame.w, container.w, zoom),
      y: clampAxis(v.pan.y, frame.h, container.h, zoom),
    },
  };
}

export function fitViewport(frame: Size, container: Size): Viewport {
  return clampViewport({ zoom: fitZoom(frame, container), pan: { x: 0, y: 0 } }, frame, container);
}

export function frameToScreen(v: Viewport, p: Point): Point {
  return { x: p.x * v.zoom + v.pan.x, y: p.y * v.zoom + v.pan.y };
}
export function screenToFrame(v: Viewport, p: Point): Point {
  return { x: (p.x - v.pan.x) / v.zoom, y: (p.y - v.pan.y) / v.zoom };
}

export function panBy(v: Viewport, dx: number, dy: number, frame: Size, container: Size): Viewport {
  return clampViewport({ zoom: v.zoom, pan: { x: v.pan.x + dx, y: v.pan.y + dy } }, frame, container);
}

/** Zoom by `factor`, keeping the frame point under `anchor` (screen space) fixed. */
export function zoomAt(v: Viewport, factor: number, anchor: Point, frame: Size, container: Size): Viewport {
  const zoom = clampZoom(v.zoom * factor, frame, container);
  const under = screenToFrame(v, anchor);
  return clampViewport(
    { zoom, pan: { x: anchor.x - under.x * zoom, y: anchor.y - under.y * zoom } },
    frame,
    container,
  );
}

/** Keep the zoom and put `target` (frame space) at the centre of the region where possible. */
export function centerOn(v: Viewport, target: Point, frame: Size, container: Size): Viewport {
  return clampViewport(
    { zoom: v.zoom, pan: { x: container.w / 2 - target.x * v.zoom, y: container.h / 2 - target.y * v.zoom } },
    frame,
    container,
  );
}

export type MapKeyAction =
  | { type: "pan"; dx: number; dy: number }
  | { type: "zoom"; factor: number }
  | { type: "fit" }
  | { type: "centre" };

/** Keyboard map: arrows pan, + and - zoom, 0 fits, c centres on the marker. */
export function keyAction(key: string): MapKeyAction | null {
  switch (key) {
    // Arrow keys move the view in the direction of the arrow.
    case "ArrowLeft":
      return { type: "pan", dx: KEY_PAN_STEP_PX, dy: 0 };
    case "ArrowRight":
      return { type: "pan", dx: -KEY_PAN_STEP_PX, dy: 0 };
    case "ArrowUp":
      return { type: "pan", dx: 0, dy: KEY_PAN_STEP_PX };
    case "ArrowDown":
      return { type: "pan", dx: 0, dy: -KEY_PAN_STEP_PX };
    case "+":
    case "=":
      return { type: "zoom", factor: ZOOM_STEP };
    case "-":
    case "_":
      return { type: "zoom", factor: 1 / ZOOM_STEP };
    case "0":
      return { type: "fit" };
    case "c":
    case "C":
      return { type: "centre" };
    default:
      return null;
  }
}

/** Apply a key action. `marker` is the marker's frame position, if one is placed. */
export function applyKeyAction(
  v: Viewport,
  action: MapKeyAction,
  frame: Size,
  container: Size,
  marker: Point | null,
): Viewport {
  switch (action.type) {
    case "pan":
      return panBy(v, action.dx, action.dy, frame, container);
    case "zoom":
      return zoomAt(v, action.factor, { x: container.w / 2, y: container.h / 2 }, frame, container);
    case "fit":
      return fitViewport(frame, container);
    case "centre":
      return marker === null ? v : centerOn(v, marker, frame, container);
  }
}
