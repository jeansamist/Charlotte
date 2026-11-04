"use client";
import { useState } from "react";
import { useUser } from "@/components/session";
import { Alert, Badge, Button, Card, Input } from "@/components/ui";
import { api, type Bluetooth, type Settings } from "@/lib/api";
import { usePoll } from "@/lib/hooks";

const ICON: Record<string, string> = {
  phone: "📱",
  computer: "💻",
  "audio-headset": "🎧",
  "audio-headphones": "🎧",
  "audio-card": "🔈",
  "input-keyboard": "⌨️",
  "input-mouse": "🖱️",
};

export default function BluetoothPage() {
  const user = useUser();
  const admin = user.role === "admin";
  const bt = usePoll<Bluetooth>("/api/audio/bluetooth", 3000);
  const [name, setName] = useState<string>();
  const [msg, setMsg] = useState<{ tone: "ok" | "warn" | "danger"; text: string }>();
  const b = bt.data;

  async function run(fn: () => Promise<void>) {
    setMsg(undefined);
    try {
      await fn();
    } catch (e) {
      setMsg({ tone: "danger", text: (e as Error).message });
    }
  }

  const saveName = () =>
    run(async () => {
      const r = await api<{ settings: Settings; warning?: string }>("/api/settings", {
        method: "PUT",
        body: { "bluetooth.name": name },
      });
      setName(undefined);
      setMsg(r.warning ? { tone: "warn", text: r.warning } : { tone: "ok", text: `Charlotte is now visible as “${r.settings["bluetooth.name"]}”.` });
      await bt.reload();
    });

  const pairing = (open: boolean) =>
    run(async () => {
      bt.setData(
        await api<Bluetooth>("/api/audio/bluetooth/pairing", open ? { method: "POST", body: { seconds: 180 } } : { method: "DELETE" }),
      );
    });

  const forget = (address: string, label: string) =>
    run(async () => {
      if (!confirm(`Forget ${label}? It will have to pair again.`)) return;
      bt.setData(await api<Bluetooth>(`/api/audio/bluetooth/devices/${encodeURIComponent(address)}`, { method: "DELETE" }));
    });

  if (bt.error && !b) return <Alert>Bluetooth: {bt.error}</Alert>;
  if (!b) return null;

  return (
    <>
      <div>
        <h1 className="text-2xl font-semibold">Bluetooth</h1>
        <p className="text-sm text-muted">Phones and computers see Charlotte as a speaker. Headphones and speakers can receive the mix.</p>
      </div>
      {msg && <Alert tone={msg.tone}>{msg.text}</Alert>}
      {!b.audio_class && (
        <Alert tone="warn">
          This machine announces itself as a <b>{b.class_name}</b>, so iPhones won’t list it. Run{" "}
          <code>services/audio-server/scripts/setup-bluetooth.sh</code> once on the server.
        </Alert>
      )}

      <Card title="This speaker">
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-3">
            <Input
              label="Name shown to other devices"
              value={name ?? b.name}
              disabled={!admin}
              maxLength={60}
              onChange={(e) => setName(e.target.value)}
            />
            {admin && name !== undefined && name.trim() && name !== b.name && <Button onClick={() => void saveName()}>Save name</Button>}
          </div>
          <dl className="grid grid-cols-2 gap-y-2 text-sm">
            <dt className="text-muted">Address</dt>
            <dd className="font-mono">{b.address}</dd>
            <dt className="text-muted">Type</dt>
            <dd>{b.class_name}</dd>
            <dt className="text-muted">Power</dt>
            <dd>{b.powered ? <Badge tone="ok">on</Badge> : <Badge tone="danger">off</Badge>}</dd>
            <dt className="text-muted">Pairing</dt>
            <dd>{b.discoverable ? <Badge tone="accent">open</Badge> : <Badge>closed</Badge>}</dd>
          </dl>
        </div>
      </Card>

      <Card
        title="Pair a new device"
        actions={
          admin &&
          (b.discoverable ? (
            <Button variant="ghost" onClick={() => void pairing(false)}>
              Stop
            </Button>
          ) : (
            <Button onClick={() => void pairing(true)}>Open pairing (3 min)</Button>
          ))
        }
      >
        <p className="text-sm text-muted">
          {b.discoverable
            ? `“${b.name}” is visible now. On the phone or computer, open Bluetooth settings and choose it. Pairing is accepted automatically.`
            : "New devices can only pair while pairing is open. Devices already paired reconnect any time."}
        </p>
      </Card>

      <Card title={`Devices (${b.devices.length})`}>
        <ul className="divide-y divide-line">
          {b.devices.map((d) => (
            <li key={d.address} className="flex flex-wrap items-center gap-3 py-3 first:pt-0 last:pb-0">
              <span className="text-xl" aria-hidden>
                {ICON[d.icon] ?? "🔵"}
              </span>
              <div className="min-w-0 flex-1">
                <div className="font-medium">{d.name || d.address}</div>
                <div className="font-mono text-xs text-muted">{d.address}</div>
              </div>
              {d.connected ? <Badge tone="ok">connected</Badge> : <Badge>offline</Badge>}
              {d.trusted && <Badge tone="accent">trusted</Badge>}
              {admin && (
                <Button variant="ghost" onClick={() => void forget(d.address, d.name || d.address)}>
                  Forget
                </Button>
              )}
            </li>
          ))}
        </ul>
      </Card>
    </>
  );
}
