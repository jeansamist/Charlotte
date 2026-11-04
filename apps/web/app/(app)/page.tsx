"use client";
import { Alert, Badge, Button, Card, Meter, Select, Slider, Toggle } from "@/components/ui";
import { api, type Mixer, type Outputs } from "@/lib/api";
import { usePoll, useThrottle } from "@/lib/hooks";

type MixerWithDucking = Mixer & { ducking: { enabled: boolean; level_db: number } };

export default function AudioPage() {
  const mixer = usePoll<MixerWithDucking>("/api/audio/mixer", 500);
  const outputs = usePoll<Outputs>("/api/audio/outputs", 5000);
  const m = mixer.data;

  const put = async (path: string, body: unknown) => {
    try {
      mixer.setData(await api<MixerWithDucking>(`/api/audio${path}`, { method: "PUT", body }));
    } catch (e) {
      alert((e as Error).message);
    }
  };
  const setVolume = useThrottle((path: string, volume: number) => void put(path, { volume }));

  async function chooseOutput(name: string) {
    try {
      outputs.setData(await api<Outputs>("/api/audio/outputs/current", { method: "PUT", body: { name } }));
    } catch (e) {
      alert((e as Error).message);
    }
  }

  if (mixer.error && !m) return <Alert>Audio server: {mixer.error}</Alert>;
  if (!m) return null;

  return (
    <>
      <div>
        <h1 className="text-2xl font-semibold">Audio</h1>
        <p className="text-sm text-muted">Every device streaming to Charlotte over Bluetooth is mixed into one output.</p>
      </div>

      <Card title="Output">
        <div className="grid gap-4 md:grid-cols-[1fr_auto] md:items-end">
          <Select label="Play the mix on" value={outputs.data?.current ?? ""} onChange={(e) => void chooseOutput(e.target.value)}>
            {!outputs.data?.current && <option value="">—</option>}
            {outputs.data?.outputs.map((o) => (
              <option key={o.id} value={o.name}>
                {o.kind === "bluetooth" ? "🎧 " : "🔈 "}
                {o.description}
              </option>
            ))}
          </Select>
          <Button variant={m.master.muted ? "danger" : "ghost"} onClick={() => void put("/mixer/master/mute", { muted: !m.master.muted })}>
            {m.master.muted ? "🔇 Muted" : "🔊 Mute all"}
          </Button>
        </div>
        <div className="mt-4 space-y-2">
          <div className="flex justify-between text-sm">
            <span>Master volume</span>
            <span className="text-muted">{Math.round(m.master.volume * 100)}%</span>
          </div>
          <Slider value={m.master.volume} onChange={(v) => setVolume("/mixer/master/volume", v)} />
          <Meter peak={m.master.peak} dim={m.master.muted} />
          {m.master.limiter_gain < 0.98 && <p className="text-xs text-warn">Limiter active (preventing distortion)</p>}
        </div>
      </Card>

      <Card
        title="Smart mixing"
        actions={<Toggle checked={m.ducking.enabled} onChange={(enabled) => void put("/mixer/ducking", { enabled })} />}
      >
        <p className="mb-3 text-sm text-muted">
          When a device starts playing, the others are lowered so it can be heard, then restored when it stops.
        </p>
        <div className="flex justify-between text-sm">
          <span>Others are lowered to</span>
          <span className="text-muted">{m.ducking.level_db.toFixed(0)} dB</span>
        </div>
        <input
          type="range"
          min={-40}
          max={-3}
          step={1}
          value={m.ducking.level_db}
          disabled={!m.ducking.enabled}
          onChange={(e) => void put("/mixer/ducking", { level_db: Number(e.target.value) })}
          className="w-full"
        />
      </Card>

      <Card title={`Inputs (${m.channels.length})`}>
        {m.channels.length === 0 && (
          <p className="text-sm text-muted">No device is streaming. Connect a phone or computer to “Charlotte” and play something.</p>
        )}
        <ul className="divide-y divide-line">
          {m.channels.map((c) => (
            <li key={c.slot} className="space-y-2 py-4 first:pt-0 last:pb-0">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">{c.name}</span>
                {c.kind === "simulated" && <Badge>test tone</Badge>}
                {c.active ? <Badge tone="ok">playing</Badge> : <Badge>silent</Badge>}
                {c.ducked && c.active && <Badge tone="warn">lowered</Badge>}
                {c.muted && <Badge tone="danger">muted</Badge>}
                <span className="ml-auto text-xs text-muted">{Math.round(c.volume * 100)}%</span>
              </div>
              <Meter peak={c.peak * c.volume} dim={c.muted} />
              <div className="flex items-center gap-3">
                <Slider value={c.volume} max={2} onChange={(v) => setVolume(`/mixer/channels/${c.slot}/volume`, v)} />
                <Button variant={c.muted ? "danger" : "ghost"} onClick={() => void put(`/mixer/channels/${c.slot}/mute`, { muted: !c.muted })}>
                  {c.muted ? "Unmute" : "Mute"}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      </Card>
    </>
  );
}
