import { useEffect, useState } from "react";
import type { AppError, AppStatus, CommandClient } from "./commands";
import type { CharacterSnapshotEnvelope } from "../../../schemas/v1/exchange";
import { describeObservation } from "./observation";
import { AtlasScreen } from "./screens/AtlasScreen";

type Screen = "today" | "atlas";
const SCREENS: { id: Screen; label: string }[] = [
  { id: "today", label: "Today" },
  { id: "atlas", label: "Atlas" },
];

type Load =
  | { phase: "loading" }
  | { phase: "error"; error: AppError }
  | { phase: "ready"; status: AppStatus; snapshot: CharacterSnapshotEnvelope | null };

export function App({ client }: { client: CommandClient }) {
  const [load, setLoad] = useState<Load>({ phase: "loading" });
  const [screen, setScreen] = useState<Screen>("today");

  useEffect(() => {
    let live = true;
    void (async () => {
      const status = await client.getAppStatus();
      if (!status.ok) return live && setLoad({ phase: "error", error: status.error });
      let snapshot: CharacterSnapshotEnvelope | null = null;
      if (status.value.active_profile !== null) {
        const snap = await client.getCharacterSnapshot(status.value.active_profile);
        if (!snap.ok) return live && setLoad({ phase: "error", error: snap.error });
        snapshot = snap.value;
      }
      if (live) setLoad({ phase: "ready", status: status.value, snapshot });
    })();
    return () => {
      live = false;
    };
  }, [client]);

  return (
    <main>
      <h1>Azeroth Navigator</h1>
      {load.phase === "loading" && <p role="status">Loading…</p>}
      {load.phase === "error" && (
        <p role="alert">
          Error {load.error.code}: {load.error.message}
          {load.error.retryable ? " (you can retry)" : ""}
        </p>
      )}
      {load.phase === "ready" && (
        <nav aria-label="Primary">
          {SCREENS.map(({ id, label }) => (
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
      {load.phase === "ready" && screen === "today" && <Today status={load.status} snapshot={load.snapshot} />}
      {load.phase === "ready" && screen === "atlas" && <AtlasScreen client={client} />}
    </main>
  );
}

function Today({ status, snapshot }: { status: AppStatus; snapshot: CharacterSnapshotEnvelope | null }) {
  return (
    <section aria-labelledby="today">
      {status.data_mode === "demo" && (
        <p className="demo-label" role="note">
          DEMO — synthetic data, not from your game
        </p>
      )}
      <h2 id="today">Today</h2>
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
