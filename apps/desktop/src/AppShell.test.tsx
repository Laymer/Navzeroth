import { render, screen, within } from "@testing-library/react";
import { App } from "./App";
import { createClient } from "./commands";
import { demoAdapter } from "./commands/demoAdapter";
import { SettingsScreen } from "./screens/SettingsScreen";
import { defaultSettings } from "./commands/today";

const PLAY = ["Today", "Atlas", "Planner", "Gear", "Party", "Dungeons", "Sessions", "Reminders"];
const SYSTEM = ["Imports", "Content", "Compatibility and coverage", "Settings", "Local diagnostics", "Backup and recovery"];

describe("app shell", () => {
  it("keeps the h1 named exactly Azeroth Navigator with decorative ornaments only", async () => {
    render(<App client={createClient(demoAdapter)} />);
    expect(await screen.findByRole("navigation", { name: "Primary" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "Azeroth Navigator" })).toBeInTheDocument();
    expect(document.querySelector(".app-ornament")).toHaveAttribute("aria-hidden", "true");
  });

  it("offers exactly 14 buttons in two labelled groups inside one Primary navigation", async () => {
    render(<App client={createClient(demoAdapter)} />);
    const nav = await screen.findByRole("navigation", { name: "Primary" });
    expect(screen.getAllByRole("navigation")).toHaveLength(1);
    expect(within(nav).getAllByRole("button")).toHaveLength(14);
    const groups = within(nav).getAllByRole("group");
    expect(groups.map((group) => group.getAttribute("aria-labelledby") && document.getElementById(group.getAttribute("aria-labelledby") ?? "")?.textContent)).toEqual(["Play", "Data & system"]);
    expect(within(groups[0] as HTMLElement).getAllByRole("button").map((b) => b.textContent)).toEqual(PLAY);
    expect(within(groups[1] as HTMLElement).getAllByRole("button").map((b) => b.textContent)).toEqual(SYSTEM);
    expect(within(nav).getByRole("button", { name: "Today" })).toHaveAttribute("aria-current", "page");
  });

  it("states non-affiliation on Settings", async () => {
    const client = createClient({
      invoke: (command: string) =>
        Promise.resolve(command === "get_app_settings" ? { revision: 1, settings: defaultSettings() } : []) as never,
    });
    render(<SettingsScreen client={client} onChanged={() => {}} />);
    expect(await screen.findByText(/not affiliated with or endorsed by Blizzard Entertainment/)).toBeVisible();
  });
});
