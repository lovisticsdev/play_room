import { writable } from 'svelte/store';

export type LogLevel = 'info' | 'success' | 'warning' | 'error' | 'protocol';

export interface EventLogEntry {
  id: number;
  at: string;
  level: LogLevel;
  message: string;
  raw?: unknown;
}

const MAX_VISIBLE_NOTIFICATIONS = 5;
const DEFAULT_TTL_MS = 4200;
const ERROR_TTL_MS = 6800;
let nextLogId = 1;

function createEventLogStore() {
  const { subscribe, set, update } = writable<{ logs: EventLogEntry[] }>({ logs: [] });
  const timers = new Map<number, ReturnType<typeof setTimeout>>();

  function remove(id: number) {
    const timer = timers.get(id);
    if (timer) clearTimeout(timer);
    timers.delete(id);
    update((state) => ({ logs: state.logs.filter((entry) => entry.id !== id) }));
  }

  function clearAll() {
    timers.forEach((timer) => clearTimeout(timer));
    timers.clear();
    set({ logs: [] });
  }

  return {
    subscribe,
    clear: clearAll,
    dismiss: remove,
    push: (level: LogLevel, message: string, raw?: unknown) => {
      const entry: EventLogEntry = {
        id: nextLogId,
        at: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
        level,
        message,
        raw,
      };
      nextLogId += 1;

      update((state) => ({
        logs: [entry, ...state.logs].slice(0, MAX_VISIBLE_NOTIFICATIONS),
      }));

      const ttl = level === 'error' || level === 'warning' ? ERROR_TTL_MS : DEFAULT_TTL_MS;
      timers.set(entry.id, setTimeout(() => remove(entry.id), ttl));
    },
  };
}

export const eventLogStore = createEventLogStore();
