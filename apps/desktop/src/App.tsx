import {TodayScreen} from "./screens/TodayScreen";
import {SettingsScreen} from "./screens/SettingsScreen";
import "./screens/settings.css";
import { ReferenceWindowControls } from "./screens/ReferenceWindowControls";
import { CompatibilityPanel } from "./screens/CompatibilityPanel";
import { DiagnosticsPanel } from "./screens/DiagnosticsPanel";
import { RecoveryPanel } from "./screens/RecoveryPanel";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import type { AppError, AppStatus, CommandClient } from "./commands";
import type { CharacterSnapshotEnvelope } from "../../../schemas/v1/exchange";
import { describeObservation } from "./observation";
import { ImportScreen } from "./screens/ImportScreen";
import { describeAge } from "./atlas/describe";
import type { Freshness } from "./commands/contract";
import { AtlasScreen } from "./screens/AtlasScreen";
import { PartyScreen } from "./screens/PartyScreen";
import { PlannerScreen } from "./screens/PlannerScreen";
import { GearScreen } from "./screens/GearScreen";
import { SessionsScreen } from "./screens/SessionsScreen";
import { ReminderScreen } from "./screens/ReminderScreen";
import { ContentScreen } from "./screens/ContentScreen";
import { DungeonsScreen } from "./screens/DungeonsScreen";
import { Button } from "./ui";

type Screen = "compatibility" | "today" | "atlas" | "imports" | "party" | "content" | "planner" | "gear" | "sessions" | "reminders" | "recovery" | "diagnostics" | "dungeons" | "settings";
const SCREENS: { id: Screen; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "atlas", label: "Atlas" },
  { id: "planner", label: "Planner" },
  { id: "gear", label: "Gear" },
  { id: "party", label: "Party" },
  { id: "dungeons", label: "Dungeons" },
  { id: "sessions", label: "Sessions" },
  { id: "reminders", label: "Reminders" },
  { id: "imports", label: "Imports" },
  { id: "content", label: "Content" },
  { id: "compatibility", label: "Compatibility and coverage" },
  { id: "settings", label: "Settings" },
  { id: "diagnostics", label: "Local diagnostics" },
  { id: "recovery", label: "Backup and recovery" },
];
// Two labelled groups, following plan section 12.1. Together they hold all 14 screens.
const NAV_GROUPS: { title: string; ids: Screen[] }[] = [
  { title: "Play", ids: ["today", "atlas", "planner", "gear", "party", "dungeons", "sessions", "reminders"] },
  { title: "Data & system", ids: ["imports", "content", "compatibility", "settings", "diagnostics", "recovery"] },
];

type Load =
  | { phase: "loading" }
  | { phase: "error"; error: AppError }
  | { phase: "ready"; status: AppStatus; snapshot: CharacterSnapshotEnvelope | null; freshness: Freshness | null };

/** A DOM-safe slug of a group title: "Data & system" becomes "data-system". */
export function navSlug(title: string): string {
  return title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
}

export function App({ client }: { client: CommandClient }) {
  const [load, setLoad] = useState<Load>({ phase: "loading" });
  const [screen, setScreen] = useState<Screen>("today");
  const navIds = useId();

  const [revision, setRevision] = useState(0);
  const [savedTopmost,setSavedTopmost]=useState(false);
  const watchedImport = useRef("");
  useEffect(()=>{let live=true;void client.getAppSettings().then(result=>{if(!live||!result.ok)return;setSavedTopmost(result.value.settings.reference_always_on_top);const a=result.value.settings.accessibility;document.documentElement.style.fontSize=`${a.text_scale_percent}%`;document.documentElement.dataset.contrast=a.contrast;document.documentElement.dataset.motion=a.motion;});return()=>{live=false;};},[client,revision]);
  const changed = useCallback(() => setRevision((value) => value + 1), []);
  const restored = useCallback(() => { setScreen("recovery"); changed(); }, [changed]);
  useEffect(() => {
    // Requery backend freshness as imported observations age; no local age math.
    const timer = window.setInterval(changed, 60_000);
    return () => window.clearInterval(timer);
  }, [changed]);
  useEffect(() => {
    let live = true;
    void (async () => {
      const status = await client.getAppStatus();
      if (!status.ok) return live && setLoad({ phase: "error", error: status.error });
      let snapshot: CharacterSnapshotEnvelope | null = null;
      let freshness: Freshness | null = null;
      if (status.value.data_mode === "stored" && status.value.active_profile !== null) {
        const stored = await client.getStoredSnapshot(status.value.active_profile);
        if (!stored.ok) return live && setLoad({ phase: "error", error: stored.error });
        snapshot = stored.value?.envelope ?? null;
        freshness = stored.value?.freshness ?? null;
      } else if (status.value.active_profile !== null) {
        const snap = await client.getCharacterSnapshot(status.value.active_profile);
        if (!snap.ok) return live && setLoad({ phase: "error", error: snap.error });
        snapshot = snap.value;
      }
      if (live) setLoad({ phase: "ready", status: status.value, snapshot, freshness });
    })();
    return () => {
      live = false;
    };
  }, [client, revision]);

  useEffect(() => {
    let live = true;
    const timer = window.setInterval(() => {
      void client.getWatch().then((result) => {
        if (!live || !result.ok || result.value.last_import === null) return;
        const key = JSON.stringify(result.value.last_import);
        if (key !== watchedImport.current) { watchedImport.current = key; changed(); }
      });
    }, 1000);
    return () => { live = false; window.clearInterval(timer); };
  }, [client, changed]);

  const navigable = load.phase === "ready" || load.phase === "error";
  const offered = SCREENS.filter(({ id }) => load.phase === "ready" || id === "recovery" || id === "diagnostics");
  return (
    <main className="app-shell" data-nav={navigable ? "true" : "false"}>
      <header className="app-header">
        <h1>Azeroth Navigator</h1>
        <span className="app-ornament" aria-hidden="true" />
        <details><summary>Optional reference window</summary><ReferenceWindowControls client={client} initialTopmost={savedTopmost}/></details>
      </header>
      <div className="app-body">
      {navigable && (
        <nav className="app-nav" aria-label="Primary">
          {NAV_GROUPS.map(({ title, ids }) => {
            const items = offered.filter(({ id }) => ids.includes(id));
            if (items.length === 0) return null;
            return (
              <div className="app-nav-group" role="group" aria-labelledby={`${navIds}-${navSlug(title)}`} key={title}>
                <p className="app-nav-title" id={`${navIds}-${navSlug(title)}`}>{title}</p>
                <ul className="app-nav-list">
                  {items.map(({ id, label }) => (
                    <li key={id}>
                      <button
                        type="button"
                        className="app-nav-button"
                        aria-current={screen === id ? "page" : undefined}
                        onClick={() => setScreen(id)}
                      >
                        {label}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            );
          })}
        </nav>
      )}
      <div className="app-content">
      {load.phase === "loading" && <p role="status">Loading…</p>}
      {load.phase === "error" && (
        <p role="alert">
          Error {load.error.code}: {load.error.message}
          {load.error.retryable ? " (you can retry)" : ""}
        </p>
      )}
      {load.phase === "error" && <Button variant="primary" onClick={changed}>Retry loading</Button>}
      {((load.phase === "error" && screen !== "diagnostics") || screen === "recovery") && <RecoveryPanel client={client} onRestored={restored} onChecked={changed} awaitingStartup={load.phase === "error" && load.error.code === "PROVIDER_UNAVAILABLE" && load.error.retryable} />}
      {(load.phase === "ready" || load.phase === "error") && screen === "diagnostics" && <DiagnosticsPanel client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "compatibility" && <CompatibilityPanel client={client} profileId={load.status.active_profile} refreshKey={revision} />}
      {load.phase === "ready" && screen === "today" && <><TodayScreen client={client} refreshKey={revision}/><Today status={load.status} snapshot={load.snapshot} freshness={load.freshness}/></>}
      {load.phase === "ready" && screen === "atlas" && <AtlasScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "settings" && <SettingsScreen client={client} refreshKey={revision} onChanged={changed}/>}
      {load.phase === "ready" && screen === "imports" && <ImportScreen client={client} onChanged={changed} />}
      {load.phase === "ready" && screen === "party" && <PartyScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "dungeons" && <DungeonsScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "planner" && <PlannerScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "gear" && <GearScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "sessions" && <SessionsScreen client={client} refreshKey={revision} onChanged={changed} />}
      {load.phase === "ready" && screen === "reminders" && <ReminderScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "content" && <ContentScreen client={client} onChanged={changed} refreshKey={revision} />}
      </div>
      </div>
    </main>
  );
}

function Today({ status, snapshot, freshness }: { status: AppStatus; snapshot: CharacterSnapshotEnvelope | null; freshness: Freshness | null }) {
  return (
    <section aria-labelledby="today">
      {status.data_mode === "demo" && (
        <p className="demo-label" role="note">
          DEMO — synthetic data, not from your game
        </p>
      )}
      {status.data_mode === "stored" && snapshot !== null && (snapshot.fixture || snapshot.namespace === "demo") && (
        <p className="demo-label" role="note">SYNTHETIC — fixture or demo data, not from your game</p>
      )}
      <h2 id="today">Imported character reference</h2>
      {freshness !== null && <p>Last imported: {describeAge(freshness)}</p>}
      {snapshot === null ? (
        <p>No character snapshot has been imported.</p>
      ) : (
        <dl>
          <dt>Snapshot</dt>
          <dd>{snapshot.payload.id}</dd>
          <dt>Level</dt>
          <dd>{describeObservation(snapshot.payload.level)}</dd>
          <dt>Role</dt>
          <dd>{describeObservation(snapshot.payload.role)}</dd>
          <dt>Location</dt>
          <dd>{describeObservation(snapshot.payload.location, (p) => `${p.map.map_id} (${p.x}, ${p.y})`)}</dd>
        </dl>
      )}
      <h3>Capabilities</h3>
      <ul>
        {Object.entries(status.capabilities).map(([name, state]) => (
          <li key={name}>
            {name}: {state}
          </li>
        ))}
      </ul>
    </section>
  );
}
