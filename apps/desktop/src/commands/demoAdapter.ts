// Browser-only adapter serving the synthetic fixture. Used for `vite dev`,
// unit tests and the explicit demo mode; it can never report stored data.
import type { CharacterSnapshotEnvelope } from "../../../../schemas/v1/exchange";
import fixture from "../../../../fixtures/contracts/v1/character-snapshot.json";
import type { AppError, CommandAdapter, CommandMap, CommandName } from "./contract";

const snapshot = fixture as unknown as CharacterSnapshotEnvelope;
export const DEMO_PROFILE_ID = snapshot.payload.character;

export const demoAdapter: CommandAdapter = {
  invoke<C extends CommandName>(command: C, input: CommandMap[C]["input"]) {
    const answer = ((): CommandMap[CommandName]["output"] => {
      switch (command) {
        case "get_app_status":
          return {
            version: "0.1.0",
            data_mode: "demo",
            active_profile: DEMO_PROFILE_ID,
            capabilities: { addon_transport: "unverified", live_position: "unverified" },
          };
        case "list_profiles":
          return [{ id: DEMO_PROFILE_ID, display_name: "Demo paladin", data_mode: "demo" }];
        case "get_character_snapshot": {
          const { profile_id } = input as CommandMap["get_character_snapshot"]["input"];
          if (profile_id !== DEMO_PROFILE_ID) {
            const err: AppError = { code: "INVALID_INPUT", message: "No such profile.", retryable: false };
            throw err;
          }
          return snapshot;
        }
        default:
          throw { code: "INVALID_INPUT", message: "Unknown command.", retryable: false } satisfies AppError;
      }
    })();
    return Promise.resolve(answer as CommandMap[C]["output"]);
  },
};
