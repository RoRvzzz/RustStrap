/*

Copyright (c) 2026-present, RoRvzzz. All rights reserved.

https://rorvzzz.cool

*/
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type EventName =
  | "bootstrap_status"
  | "progress"
  | "prompt_required"
  | "connectivity_error"
  | "fatal_error"
  | "watcher_activity";

export interface EventPayload {
  timestamp: string;
  event: EventName;
  payload: unknown;
}

export async function subscribeToCoreEvents(
  onEvent: (payload: EventPayload) => void
): Promise<UnlistenFn[]> {
  const eventNames: EventName[] = [
    "bootstrap_status",
    "progress",
    "prompt_required",
    "connectivity_error",
    "fatal_error",
    "watcher_activity"
  ];

  const unlistenFns = await Promise.all(
    eventNames.map((eventName) =>
      listen<unknown>(eventName, (event) => {
        onEvent({
          timestamp: new Date().toISOString(),
          event: eventName,
          payload: event.payload
        });
      })
    )
  );

  return unlistenFns;
}
