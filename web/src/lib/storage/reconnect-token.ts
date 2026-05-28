const RECONNECT_TOKEN_KEY = 'play-room.reconnect-token';
const DISPLAY_NAME_KEY = 'play-room.display-name';
const SERVER_URL_KEY = 'play-room.server-url';

export function saveReconnectToken(token: string): void {
  sessionStorage.setItem(RECONNECT_TOKEN_KEY, token);
}

export function loadReconnectToken(): string | null {
  return sessionStorage.getItem(RECONNECT_TOKEN_KEY);
}

export function clearReconnectToken(): void {
  sessionStorage.removeItem(RECONNECT_TOKEN_KEY);
  localStorage.removeItem(RECONNECT_TOKEN_KEY);
}

export function saveDisplayName(displayName: string): void {
  localStorage.setItem(DISPLAY_NAME_KEY, displayName);
}

export function loadDisplayName(): string {
  return localStorage.getItem(DISPLAY_NAME_KEY) ?? '';
}

export function saveServerUrl(serverUrl: string): void {
  localStorage.setItem(SERVER_URL_KEY, serverUrl);
}

export function loadServerUrl(defaultServerUrl: string): string {
  return localStorage.getItem(SERVER_URL_KEY) ?? defaultServerUrl;
}
