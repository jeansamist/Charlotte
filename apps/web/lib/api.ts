export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

/** JSON fetch against the gateway (same origin). Redirects to /login on 401. */
export async function api<T = unknown>(path: string, init: { method?: string; body?: unknown } = {}): Promise<T> {
  const res = await fetch(path, {
    method: init.method ?? "GET",
    headers: init.body === undefined ? {} : { "content-type": "application/json" },
    body: init.body === undefined ? undefined : JSON.stringify(init.body),
    credentials: "same-origin",
    cache: "no-store",
  });
  const data = await res.json().catch(() => ({}));
  if (res.status === 401 && typeof window !== "undefined" && !window.location.pathname.startsWith("/login")) {
    window.location.href = "/login";
  }
  if (!res.ok) throw new ApiError(res.status, (data as { error?: string }).error ?? `HTTP ${res.status}`);
  return data as T;
}

export type Role = "admin" | "member";
export interface User {
  id: string;
  username: string;
  displayName: string;
  role: Role;
  createdAt?: string;
}
export interface Channel {
  slot: number;
  name: string;
  node: string;
  kind: "bluetooth" | "simulated";
  volume: number;
  muted: boolean;
  peak: number;
  streaming: boolean;
  buffered_ms: number;
  underruns: number;
  active: boolean;
  ducked: boolean;
  duck_gain: number;
}
export interface Mixer {
  master: { volume: number; muted: boolean; peak: number; limiter_gain: number };
  channels: Channel[];
}
export interface Outputs {
  current: string | null;
  outputs: { id: number; name: string; description: string; kind: string }[];
}
export interface BtDevice {
  address: string;
  name: string;
  icon: string;
  paired: boolean;
  trusted: boolean;
  connected: boolean;
}
export interface Bluetooth {
  adapter: string;
  address: string;
  name: string;
  class_name: string;
  audio_class: boolean;
  powered: boolean;
  discoverable: boolean;
  discoverable_timeout: number;
  pairable: boolean;
  devices: BtDevice[];
}
export interface Settings {
  "assistant.name": string;
  "assistant.language": "en" | "fr";
  "bluetooth.name": string;
}
