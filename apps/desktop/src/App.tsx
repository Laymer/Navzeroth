import { ReferenceWindowControls } from "./screens/ReferenceWindowControls";
import { DiagnosticsPanel } from "./screens/DiagnosticsPanel";
import { RecoveryPanel } from "./screens/RecoveryPanel";
import { useCallback, useEffect, useRef, useState } from "react";
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

type Screen = "today" | "atlas" | "imports" | "party" | "content" | "planner" | "gear" | "sessions" | "reminders" | "recovery" | "diagnostics";
const SCREENS: { id: Screen; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "diagnostics", label: "Local diagnostics" },
  { id: "recovery", label: "Backup and recovery" },
  { id: "atlas", label: "Atlas" },
  { id: "imports", label: "Imports" },
  { id: "party", label: "Party" },
  { id: "content", label: "Content" },
  { id: "planner", label: "Planner" },
  { id: "gear", label: "Gear" },
  { id: "sessions", label: "Sessions" },
  { id: "reminders", label: "Reminders" },
];

type Load =
  | { phase: "loading" }
  | { phase: "error"; error: AppError }
  | { phase: "ready"; status: AppStatus; snapshot: CharacterSnapshotEnvelope | null; freshness: Freshness | null };

export function App({ client }: { client: CommandClient }) {
  const [load, setLoad] = useState<Load>({ phase: "loading" });
  const [screen, setScreen] = useState<Screen>("today");

  const [revision, setRevision] = useState(0);
  const watchedImport = useRef("");
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

  return (
    <main>
      <h1>Azeroth Navigator</h1>
      <details><summary>Optional reference window</summary><ReferenceWindowControls client={client}/></details>
      {load.phase === "loading" && <p role="status">Loading…</p>}
      {load.phase === "error" && (
        <p role="alert">
          Error {load.error.code}: {load.error.message}
          {load.error.retryable ? " (you can retry)" : ""}
        </p>
      )}
      {load.phase === "error" && <button type="button" onClick={changed}>Retry loading</button>}
      {((load.phase === "error" && screen !== "diagnostics") || screen === "recovery") && <RecoveryPanel client={client} onRestored={restored} onChecked={changed} />}
      {(load.phase === "ready" || load.phase === "error") && (
        <nav aria-label="Primary">
          {SCREENS.filter(({id}) => load.phase === "ready" || id === "recovery" || id === "diagnostics").map(({ id, label }) => (
            <button
              key={id}
              type="button"
              aria-current={screen === id ? "page" : undefined}
              onClick={() => setScreen(id)}
            >
              {label}
            </button>
          ))}
        </nav>
      )}
      {(load.phase === "ready" || load.phase === "error") && screen === "diagnostics" && <DiagnosticsPanel client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "today" && <Today status={load.status} snapshot={load.snapshot} freshness={load.freshness} />}
      {load.phase === "ready" && screen === "atlas" && <AtlasScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "imports" && <ImportScreen client={client} onChanged={changed} />}
      {load.phase === "ready" && screen === "party" && <PartyScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "planner" && <PlannerScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "gear" && <GearScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "sessions" && <SessionsScreen client={client} refreshKey={revision} onChanged={changed} />}
      {load.phase === "ready" && screen === "reminders" && <ReminderScreen client={client} refreshKey={revision} />}
      {load.phase === "ready" && screen === "content" && <ContentScreen client={client} onChanged={changed} refreshKey={revision} />}
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
      <h2 id="today">Today</h2>
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
