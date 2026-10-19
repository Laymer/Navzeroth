import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { OriginMarker, ORIGIN_LABEL } from "./index";

describe("OriginMarker", () => {
  it.each([[[]], [null], [undefined]] as const)("shows nothing for Forever-origin or unresolved facts (%s)", (marks) => {
    const { container } = render(<p>Fact<OriginMarker marks={marks} /></p>);
    expect(container.querySelector(".ui-origin")).toBeNull();
    expect(container.textContent).toBe("Fact");
  });

  it("labels each accepted origin in text, in a fixed order, with a decorative shape", () => {
    const { container } = render(<OriginMarker marks={["inferred", "wow_classic_era", "vanilla_1_12"]} />);
    expect(container.querySelector(".ui-origin")?.textContent?.replace(/[◆▲]/g, "")).toBe("From 1.12 dataFrom Classic Era dataInferred");
    expect(container.querySelector(".ui-origin")).toHaveAttribute("data-origin", "vanilla_1_12 wow_classic_era inferred");
    expect(screen.getByText("Inferred")).toHaveClass("ui-badge--warn");
    expect(screen.getByText("From 1.12 data")).toHaveClass("ui-badge--info");
    for (const shape of container.querySelectorAll(".ui-badge__shape")) expect(shape).toHaveAttribute("aria-hidden", "true");
    expect(ORIGIN_LABEL.wow_classic_era).toBe("From Classic Era data");
  });
});
