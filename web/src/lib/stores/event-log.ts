import { writable } from 'svelte/store';

export type LogLevel = 'info' | 'success' | 'warning' | 'error' | 'protocol';

export interface EventLogEntry {
  id: number;
  at: string;
  level: LogLevel;
  message: string;
  raw?: unknown;
}

const MAX_LOG_ENTRIES = 120;
let nextLogId = 1;

function createEventLogStore() {
  const { subscribe, set, update } = writable<{ logs: EventLogEntry[] }>({ logs: [] });

  return {
    subscribe,
    clear: () => set({ logs: [] }),
    push: (level: LogLevel, message: string, raw?: unknown) => {
      update((state) => {
        const entry: EventLogEntry = {
          id: nextLogId,
          at: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
          level,
          message,
          raw,
        };

        nextLogId += 1;

        return {
          logs: [entry, ...state.logs].slice(0, MAX_LOG_ENTRIES),
        };
      });
    },
  };
}

export const eventLogStore = createEventLogStore();
