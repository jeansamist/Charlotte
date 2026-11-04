"use client";
import { useEffect, useState } from "react";
import { useUser } from "@/components/session";
import { Alert, Button, Card, Input, Select } from "@/components/ui";
import { api, type Settings } from "@/lib/api";
import { usePoll } from "@/lib/hooks";

export default function SettingsPage() {
  const admin = useUser().role === "admin";
  const remote = usePoll<{ settings: Settings }>("/api/settings");
  const [form, setForm] = useState<Settings>();
  const [msg, setMsg] = useState<{ tone: "ok" | "warn" | "danger"; text: string }>();

  useEffect(() => {
    if (remote.data && !form) setForm(remote.data.settings);
  }, [remote.data, form]);

  if (!form) return null;
  const set = (k: keyof Settings, v: string) => setForm({ ...form, [k]: v } as Settings);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    setMsg(undefined);
    try {
      const r = await api<{ settings: Settings; warning?: string }>("/api/settings", { method: "PUT", body: form });
      setForm(r.settings);
      setMsg(r.warning ? { tone: "warn", text: r.warning } : { tone: "ok", text: "Settings saved." });
    } catch (err) {
      setMsg({ tone: "danger", text: (err as Error).message });
    }
  }

  return (
    <>
      <div>
        <h1 className="text-2xl font-semibold">Settings</h1>
        <p className="text-sm text-muted">{admin ? "Only administrators can change these." : "Read only: ask an administrator to change them."}</p>
      </div>
      {msg && <Alert tone={msg.tone}>{msg.text}</Alert>}
      <form onSubmit={save} className="space-y-6">
        <Card title="Assistant">
          <div className="grid gap-4 md:grid-cols-2">
            <Input label="Name (also the wake word)" value={form["assistant.name"]} disabled={!admin} maxLength={40} onChange={(e) => set("assistant.name", e.target.value)} />
            <Select label="Language" value={form["assistant.language"]} disabled={!admin} onChange={(e) => set("assistant.language", e.target.value)}>
              <option value="en">English</option>
              <option value="fr">Français</option>
            </Select>
          </div>
        </Card>
        <Card title="Bluetooth">
          <Input label="Name shown to phones and computers" value={form["bluetooth.name"]} disabled={!admin} maxLength={60} onChange={(e) => set("bluetooth.name", e.target.value)} />
        </Card>
        {admin && <Button type="submit">Save</Button>}
      </form>
    </>
  );
}
