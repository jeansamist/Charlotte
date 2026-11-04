"use client";
import { LockPasswordIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { toast } from "sonner";
import { PageHeader } from "@/components/page-header";
import { useUser } from "@/components/session";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { api } from "@/lib/api";
import { attempt } from "@/lib/errors";

export default function AccountPage() {
  const me = useUser();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [repeat, setRepeat] = useState("");
  const name = me.displayName || me.username;

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (next !== repeat) return void toast.error("The new passwords do not match.");
    const ok = await attempt(
      () => api("/api/auth/password", { method: "POST", body: { current, next } }),
      "Password changed. Other sessions were signed out.",
    );
    if (ok) {
      setCurrent("");
      setNext("");
      setRepeat("");
    }
  }

  return (
    <>
      <PageHeader title="My account" />
      <Card>
        <CardHeader className="flex flex-row items-center gap-4">
          <Avatar className="size-12">
            <AvatarFallback className="text-base">{name.slice(0, 2).toUpperCase()}</AvatarFallback>
          </Avatar>
          <div className="space-y-1">
            <CardTitle>{name}</CardTitle>
            <CardDescription className="flex items-center gap-2">
              @{me.username} <Badge variant={me.role === "admin" ? "default" : "secondary"}>{me.role}</Badge>
            </CardDescription>
          </div>
        </CardHeader>
      </Card>
      <Card>
        <form onSubmit={submit}>
          <CardHeader>
            <CardTitle>Change password</CardTitle>
            <CardDescription>Your other sessions will be signed out.</CardDescription>
          </CardHeader>
          <CardContent className="py-6">
            <FieldGroup className="max-w-md">
              <Field>
                <FieldLabel htmlFor="current">Current password</FieldLabel>
                <Input id="current" type="password" autoComplete="current-password" required value={current} onChange={(e) => setCurrent(e.target.value)} />
              </Field>
              <Field>
                <FieldLabel htmlFor="next">New password</FieldLabel>
                <Input id="next" type="password" autoComplete="new-password" minLength={10} required value={next} onChange={(e) => setNext(e.target.value)} />
                <FieldDescription>At least 10 characters.</FieldDescription>
              </Field>
              <Field>
                <FieldLabel htmlFor="repeat">Repeat new password</FieldLabel>
                <Input id="repeat" type="password" autoComplete="new-password" required value={repeat} onChange={(e) => setRepeat(e.target.value)} />
              </Field>
            </FieldGroup>
          </CardContent>
          <CardFooter className="justify-end">
            <Button type="submit">
              <HugeiconsIcon icon={LockPasswordIcon} strokeWidth={2} />
              Change password
            </Button>
          </CardFooter>
        </form>
      </Card>
    </>
  );
}
