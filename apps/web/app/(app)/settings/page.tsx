"use client";
import { AiMagicIcon, BluetoothIcon, FloppyDiskIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { PageHeader } from "@/components/page-header";
import { useUser } from "@/components/session";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { api, type Settings } from "@/lib/api";
import { attempt } from "@/lib/errors";
import { usePoll } from "@/lib/hooks";

export default function SettingsPage() {
  const admin = useUser().role === "admin";
  const remote = usePoll<{ settings: Settings }>("/api/settings");
  const [form, setForm] = useState<Settings>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (remote.data && !form) setForm(remote.data.settings);
  }, [remote.data, form]);

  if (!form)
    return (
      <div className="space-y-6">
        <Skeleton className="h-8 w-40" />
        <Skeleton className="h-56 w-full rounded-xl" />
      </div>
    );

  const set = <K extends keyof Settings>(k: K, v: Settings[K]) => setForm({ ...form, [k]: v });

  async function save(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    await attempt(async () => {
      const r = await api<{ settings: Settings; warning?: string }>("/api/settings", { method: "PUT", body: form });
      setForm(r.settings);
      if (r.warning) toast.warning(r.warning);
      else toast.success("Settings saved");
    });
    setSaving(false);
  }

  return (
    <>
      <PageHeader
        title="Settings"
        description={admin ? "How Charlotte presents itself." : "Read only: ask an administrator to change these."}
      />
      <form onSubmit={save} className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <HugeiconsIcon icon={AiMagicIcon} strokeWidth={2} className="size-4 text-primary" />
              Assistant
            </CardTitle>
            <CardDescription>Its name is also the word that will wake it up.</CardDescription>
          </CardHeader>
          <CardContent>
            <FieldGroup className="grid gap-6 md:grid-cols-2">
              <Field>
                <FieldLabel htmlFor="assistant-name">Name</FieldLabel>
                <Input
                  id="assistant-name"
                  value={form["assistant.name"]}
                  disabled={!admin}
                  maxLength={40}
                  onChange={(e) => set("assistant.name", e.target.value)}
                />
              </Field>
              <Field>
                <FieldLabel>Language</FieldLabel>
                <Select
                  value={form["assistant.language"]}
                  disabled={!admin}
                  onValueChange={(v) => set("assistant.language", v as Settings["assistant.language"])}
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="en">English</SelectItem>
                    <SelectItem value="fr">Français</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </FieldGroup>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <HugeiconsIcon icon={BluetoothIcon} strokeWidth={2} className="size-4 text-primary" />
              Bluetooth
            </CardTitle>
            <CardDescription>The name phones and computers see when they look for a speaker.</CardDescription>
          </CardHeader>
          <CardContent>
            <Field>
              <FieldLabel htmlFor="bt-name">Speaker name</FieldLabel>
              <Input
                id="bt-name"
                value={form["bluetooth.name"]}
                disabled={!admin}
                maxLength={60}
                onChange={(e) => set("bluetooth.name", e.target.value)}
              />
              <FieldDescription>Applied immediately. Already-paired devices may keep showing the old name until they reconnect.</FieldDescription>
            </Field>
          </CardContent>
          {admin && (
            <CardFooter className="justify-end">
              <Button type="submit" disabled={saving}>
                {saving ? <Spinner /> : <HugeiconsIcon icon={FloppyDiskIcon} strokeWidth={2} />}
                Save
              </Button>
            </CardFooter>
          )}
        </Card>
      </form>
    </>
  );
}
