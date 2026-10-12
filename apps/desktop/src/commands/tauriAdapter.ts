import { invoke } from "@tauri-apps/api/core";
import type { CommandAdapter } from "./contract";

/** Native adapter: each command name is a registered Rust command, no dispatcher. */
export const tauriAdapter: CommandAdapter = {
  invoke: (command, input) => invoke(command, input === undefined ? {} : input),
};

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
