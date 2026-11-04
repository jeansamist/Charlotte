"use client";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert, Button, Input } from "@/components/ui";
import { api } from "@/lib/api";

export default function LoginPage() {
  const router = useRouter();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(undefined);
    try {
      await api("/api/auth/login", { method: "POST", body: { username, password } });
      router.replace("/");
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center p-4">
      <form onSubmit={submit} className="w-full max-w-sm space-y-4 rounded-2xl border border-line bg-panel p-6 shadow-sm">
        <div>
          <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-xl bg-accent text-lg font-bold text-white">C</div>
          <h1 className="text-xl font-semibold">Sign in to Charlotte</h1>
          <p className="text-sm text-muted">Your self-hosted home assistant.</p>
        </div>
        {error && <Alert>{error}</Alert>}
        <Input label="Username" autoComplete="username" autoFocus value={username} onChange={(e) => setUsername(e.target.value)} required />
        <Input label="Password" type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} required />
        <Button type="submit" disabled={busy} className="w-full">
          {busy ? "Signing in…" : "Sign in"}
        </Button>
      </form>
    </main>
  );
}
