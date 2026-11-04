"use client";
import { useState } from "react";
import { useUser } from "@/components/session";
import { Alert, Badge, Button, Card, Input, Select } from "@/components/ui";
import { api, type User } from "@/lib/api";
import { usePoll } from "@/lib/hooks";

export default function UsersPage() {
  const me = useUser();
  const list = usePoll<{ users: User[] }>("/api/users");
  const [form, setForm] = useState({ username: "", displayName: "", password: "", role: "member" });
  const [msg, setMsg] = useState<{ tone: "ok" | "danger"; text: string }>();

  if (me.role !== "admin") return <Alert>Administrators only.</Alert>;

  async function run(fn: () => Promise<void>, ok?: string) {
    setMsg(undefined);
    try {
      await fn();
      if (ok) setMsg({ tone: "ok", text: ok });
      await list.reload();
    } catch (e) {
      setMsg({ tone: "danger", text: (e as Error).message });
    }
  }

  const create = (e: React.FormEvent) => {
    e.preventDefault();
    void run(async () => {
      await api("/api/users", { method: "POST", body: form });
      setForm({ username: "", displayName: "", password: "", role: "member" });
    }, `User ${form.username} created.`);
  };

  const resetPassword = (u: User) => {
    const password = prompt(`New password for ${u.username} (min 10 characters):`);
    if (password) void run(() => api(`/api/users/${u.id}`, { method: "PATCH", body: { password } }).then(() => {}), "Password changed.");
  };
  const toggleRole = (u: User) =>
    void run(() => api(`/api/users/${u.id}`, { method: "PATCH", body: { role: u.role === "admin" ? "member" : "admin" } }).then(() => {}));
  const remove = (u: User) => {
    if (confirm(`Delete ${u.username}?`)) void run(() => api(`/api/users/${u.id}`, { method: "DELETE" }).then(() => {}), "User deleted.");
  };

  return (
    <>
      <h1 className="text-2xl font-semibold">Users</h1>
      {msg && <Alert tone={msg.tone}>{msg.text}</Alert>}
      <Card title="Accounts">
        <ul className="divide-y divide-line">
          {list.data?.users.map((u) => (
            <li key={u.id} className="flex flex-wrap items-center gap-3 py-3 first:pt-0 last:pb-0">
              <div className="min-w-0 flex-1">
                <div className="font-medium">
                  {u.displayName || u.username} {u.id === me.id && <span className="text-xs text-muted">(you)</span>}
                </div>
                <div className="text-xs text-muted">@{u.username}</div>
              </div>
              <Badge tone={u.role === "admin" ? "accent" : "muted"}>{u.role}</Badge>
              <Button variant="ghost" onClick={() => resetPassword(u)}>
                Reset password
              </Button>
              {u.id !== me.id && (
                <>
                  <Button variant="ghost" onClick={() => toggleRole(u)}>
                    Make {u.role === "admin" ? "member" : "admin"}
                  </Button>
                  <Button variant="danger" onClick={() => remove(u)}>
                    Delete
                  </Button>
                </>
              )}
            </li>
          ))}
        </ul>
      </Card>
      <Card title="Add a user">
        <form onSubmit={create} className="grid gap-4 md:grid-cols-2">
          <Input label="Username" value={form.username} required onChange={(e) => setForm({ ...form, username: e.target.value })} />
          <Input label="Display name" value={form.displayName} onChange={(e) => setForm({ ...form, displayName: e.target.value })} />
          <Input label="Password (min 10)" type="password" minLength={10} required value={form.password} onChange={(e) => setForm({ ...form, password: e.target.value })} />
          <Select label="Role" value={form.role} onChange={(e) => setForm({ ...form, role: e.target.value })}>
            <option value="member">Member</option>
            <option value="admin">Administrator</option>
          </Select>
          <div>
            <Button type="submit">Create</Button>
          </div>
        </form>
      </Card>
    </>
  );
}
