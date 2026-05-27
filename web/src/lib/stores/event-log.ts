import { writable } from 'svelte/store';

export type LogLevel = 'info' | 'success' | 'warning' | 'error' | 'protocol';

export interface EventLogEntry {
  id: number;
  at: string;
  level: LogLevel;
  message: string;
  raw?: unknown;
}

const MAX_LOG_ENTRIES = 200;
let nextLogId = 1;

function createLogStore() {
  const { subscribe, set, update } = writable<{ logs: EventLogEntry[] }>({ logs: [] });

  return {
    subscribe,
    clear: () => set({ logs: [] }),
    push: (level: LogLevel, message: string, raw?: unknown) => {
      update(state => {
        const newLog: EventLogEntry = {
          id: nextLogId++,
          at: new Date().toLocaleTimeString(),
          level,
          message,
          raw,
        };
        // Bounded array growth prevents memory leak
        const logs = [newLog, ...state.logs].slice(0, MAX_LOG_ENTRIES);
        return { logs };
      });
    }
  };
}

export const logStore = createLogStore();