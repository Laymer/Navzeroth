import {
  KEY_PAN_STEP_PX,
  MAX_ZOOM_FACTOR,
  MIN_ZOOM_FACTOR,
  applyKeyAction,
  centerOn,
  clampViewport,
  fitViewport,
  fitBounds,
  fitZoom,
  frameToScreen,
  keyAction,
  panBy,
  screenToFrame,
  zoomAt,
  type Size,
  type MapKeyAction,
  type Viewport,
} from "./viewport";

/** keyAction returns null for unmapped keys; these tests only use mapped ones. */
function key(name: string): MapKeyAction {
  const action = keyAction(name);
  if (action === null) throw new Error(`no action for ${name}`);
  return action;
}

const frame: Size = { w: 64, h: 96 };
const container: Size = { w: 640, h: 480 };

describe("fit", () => {
  it("fits the whole frame and centres it", () => {
    const v = fitViewport(frame, container);
    expect(v.zoom).toBe(5); // 480 / 96
    expect(v.pan).toEqual({ x: (640 - 64 * 5) / 2, y: 0 });
    expect(frameToScreen(v, { x: 0, y: 0 })).toEqual(v.pan);
    expect(frameToScreen(v, { x: 64, y: 96 })).toEqual({ x: v.pan.x + 320, y: 480 });
  });
  it("never yields a zero or negative zoom for degenerate sizes", () => {
    expect(fitZoom({ w: 0, h: 10 }, container)).toBe(1);
    expect(fitZoom(frame, { w: 0, h: 0 })).toBe(1);
  });
});

describe("zoom", () => {
  it("clamps to the allowed range around the fit zoom", () => {
    const fit = fitZoom(frame, container);
    const tooFar: Viewport = { zoom: 1e9, pan: { x: 0, y: 0 } };
    expect(clampViewport(tooFar, frame, container).zoom).toBe(fit * MAX_ZOOM_FACTOR);
    expect(clampViewport({ zoom: 1e-9, pan: { x: 0, y: 0 } }, frame, container).zoom).toBe(fit * MIN_ZOOM_FACTOR);
    expect(clampViewport({ zoom: Number.NaN, pan: { x: 0, y: 0 } }, frame, container).zoom).toBe(fit);
  });
  it("keeps the anchored frame point under the pointer", () => {
    const start = fitViewport(frame, container);
    const anchor = { x: 320, y: 240 };
    const before = screenToFrame(start, anchor);
    const next = zoomAt(start, 2, anchor, frame, container);
    expect(next.zoom).toBe(start.zoom * 2);
    const after = screenToFrame(next, anchor);
    expect(after.x).toBeCloseTo(before.x, 9);
    expect(after.y).toBeCloseTo(before.y, 9);
  });
});

describe("pan", () => {
  it("cannot leave a gap once the frame is larger than the region", () => {
    const zoomed = zoomAt(fitViewport(frame, container), 4, { x: 320, y: 240 }, frame, container);
    const far = panBy(zoomed, 1e6, 1e6, frame, container);
    expect(far.pan).toEqual({ x: 0, y: 0 });
    const other = panBy(zoomed, -1e6, -1e6, frame, container);
    expect(other.pan.x).toBe(container.w - frame.w * zoomed.zoom);
    expect(other.pan.y).toBe(container.h - frame.h * zoomed.zoom);
  });
  it("keeps a smaller frame centred however it is dragged", () => {
    const fit = fitViewport(frame, container);
    expect(panBy(fit, 500, 500, frame, container)).toEqual(fit);
  });
  it("centres on a frame point where the clamps allow", () => {
    const zoomed = zoomAt(fitViewport(frame, container), 4, { x: 320, y: 240 }, frame, container);
    const centred = centerOn(zoomed, { x: 32, y: 48 }, frame, container);
    const screen = frameToScreen(centred, { x: 32, y: 48 });
    expect(screen.x).toBeCloseTo(320, 9);
    expect(screen.y).toBeCloseTo(240, 9);
    // A point near the edge cannot be centred without showing a gap.
    const edge = centerOn(zoomed, { x: 0, y: 0 }, frame, container);
    expect(edge.pan).toEqual({ x: 0, y: 0 });
  });
});

describe("coordinates", () => {
  it("round-trips screen and frame space", () => {
    const v = zoomAt(fitViewport(frame, container), 3, { x: 100, y: 100 }, frame, container);
    for (const p of [{ x: 0, y: 0 }, { x: 16, y: 96 }, { x: 63.5, y: 0.25 }]) {
      const back = screenToFrame(v, frameToScreen(v, p));
      expect(back.x).toBeCloseTo(p.x, 9);
      expect(back.y).toBeCloseTo(p.y, 9);
    }
  });
});

describe("keyboard", () => {
  it("maps the documented keys and ignores the rest", () => {
    expect(keyAction("ArrowLeft")).toEqual({ type: "pan", dx: KEY_PAN_STEP_PX, dy: 0 });
    expect(keyAction("ArrowDown")).toEqual({ type: "pan", dx: 0, dy: -KEY_PAN_STEP_PX });
    expect(keyAction("+")).toMatchObject({ type: "zoom" });
    expect(keyAction("-")).toMatchObject({ type: "zoom" });
    expect(keyAction("0")).toEqual({ type: "fit" });
    expect(keyAction("c")).toEqual({ type: "centre" });
    expect(keyAction("x")).toBeNull();
    expect(keyAction("Enter")).toBeNull();
  });
  it("steps pan and zoom, fits, and centres only when a marker is placed", () => {
    const fit = fitViewport(frame, container);
        expect(applyKeyAction(fit, key("+"), frame, container, null).zoom).toBeGreaterThan(fit.zoom);
    // Zoom far enough that the frame overflows the region, so panning has room.
    const zoomed = zoomAt(fit, 4, { x: 320, y: 240 }, frame, container);
    const panned = applyKeyAction(zoomed, key("ArrowRight"), frame, container, null);
    expect(panned.pan.x).toBeLessThan(zoomed.pan.x);
    expect(applyKeyAction(panned, key("0"), frame, container, null)).toEqual(fit);
    expect(applyKeyAction(zoomed, key("c"), frame, container, null)).toEqual(zoomed);
    const centred = applyKeyAction(zoomed, key("c"), frame, container, { x: 32, y: 48 });
    expect(frameToScreen(centred, { x: 32, y: 48 }).x).toBeCloseTo(320, 9);
  });
});

it("fits endpoint boxes including zero/coincident coordinates within zoom/pan limits", () => {
  for (const box of [{minX:0,minY:0,maxX:64,maxY:96},{minX:0,minY:0,maxX:0,maxY:0},{minX:16,minY:24,maxX:32,maxY:48}]) {
    const v = fitBounds(box, frame, container);
    expect(v.zoom).toBeLessThanOrEqual(fitZoom(frame,container)*MAX_ZOOM_FACTOR);
    for (const p of [{x:box.minX,y:box.minY},{x:box.maxX,y:box.maxY}]) {
      const screen = frameToScreen(v,p);
      expect(screen.x).toBeGreaterThanOrEqual(0); expect(screen.x).toBeLessThanOrEqual(container.w);
      expect(screen.y).toBeGreaterThanOrEqual(0); expect(screen.y).toBeLessThanOrEqual(container.h);
    }
  }
  expect(fitBounds({minX:0,minY:0,maxX:Infinity,maxY:0},frame,container)).toEqual(fitViewport(frame,container));
});
