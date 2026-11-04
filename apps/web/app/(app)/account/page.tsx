"use client";
import { useState } from "react";
import { useUser } from "@/components/session";
import { Alert, Button, Card, Input } from "@/components/ui";
import { api } from "@/lib/api";

export default function AccountPage() {
  const me = useUser();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [repeat, setRepeat] = useState("");
  const [msg, setMsg] = useState<{ tone: "ok" | "danger"; text: string }>();

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (next !== repeat) return setMsg({ tone: "danger", text: "The new passwords do not match." });
    try {
      await api("/api/auth/password", { method: "POST", body: { current, next } });
      setCurrent("");
      setNext("");
      setRepeat("");
      setMsg({ tone: "ok", text: "Password changed. Other sessions were signed out." });
    } catch (err) {
      setMsg({ tone: "danger", text: (err as Error).message });
    }
  }

  return (
    <>
      <h1 className="text-2xl font-semibold">My account</h1>
      <Card title="Profile">
        <dl className="grid grid-cols-[8rem_1fr] gap-y-2 text-sm">
          <dt className="text-muted">Username</dt>
          <dd>@{me.username}</dd>
          <dt className="text-muted">Role</dt>
          <dd>{me.role}</dd>
        </dl>
      </Card>
      <Card title="Change password">
        <form onSubmit={submit} className="max-w-sm space-y-3">
          {msg && <Alert tone={msg.tone}>{msg.text}</Alert>}
          <Input label="Current password" type="password" autoComplete="current-password" required value={current} onChange={(e) => setCurrent(e.target.value)} />
          <Input label="New password (min 10)" type="password" autoComplete="new-password" minLength={10} required value={next} onChange={(e) => setNext(e.target.value)} />
          <Input label="Repeat new password" type="password" autoComplete="new-password" required value={repeat} onChange={(e) => setRepeat(e.target.value)} />
          <Button type="submit">Change password</Button>
        </form>
      </Card>
    </>
  );
}
