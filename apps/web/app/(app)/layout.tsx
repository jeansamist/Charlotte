"use client";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { AppSidebar } from "@/components/app-sidebar";
import { SessionContext } from "@/components/session";
import { ThemeToggle } from "@/components/theme-toggle";
import { Separator } from "@/components/ui/separator";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Spinner } from "@/components/ui/spinner";
import { api, type User } from "@/lib/api";

export default function AppLayout({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    api<{ user: User }>("/api/auth/me")
      .then((r) => setUser(r.user))
      .catch(() => router.replace("/login"));
  }, [router]);

  async function signOut() {
    await api("/api/auth/logout", { method: "POST" }).catch(() => {});
    router.replace("/login");
  }

  if (!user)
    return (
      <div className="flex min-h-svh items-center justify-center">
        <Spinner className="size-6" />
      </div>
    );

  return (
    <SessionContext.Provider value={user}>
      <SidebarProvider>
        <AppSidebar user={user} onSignOut={signOut} />
        <SidebarInset>
          <header className="flex h-14 shrink-0 items-center gap-2 border-b px-4">
            <SidebarTrigger className="-ml-1" />
            <Separator orientation="vertical" className="mr-2 data-[orientation=vertical]:h-4" />
            <span className="text-sm text-muted-foreground">Charlotte</span>
            <div className="ml-auto">
              <ThemeToggle />
            </div>
          </header>
          <div className="mx-auto w-full max-w-5xl flex-1 space-y-6 p-4 md:p-8">{children}</div>
        </SidebarInset>
      </SidebarProvider>
    </SessionContext.Provider>
  );
}
