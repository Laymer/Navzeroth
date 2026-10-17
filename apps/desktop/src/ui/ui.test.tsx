import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Backdrop } from "../theme/Backdrop";
import { Button } from "./Button";
import { Field } from "./Field";
import { HelpTip } from "./HelpTip";
import { Panel } from "./Panel";
import { StatusBadge } from "./StatusBadge";

describe("Button", () => {
  it("defaults to type=button and keeps its accessible name", () => {
    render(<Button>Save</Button>);
    const button = screen.getByRole("button", { name: "Save" });
    expect(button).toHaveAttribute("type", "button");
    expect(button).toHaveClass("ui-button--secondary");
  });

  it.each(["primary", "secondary", "ghost", "danger"] as const)("renders the %s variant and fires clicks", (variant) => {
    const onClick = vi.fn();
    render(<Button variant={variant} onClick={onClick}>Go</Button>);
    fireEvent.click(screen.getByRole("button", { name: "Go" }));
    expect(onClick).toHaveBeenCalledOnce();
    expect(screen.getByRole("button")).toHaveClass(`ui-button--${variant}`);
  });

  it("does not click while disabled", () => {
    const onClick = vi.fn();
    render(<Button disabled onClick={onClick}>Off</Button>);
    fireEvent.click(screen.getByRole("button", { name: "Off" }));
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe("Panel", () => {
  it("is a region named by its heading at the requested level", () => {
    render(<Panel title="Route" level={3}>body</Panel>);
    expect(screen.getByRole("region", { name: "Route" })).toHaveTextContent("body");
    expect(screen.getByRole("heading", { level: 3, name: "Route" })).toBeInTheDocument();
  });
});

describe("HelpTip", () => {
  it("is a button named exactly Help and hides the bubble while closed", () => {
    render(<HelpTip>Explains the field.</HelpTip>);
    const trigger = screen.getByRole("button", { name: "Help" });
    expect(trigger).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByText("Explains the field.")).not.toBeVisible();
  });

  it("opens on click, links the bubble with aria-describedby, and never uses a status role", () => {
    render(<HelpTip>Explains the field.</HelpTip>);
    const trigger = screen.getByRole("button", { name: "Help" });
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute("aria-expanded", "true");
    const bubble = screen.getByText("Explains the field.");
    expect(trigger.getAttribute("aria-describedby")).toBe(bubble.id);
    expect(trigger.getAttribute("aria-controls")).toBe(bubble.id);
    expect(screen.queryByRole("status")).toBeNull();
    fireEvent.click(trigger);
    expect(screen.getByText("Explains the field.")).not.toBeVisible();
  });

  it("opens on keyboard focus and hover, and Escape closes it", () => {
    render(<HelpTip>Hint text.</HelpTip>);
    const trigger = screen.getByRole("button", { name: "Help" });
    fireEvent.focus(trigger);
    expect(screen.getByText("Hint text.")).toBeVisible();
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.getByText("Hint text.")).not.toBeVisible();
    fireEvent.blur(trigger);
    fireEvent.mouseEnter(trigger.parentElement as HTMLElement);
    expect(screen.getByText("Hint text.")).toBeVisible();
    fireEvent.mouseLeave(trigger.parentElement as HTMLElement);
    expect(screen.getByText("Hint text.")).not.toBeVisible();
  });

  it("closes on Escape pressed outside the trigger after hover-only opening", () => {
    render(<HelpTip>Hover only.</HelpTip>);
    const trigger = screen.getByRole("button", { name: "Help" });
    fireEvent.mouseEnter(trigger.parentElement as HTMLElement);
    expect(screen.getByText("Hover only.")).toBeVisible();
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.getByText("Hover only.")).not.toBeVisible();
  });

  it("stays reachable and operable inside a disabled fieldset", () => {
    render(<fieldset disabled><HelpTip>Still readable.</HelpTip></fieldset>);
    const trigger = screen.getByRole("button", { name: "Help" });
    expect(trigger).toHaveAttribute("tabindex", "0");
    fireEvent.keyDown(trigger, { key: "Enter" });
    expect(screen.getByText("Still readable.")).toBeVisible();
    fireEvent.keyDown(trigger, { key: " " });
    expect(screen.getByText("Still readable.")).not.toBeVisible();
  });

  it("closes a pinned bubble on an outside pointer press", () => {
    render(<><HelpTip>Pinned.</HelpTip><p>elsewhere</p></>);
    fireEvent.click(screen.getByRole("button", { name: "Help" }));
    expect(screen.getByText("Pinned.")).toBeVisible();
    fireEvent.pointerDown(screen.getByText("elsewhere"));
    expect(screen.getByText("Pinned.")).not.toBeVisible();
  });
});

describe("Field", () => {
  it("names the control by its label alone and keeps Help out of the label", () => {
    render(
      <Field label="Budget" hint="Minutes." help="Planner time budget." >
        {(control) => <input {...control} />}
      </Field>,
    );
    const input = screen.getByRole("textbox", { name: "Budget" });
    expect(input).toHaveAccessibleDescription("Minutes.");
    const label = screen.getByText("Budget", { selector: "label" });
    expect(label).not.toContainElement(screen.getByRole("button", { name: "Help: Budget" }));
  });

  it("marks the control invalid and describes the error", () => {
    render(<Field label="Path" error="Too long.">{(control) => <input {...control} />}</Field>);
    const input = screen.getByRole("textbox", { name: "Path" });
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAccessibleDescription("Too long.");
  });

  it("omits Help when no help text is declared", () => {
    render(<Field label="Name">{(control) => <input {...control} />}</Field>);
    expect(screen.queryByRole("button", { name: /^Help/ })).toBeNull();
  });
});

describe("StatusBadge", () => {
  it("carries meaning in text with a decorative shape", () => {
    render(<StatusBadge tone="warn">Stale</StatusBadge>);
    const badge = screen.getByText("Stale");
    expect(badge).toHaveClass("ui-badge--warn");
    expect(badge.querySelector("[aria-hidden=true]")).not.toBeNull();
    expect(badge).toHaveTextContent("▲Stale");
  });

  it("uses a different shape for every tone", () => {
    const shapes = (["ok", "warn", "danger", "info", "neutral"] as const).map((tone) => {
      const { container, unmount } = render(<StatusBadge tone={tone}>x</StatusBadge>);
      const shape = container.querySelector(".ui-badge__shape")?.textContent;
      unmount();
      return shape;
    });
    expect(new Set(shapes).size).toBe(5);
  });
});

describe("Backdrop", () => {
  it("is decorative: hidden from assistive technology with no interactive content", () => {
    const { container } = render(<Backdrop />);
    expect(container.firstElementChild).toHaveAttribute("aria-hidden", "true");
    expect(container.querySelector("button, a, input, img, image, [tabindex]")).toBeNull();
    expect(container.querySelectorAll("circle.backdrop-ember").length).toBeGreaterThan(0);
  });
});
