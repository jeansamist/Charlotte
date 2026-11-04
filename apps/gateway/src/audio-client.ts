/** Small client for the audio-server's local HTTP API. */
export class AudioClient {
  constructor(private baseUrl: string) {}

  async request(method: string, path: string, body?: unknown): Promise<{ status: number; data: unknown }> {
    const res = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers: body === undefined ? {} : { "content-type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(8000),
    });
    const text = await res.text();
    let data: unknown = null;
    try {
      data = text ? JSON.parse(text) : null;
    } catch {
      data = { error: text };
    }
    return { status: res.status, data };
  }

  async health(): Promise<{ started_at: number; version: string } | undefined> {
    try {
      const r = await this.request("GET", "/api/health");
      return r.status === 200 ? (r.data as { started_at: number; version: string }) : undefined;
    } catch {
      return undefined;
    }
  }

  async setBluetoothName(name: string): Promise<void> {
    const r = await this.request("PUT", "/api/bluetooth/name", { name });
    if (r.status !== 200) throw new Error(`audio-server: ${JSON.stringify(r.data)}`);
  }
}
