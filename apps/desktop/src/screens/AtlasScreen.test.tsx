import { fireEvent, render, screen, within } from "@testing-library/react";
import { App } from "../App";
import { createClient, type CommandAdapter } from "../commands";
import { demoAdapter } from "../commands/demoAdapter";
import type { AppError, AtlasView, CommandMap, CommandName, SnapshotMarker } from "../commands/contract";
import fresh from "../../../../fixtures/desktop/atlas/snapshot-markers-fresh.json";
import stale from "../../../../fixtures/desktop/atlas/snapshot-markers-stale.json";
import restricted from "../../../../fixtures/desktop/atlas/snapshot-markers-location-restricted.json";
import otherFloor from "../../../../fixtures/desktop/atlas/snapshot-markers-gap-different-floor.json";
import { AtlasScreen } from "./AtlasScreen";

type Overrides = Partial<{ [C in CommandName]: () => CommandMap[C]["output"] | Promise<CommandMap[C]["output"]> }>;

/** The demo adapter with selected commands replaced by fixed answers. */
function adapterWith(overrides: Overrides): CommandAdapter {
  return {
    invoke: (command, input) => {
      const override = overrides[command];
      if (override !== undefined) return Promise.resolve(override()) as never;
      return demoAdapter.invoke(command, input);
    },
  };
}
const renderAtlas = (adapter: CommandAdapter) => render(<AtlasScreen client={createClient(adapter)} />);
const markers = (golden: unknown) => () => golden as SnapshotMarker[];
const noLive = () => expect(document.body.textContent).not.toMatch(/\blive\b/i);

describe("Atlas screen over the demo atlas", () => {
  it("loads from bundled demo data with no network, and keeps the DEMO label", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    renderAtlas(demoAdapter);
    expect(screen.getByText("Loading the atlas…")).toBeInTheDocument();
    expect(await screen.findByText(/DEMO — synthetic data/)).toBeInTheDocument();
    expect(screen.getByText(/Dataset demo-imagery-r1 \(demo namespace, fixture\)/)).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText(/Flight network/));
    expect(screen.getByText(/DEMO — synthetic data/)).toBeInTheDocument();
    expect(fetchSpy).not.toHaveBeenCalled();
    fetchSpy.mockRestore();
  });

  it("labels the snapshot as last imported, never live, with approximate calibration shown", async () => {
    renderAtlas(demoAdapter);
    const badge = await screen.findByText(/^Last imported:/);
    expect(badge).toHaveTextContent("Last imported: age uncertain");
    expect(screen.getByText("Last imported position, age uncertain: approximate, uncalibrated")).toBeInTheDocument();
    expect(screen.getAllByText("approximate, uncalibrated").length).toBeGreaterThan(0);
    expect(screen.getByTestId("uncertainty-ring")).toBeInTheDocument();
    noLive();
  });

  it("states the age of a fresh and a stale import and flags staleness in words", async () => {
    const view = renderAtlas(adapterWith({ get_snapshot_markers: markers(fresh) }));
    expect(await screen.findByText("Last imported: captured 1 min ago")).toBeInTheDocument();
    expect(screen.queryByText(/^Stale:/)).not.toBeInTheDocument();
    view.unmount();

    renderAtlas(adapterWith({ get_snapshot_markers: markers(stale) }));
    expect(await screen.findByText("Last imported: captured 31 min ago (stale)")).toBeInTheDocument();
    expect(screen.getByText(/^Stale: this position is older than 30 minutes/)).toBeInTheDocument();
    noLive();
  });

  it("labels reference pins and layers as reference knowledge, not unlocks", async () => {
    renderAtlas(demoAdapter);
    const pins = await screen.findByRole("list");
    expect(within(pins).getByText(/Reference flight master\. Not your unlocks/)).toBeInTheDocument();
    expect(screen.getByLabelText("Roads (traced, unverified)")).toBeChecked();
    expect(screen.getByLabelText("Flight network (reference, not your unlocks)")).not.toBeChecked();
  });

  it("switches layers with the toggles", async () => {
    renderAtlas(demoAdapter);
    expect(await screen.findByText(/Showing layer: roads/)).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText(/Flight network/));
    expect(screen.getByText(/Showing layer: flight/)).toBeInTheDocument();
    fireEvent.click(screen.getByLabelText(/Flight network/));
    fireEvent.click(screen.getByLabelText(/Roads/));
    expect(screen.getByText(/Showing layer: overview/)).toBeInTheDocument();
  });

  it("explains a gap in words and draws no marker for it", async () => {
    const { container } = renderAtlas(adapterWith({ get_snapshot_markers: markers(otherFloor) }));
    expect(await screen.findByText(/^Not drawn: the position is on floor 1 of map demo-map-1/)).toBeInTheDocument();
    expect(screen.getByText("Not drawn: position is on another floor")).toBeInTheDocument();
    expect(container.querySelector("g.marker")).toBeNull();
  });

  it("never draws a guessed marker for a restricted location", async () => {
    const { container } = renderAtlas(adapterWith({ get_snapshot_markers: markers(restricted) }));
    expect(await screen.findByText("Location restricted by the game")).toBeInTheDocument();
    expect(container.querySelector("g.marker")).toBeNull();
    expect(screen.getByText("Last imported: captured 1 min ago")).toBeInTheDocument();
  });
});

describe("schematic fallback", () => {
  const withoutUrls = (view: AtlasView): AtlasView => ({
    ...view,
    images: view.images.map((i) => ({ ...i, layers: i.layers.map((l) => ({ ...l, url: null })) })),
  });
  it("renders when no layer URL exists, with pins and marker still placed", async () => {
    const base = await demoAdapter.invoke("query_atlas", { image: null });
    const { container } = renderAtlas(adapterWith({ query_atlas: () => withoutUrls(base) }));
    expect(await screen.findByTestId("schematic")).toBeInTheDocument();
    expect(screen.getByText(/Schematic view: no imagery could be shown/)).toBeInTheDocument();
    expect(screen.getByText(/Showing: schematic frame \(no imagery\)\./)).toBeInTheDocument();
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("g.marker")).not.toBeNull();
    expect(container.querySelectorAll("g.pin").length).toBe(1);
  });

  it("renders when the images fail to load", async () => {
    const { container } = renderAtlas(demoAdapter);
    await screen.findByText(/Showing layer: roads/);
    expect(screen.queryByTestId("schematic")).not.toBeInTheDocument();
    // roads fails, then the overview base layer fails, leaving nothing to draw
    for (let i = 0; i < 2; i += 1) {
      const img = container.querySelector("img");
      if (img !== null) fireEvent.error(img);
    }
    expect(await screen.findByTestId("schematic")).toBeInTheDocument();
    expect(screen.getAllByText(/the image failed to load/).length).toBe(2);
  });

  it("shows why a refused layer is not drawn", async () => {
    const base = await demoAdapter.invoke("query_atlas", { image: null });
    const refused: AtlasView = {
      ...base,
      images: base.images.map((i) => ({
        ...i,
        layers: i.layers.map((l) => ({ ...l, url: null, availability: "integrity_mismatch" as const })),
      })),
    };
    renderAtlas(adapterWith({ query_atlas: () => refused }));
    expect((await screen.findAllByText(/refused, the file does not match its declared digest/)).length).toBe(3);
    expect(screen.getByTestId("schematic")).toBeInTheDocument();
  });
});

describe("loading, empty, error and stale states are distinct", () => {
  it("loading", () => {
    renderAtlas({ invoke: () => new Promise(() => undefined) });
    expect(screen.getByRole("status")).toHaveTextContent("Loading the atlas…");
  });

  it("error shows the code and the safe message", async () => {
    const error: AppError = { code: "DATASET_INCOMPLETE", message: "Atlas dataset is incomplete", retryable: false };
    renderAtlas({ invoke: () => Promise.reject(error) });
    expect(await screen.findByRole("alert")).toHaveTextContent("Atlas error DATASET_INCOMPLETE: Atlas dataset is incomplete");
  });

  it("empty: no dataset", async () => {
    const base = await demoAdapter.invoke("query_atlas", { image: null });
    renderAtlas(adapterWith({ query_atlas: () => ({ ...base, images: [] }) }));
    expect(await screen.findByText("No atlas dataset is installed, so there is no map to show.")).toBeInTheDocument();
    expect(screen.getByText(/DEMO — synthetic data/)).toBeInTheDocument();
  });

  it("empty: no snapshot, but the map is still usable", async () => {
    renderAtlas(adapterWith({ get_snapshot_markers: () => [] }));
    expect(await screen.findByText("No snapshot imported")).toBeInTheDocument();
    expect(screen.getByText("No imported snapshot: no position to show")).toBeInTheDocument();
    expect(screen.getByRole("group", { name: /Atlas map of demo-image-1/ })).toBeInTheDocument();
  });

  it("the state texts differ from one another", async () => {
    const texts = new Set<string>();
    const a = renderAtlas({ invoke: () => new Promise(() => undefined) });
    texts.add(screen.getByRole("status").textContent ?? "");
    a.unmount();
    const b = renderAtlas(adapterWith({ get_snapshot_markers: markers(stale) }));
    texts.add((await screen.findByText(/^Stale:/)).textContent ?? "");
    b.unmount();
    const c = renderAtlas(adapterWith({ get_snapshot_markers: () => [] }));
    texts.add((await screen.findByText("No snapshot imported")).textContent ?? "");
    c.unmount();
    expect(texts.size).toBe(3);
  });
});

describe("keyboard", () => {
  it("handles map keys only when the map region has focus", async () => {
    renderAtlas(demoAdapter);
    const region = await screen.findByRole("group", { name: /Atlas map of demo-image-1/ });
    expect(region).toHaveAttribute("tabindex", "0");
    expect(region).toHaveAccessibleDescription(/arrow keys pan/);
    const layer = region.querySelector<HTMLElement>(".map-layer");
    const before = layer?.style.transform;
    fireEvent.keyDown(region, { key: "+" });
    expect(layer?.style.transform).not.toBe(before);
    fireEvent.keyDown(region, { key: "0" });
    expect(layer?.style.transform).toBe(before);
  });
});

describe("navigation", () => {
  it("switches between Today and Atlas and keeps the DEMO label on both", async () => {
    render(<App client={createClient(demoAdapter)} />);
    expect(await screen.findByText("demo-snapshot-001")).toBeInTheDocument();
    const nav = screen.getByRole("navigation", { name: "Primary" });
    expect(within(nav).getByRole("button", { name: "Today" })).toHaveAttribute("aria-current", "page");
    fireEvent.click(within(nav).getByRole("button", { name: "Atlas" }));
    expect(await screen.findByRole("heading", { name: "Atlas" })).toBeInTheDocument();
    expect(await screen.findByText(/DEMO — synthetic data/)).toBeInTheDocument();
    expect(within(nav).getByRole("button", { name: "Atlas" })).toHaveAttribute("aria-current", "page");
    fireEvent.click(within(nav).getByRole("button", { name: "Today" }));
    expect(await screen.findByText("demo-snapshot-001")).toBeInTheDocument();
  });
});

describe("dense reference search", () => {
  it("filters pins on the map and list, navigates by keyboard, and preserves selection on refresh", async () => {
    const original = await demoAdapter.invoke("query_atlas", { image: null });
    const image = original.images[0];
    if (!image) throw new Error("Missing demo image");
    const template = image.pins[0];
    if (!template) throw new Error("Missing demo pin");
    const dense: AtlasView = { ...original, images: [{ ...image, pins: Array.from({ length: 200 }, (_, i) => ({ ...template, kind: i % 2 === 0 ? template.kind : { kind: "landmark" }, id: `dense-${i}`, label: `Reference ${String(i).padStart(3, "0")}`, pixel: { x: i % image.width_px, y: i % image.height_px } })) }] };
    let queries = 0;
    renderAtlas(adapterWith({ query_atlas: () => ({ ...dense, revision: ++queries === 1 ? "dense-original" : "dense-refreshed" }) }));
    const last = await screen.findByRole("button", { name: "Show Reference 199 on the map" });
    fireEvent.change(screen.getByLabelText("Point type"), { target: { value: "landmark" } });
    expect(screen.getByText("100 of 200 reference pins shown.")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Point type"), { target: { value: "all" } });
    fireEvent.click(last);
    expect(last).toHaveAttribute("aria-pressed", "true");
    last.focus();
    fireEvent.keyDown(last, { key: "Home" });
    expect(screen.getByRole("button", { name: "Show Reference 000 on the map" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("button", { name: "Show Reference 000 on the map" }), { key: "ArrowUp" });
    expect(last).toHaveFocus();
    fireEvent.keyDown(last, { key: "ArrowDown" });
    expect(screen.getByRole("button", { name: "Show Reference 000 on the map" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("button", { name: "Show Reference 000 on the map" }), { key: "End" });
    expect(last).toHaveFocus();
    fireEvent.change(screen.getByLabelText("Search reference pins"), { target: { value: "Reference 199" } });
    expect(screen.getAllByRole("button", { name: /^Show Reference/ })).toHaveLength(1);
    expect(document.querySelectorAll(".pin")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Refresh atlas" }));
    expect(await screen.findByText(/Dataset dense-refreshed/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Show Reference 199 on the map" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByLabelText("Search reference pins")).toHaveValue("Reference 199");
    fireEvent.change(screen.getByLabelText("Search reference pins"), { target: { value: "missing" } });
    expect(screen.getByText("No reference pins match these filters.")).toBeInTheDocument();
    expect(screen.getByText("The selected pin is hidden by the current filters.")).toBeInTheDocument();
  });
});


it("keeps pin, search, point type and layers through a failed refresh and successful retry", async () => {
  const original = await demoAdapter.invoke("query_atlas", { image: null });
  let queries = 0;
  renderAtlas(adapterWith({ query_atlas: () => {
    queries += 1;
    if (queries === 2) return Promise.reject({ code: "PROVIDER_UNAVAILABLE", message: "Temporary atlas read failure", retryable: true });
    return { ...original, revision: queries === 1 ? "before-error" : "after-retry" };
  } }));
  const image = original.images[0];
  const pin = image?.pins[0];
  if (!pin) throw new Error("Missing demo pin");
  const button = await screen.findByRole("button", { name: `Show ${pin.label} on the map` });
  fireEvent.click(button);
  fireEvent.change(screen.getByLabelText("Search reference pins"), { target: { value: pin.id } });
  fireEvent.change(screen.getByLabelText("Point type"), { target: { value: pin.kind.kind } });
  fireEvent.click(screen.getByLabelText(/Flight network/));
  fireEvent.click(screen.getByRole("button", { name: "Refresh atlas" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("The previous atlas remains displayed");
  expect(screen.getByText(/Dataset before-error/)).toBeInTheDocument();
  expect(button).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByLabelText("Search reference pins")).toHaveValue(pin.id);
  expect(screen.getByLabelText("Point type")).toHaveValue(pin.kind.kind);
  expect(screen.getByLabelText(/Flight network/)).toBeChecked();
  fireEvent.click(screen.getByRole("button", { name: "Refresh atlas" }));
  expect(await screen.findByText(/Dataset after-retry/)).toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: `Show ${pin.label} on the map` })).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByLabelText("Search reference pins")).toHaveValue(pin.id);
  expect(screen.getByLabelText("Point type")).toHaveValue(pin.kind.kind);
  expect(screen.getByLabelText(/Flight network/)).toBeChecked();
});

it("distinguishes a removed selected pin from a filter-hidden selection", async () => {
  const original = await demoAdapter.invoke("query_atlas", { image: null });
  let queries = 0;
  renderAtlas(adapterWith({ query_atlas: () => ++queries === 1 ? original : {
    ...original, revision: "pin-removed", images: original.images.map((image) => ({ ...image, pins: [] })),
  } }));
  const pin = original.images[0]?.pins[0];
  if (!pin) throw new Error("Missing demo pin");
  fireEvent.click(await screen.findByRole("button", { name: `Show ${pin.label} on the map` }));
  fireEvent.change(screen.getByLabelText("Point type"), { target: { value: pin.kind.kind } });
  fireEvent.click(screen.getByRole("button", { name: "Refresh atlas" }));
  expect(await screen.findByText(/Dataset pin-removed/)).toBeInTheDocument();
  expect(screen.getByLabelText("Point type")).toHaveValue(pin.kind.kind);
  expect(screen.getByRole("option", { name: /absent in this revision/ })).toBeInTheDocument();
  expect(screen.getByText("The selected pin is no longer present in this atlas revision.")).toBeInTheDocument();
  expect(screen.queryByText("The selected pin is hidden by the current filters.")).not.toBeInTheDocument();
});


it("renders schema-valid UTC limits without throwing and preserves a zero timestamp", async () => {
  const marker = (fresh as unknown as SnapshotMarker[])[0];
  if (!marker) throw new Error("Missing synthetic marker");
  for (const timestamp of [Number.MAX_SAFE_INTEGER, Number.MIN_SAFE_INTEGER, 0]) {
    const instance = renderAtlas(adapterWith({ get_snapshot_markers: () => [{ ...marker, observed_at_utc_ms: timestamp }] }));
    const captured = await screen.findByText(/^Captured \(UTC\):/);
    expect(captured).toHaveTextContent(timestamp === 0
      ? "1970-01-01T00:00:00.000Z"
      : `${timestamp} ms since epoch (outside date display range)`);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    instance.unmount();
  }
});

it("bounds dense map/list pages and keeps global keyboard focus across pages", async () => {
  const original = await demoAdapter.invoke("query_atlas", { image: null });
  const image = original.images[0];
  const template = image?.pins[0];
  if (!image || !template) throw new Error("Missing demo image/pin");
  let removed = false;
  const pins = Array.from({ length: 450 }, (_, i) => ({ ...template, id: `dense-${i}`, label: `Dense ${i}` }));
  renderAtlas(adapterWith({ query_atlas: () => ({ ...original, revision: removed ? "trimmed" : "full", images: [{ ...image, pins: removed ? pins.slice(0, 20) : pins }] }) }));
  const first = await screen.findByRole("button", { name: "Show Dense 0 on the map" });
  expect(document.querySelectorAll(".pins button")).toHaveLength(200);
  expect(document.querySelectorAll("g.pin")).toHaveLength(200);
  first.focus(); fireEvent.keyDown(first, { key: "End" });
  const last = screen.getByRole("button", { name: "Show Dense 449 on the map" });
  expect(last).toHaveFocus();
  expect(document.querySelectorAll(".pins button")).toHaveLength(50);
  expect(screen.getByText("Page 3 of 3")).toBeInTheDocument();
  fireEvent.click(last);
  fireEvent.keyDown(last, { key: "ArrowDown" });
  expect(screen.getByRole("button", { name: "Show Dense 0 on the map" })).toHaveFocus();
  expect(screen.getByText(/The selected pin is on another page/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Next pin page" }));
  expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Search reference pins"), { target: { value: "Dense 449" } });
  expect(screen.getByRole("button", { name: "Show Dense 449 on the map" })).toHaveAttribute("aria-pressed", "true");
  expect(screen.queryByRole("group", { name: "Reference pin pages" })).not.toBeInTheDocument();
  removed = true;
  fireEvent.click(screen.getByRole("button", { name: "Refresh atlas" }));
  await screen.findByText(/Dataset trimmed/);
  expect(screen.getByText("The selected pin is no longer present in this atlas revision.")).toBeInTheDocument();
});
