import { act, fireEvent, render, screen } from "@testing-library/react";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { createRef } from "react";
import { createPortal } from "react-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Button } from "./Button";
import { Field } from "./Field";
import { HelpTip, clampShift } from "./HelpTip";
import { Panel, PanelBoundary } from "./Panel";

describe("HelpTip label", () => {
  it("is named exactly Help without a label", () => {
    render(<HelpTip>Text.</HelpTip>);
    expect(screen.getByRole("button", { name: "Help" })).toBeInTheDocument();
  });

  it("is named Help: <label> with a label, so many tips are distinguishable", () => {
    render(<><HelpTip label="Budget (ms)">One.</HelpTip><HelpTip label="Origin">Two.</HelpTip></>);
    expect(screen.getByRole("button", { name: "Help: Budget (ms)" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Help: Origin" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Help" })).toBeNull();
  });

  it("is passed the field label by Field", () => {
    render(<Field label="Budget (ms)" help="Explains.">{(control) => <input {...control} />}</Field>);
    expect(screen.getByRole("button", { name: "Help: Budget (ms)" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Budget (ms)" })).toBeInTheDocument();
  });
});

describe("Button ref and aria-pressed", () => {
  it("forwards a ref to the native button", () => {
    const ref = createRef<HTMLButtonElement>();
    render(<Button ref={ref}>Focus me</Button>);
    expect(ref.current).toBe(screen.getByRole("button", { name: "Focus me" }));
    ref.current?.focus();
    expect(ref.current).toHaveFocus();
  });

  it("supports callback refs and reports aria-pressed", () => {
    const nodes: (HTMLButtonElement | null)[] = [];
    render(<Button ref={(node) => { nodes.push(node); }} aria-pressed={true}>Hop</Button>);
    expect(nodes[0]).toBe(screen.getByRole("button", { name: "Hop" }));
    expect(screen.getByRole("button", { name: "Hop" })).toHaveAttribute("aria-pressed", "true");
  });
});

describe("nested Panel", () => {
  it("marks only a Panel inside another Panel as nested", () => {
    render(
      <Panel title="Outer">
        <Panel title="Inner" level={3}>x</Panel>
      </Panel>,
    );
    expect(screen.getByRole("region", { name: "Outer" })).not.toHaveClass("ui-panel--nested");
    expect(screen.getByRole("region", { name: "Inner" })).toHaveClass("ui-panel--nested");
    expect(screen.getByRole("heading", { level: 3, name: "Inner" })).toBeInTheDocument();
  });

  it("does not mark a sibling Panel as nested", () => {
    render(<><Panel title="A">a</Panel><Panel title="B">b</Panel></>);
    expect(screen.getByRole("region", { name: "B" })).not.toHaveClass("ui-panel--nested");
  });

  // G6 O2: context crosses portals, so portal content needs a PanelBoundary.
  it("treats a Panel in a portal as nested unless a PanelBoundary resets it", () => {
    const host = document.createElement("div");
    document.body.append(host);
    render(
      <Panel title="Opener">
        {createPortal(<Panel title="Portal without boundary">a</Panel>, host)}
        {createPortal(<PanelBoundary><Panel title="Dialog panel">b</Panel></PanelBoundary>, host)}
      </Panel>,
    );
    expect(screen.getByRole("region", { name: "Portal without boundary" })).toHaveClass("ui-panel--nested");
    expect(screen.getByRole("region", { name: "Dialog panel" })).not.toHaveClass("ui-panel--nested");
    host.remove();
  });

  it("requires a PanelBoundary in every source file that opens a portal or dialog", () => {
    const files: string[] = [];
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir)) {
        const path = join(dir, entry);
        if (statSync(path).isDirectory()) walk(path);
        else if (/\.tsx$/.test(path) && !/\.test\.tsx$/.test(path)) files.push(path);
      }
    };
    walk(join(__dirname, ".."));
    expect(files.length).toBeGreaterThan(20);
    for (const file of files) {
      const source = readFileSync(file, "utf8");
      if (/createPortal\(|<dialog\b|role="dialog"|role="alertdialog"/.test(source)) expect(source, file).toMatch(/PanelBoundary/);
    }
  });
});

describe("HelpTip viewport clamp", () => {
  const original = Object.getOwnPropertyDescriptor(Element.prototype, "getBoundingClientRect");
  afterEach(() => {
    if (original) Object.defineProperty(Element.prototype, "getBoundingClientRect", original);
    vi.restoreAllMocks();
  });

  it("computes the shift for left overflow, right overflow and fit", () => {
    expect(clampShift(-20, 100, 320)).toBe(28);
    expect(clampShift(300, 100, 320)).toBe(-88);
    expect(clampShift(50, 100, 320)).toBe(0);
  });

  /** A bubble of 200 px whose natural left edge is `naturalLeft`; moves with its translateX like a real one. */
  function mockLayout(naturalLeft: number, viewport: { value: number }) {
    Object.defineProperty(document.documentElement, "clientWidth", { configurable: true, get: () => viewport.value });
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (this: Element) {
      const isBubble = this.classList.contains("ui-helptip__bubble");
      const shift = isBubble ? Number(/translateX\((-?[\d.]+)px\)/.exec((this as HTMLElement).style.transform)?.[1] ?? 0) : 0;
      const width = isBubble ? 200 : 0;
      const left = isBubble ? naturalLeft + shift : 0;
      return { x: left, y: 0, left, top: 0, width, height: isBubble ? 40 : 0, right: left + width, bottom: 40, toJSON: () => ({}) } as DOMRect;
    });
  }

  it("pulls a bubble that overflows the right edge back inside, and re-measures on resize", () => {
    const viewport = { value: 320 };
    mockLayout(200, viewport); // 200..400 in a 320 px window
    render(<HelpTip>Wide help.</HelpTip>);
    fireEvent.click(screen.getByRole("button", { name: "Help" }));
    const bubble = screen.getByText("Wide help.");
    expect(bubble.style.transform).toBe("translateX(-88px)"); // right edge at 312 = 320 - 8
    // The window grows: the bubble fits, so the stale shift must go.
    viewport.value = 800;
    act(() => { window.dispatchEvent(new Event("resize")); });
    expect(bubble.style.transform).toBe("");
    // The window shrinks again: it is clamped again.
    viewport.value = 300;
    act(() => { window.dispatchEvent(new Event("resize")); });
    expect(bubble.style.transform).toBe("translateX(-108px)");
  });

  it("pushes a bubble that overflows the left edge inside", () => {
    mockLayout(-30, { value: 320 });
    render(<HelpTip>Left help.</HelpTip>);
    fireEvent.click(screen.getByRole("button", { name: "Help" }));
    expect(screen.getByText("Left help.").style.transform).toBe("translateX(38px)");
  });

  it("stops listening for resize once closed", () => {
    const add = vi.spyOn(window, "addEventListener");
    const remove = vi.spyOn(window, "removeEventListener");
    const viewport = { value: 320 };
    mockLayout(200, viewport);
    render(<HelpTip>Closing.</HelpTip>);
    const trigger = screen.getByRole("button", { name: "Help" });
    const bubble = screen.getByText("Closing.");
    fireEvent.click(trigger);
    const added = add.mock.calls.filter(([type]) => type === "resize").map(([, listener]) => listener);
    expect(added).toHaveLength(1);
    fireEvent.click(trigger);
    expect(bubble).not.toBeVisible();
    // The very listener that was added is the one removed, not merely some resize listener.
    expect(remove.mock.calls.filter(([type]) => type === "resize").map(([, listener]) => listener)).toEqual(added);
    // And a later resize no longer measures the closed bubble.
    const rect = vi.mocked(Element.prototype.getBoundingClientRect);
    rect.mockClear();
    viewport.value = 200;
    act(() => { window.dispatchEvent(new Event("resize")); });
    expect(rect.mock.contexts.filter((el) => el === bubble)).toHaveLength(0);
    expect(bubble.style.transform).toBe("");
  });
});
