import type { Page } from "@playwright/test";
import type { CommandMap } from "../../src/commands/contract";

/**
 * A typed stand-in for the Tauri IPC seam. Handlers run in the test process and are
 * checked against `CommandMap` (`satisfies`), so a renamed command or a changed
 * DTO fails type checking instead of silently drifting. The page sees a plain
 * `__TAURI_INTERNALS__.invoke`, which is what makes the app pick the Tauri adapter.
 * This is a mock: it says nothing about the Rust side (see command_parity.rs).
 */
export type Handlers = {
  [K in keyof CommandMap]?: (
    input: CommandMap[K]["input"],
  ) => CommandMap[K]["output"] | Promise<CommandMap[K]["output"]>;
};

export interface MockError {
  code: string;
  message: string;
  retryable: boolean;
}

export const fail = (code: string, message: string, retryable = false): MockError => ({
  code,
  message,
  retryable,
});

type Reply = { ok: true; value: unknown } | { ok: false; error: unknown };

/** Install before `page.goto`. Unlisted commands reject, loudly, as INVALID_INPUT. */
export async function installMock(page: Page, handlers: Handlers): Promise<{ calls: string[] }> {
  const calls: string[] = [];
  const table = handlers as Record<string, ((input: unknown) => unknown) | undefined>;
  await page.exposeFunction("__mockInvoke", async (command: string, input: unknown): Promise<Reply> => {
    calls.push(command);
    const handler = table[command];
    if (!handler) return { ok: false, error: fail("INVALID_INPUT", `Unexpected mock command ${command}`) };
    try {
      return { ok: true, value: await handler(input) };
    } catch (error) {
      return { ok: false, error };
    }
  });
  await page.addInitScript(() => {
    const bridge = (window as unknown as {
      __mockInvoke: (command: string, input: unknown) => Promise<Reply>;
    }).__mockInvoke;
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      value: {
        invoke: async (command: string, input: unknown) => {
          const reply = await bridge(command, input);
          if (reply.ok) return reply.value;
          throw reply.error;
        },
      },
    });
  });
  return { calls };
}
