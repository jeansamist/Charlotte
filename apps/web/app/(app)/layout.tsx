"use client";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { SessionContext } from "@/components/session";
import { Spinner, cx } from "@/components/ui";
import { api, type User } from "@/lib/api";

const NAV = [
  { href: "/", label: "Audio", icon: "🎚️" },
  { href: "/bluetooth", label: "Bluetooth", icon: "📶" },
  { href: "/settings", label: "Settings", icon: "⚙️" },
  { href: "/users", label: "Users", icon: "👥", admin: true },
  { href: "/account", label: "My account", icon: "👤" },
];

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const pathname = usePathname();
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    api<{ user: User }>("/api/auth/me")
      .then((r) => setUser(r.user))
      .catch(() => router.replace("/login"));
  }, [router]);

  async function logout() {
    await api("/api/auth/logout", { method: "POST" }).catch(() => {});
    router.replace("/login");
  }

  if (!user)
    return (
      <div className="flex min-h-screen items-center justify-center">
        <Spinner />
      </div>
    );

  return (
    <SessionContext.Provider value={user}>
      <div className="mx-auto flex min-h-screen max-w-6xl flex-col gap-6 p-4 md:flex-row md:p-6">
        <aside className="md:w-56 md:shrink-0">
          <div className="mb-4 flex items-center gap-2 px-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-accent font-bold text-white">C</div>
            <div>
              <div className="font-semibold leading-tight">Charlotte</div>
              <div className="text-xs text-muted">{user.displayName || user.username}</div>
            </div>
          </div>
          <nav className="flex gap-1 overflow-x-auto md:flex-col">
            {NAV.filter((n) => !n.admin || user.role === "admin").map((n) => (
              <Link
                key={n.href}
                href={n.href}
                className={cx(
                  "flex items-center gap-2 whitespace-nowrap rounded-xl px-3 py-2 text-sm",
                  pathname === n.href ? "bg-accent-soft font-medium text-accent" : "hover:bg-accent-soft/60",
                )}
              >
                <span aria-hidden>{n.icon}</span>
                {n.label}
              </Link>
            ))}
            <button onClick={logout} className="flex items-center gap-2 whitespace-nowrap rounded-xl px-3 py-2 text-left text-sm text-muted hover:bg-accent-soft/60">
              <span aria-hidden>↩︎</span> Sign out
            </button>
          </nav>
        </aside>
        <main className="min-w-0 flex-1 space-y-6">{children}</main>
      </div>
    </SessionContext.Provider>
  );
}
