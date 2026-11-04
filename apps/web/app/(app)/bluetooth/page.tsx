"use client";
import {
  Alert02Icon,
  AlertCircleIcon,
  BluetoothIcon,
  BluetoothSearchIcon,
  ComputerIcon,
  Delete02Icon,
  HeadphonesIcon,
  KeyboardIcon,
  LaptopIcon,
  Mouse01Icon,
  SmartPhone01Icon,
  Speaker01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import { useState } from "react";
import { PageHeader } from "@/components/page-header";
import { useUser } from "@/components/session";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldDescription, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemMedia, ItemSeparator, ItemTitle } from "@/components/ui/item";
import { Skeleton } from "@/components/ui/skeleton";
import { api, type Bluetooth, type Settings } from "@/lib/api";
import { attempt } from "@/lib/errors";
import { usePoll } from "@/lib/hooks";
import { toast } from "sonner";

const ICONS: Record<string, IconSvgElement> = {
  phone: SmartPhone01Icon,
  computer: LaptopIcon,
  "audio-headset": HeadphonesIcon,
  "audio-headphones": HeadphonesIcon,
  "audio-card": Speaker01Icon,
  "input-keyboard": KeyboardIcon,
  "input-mouse": Mouse01Icon,
};

export default function BluetoothPage() {
  const admin = useUser().role === "admin";
  const bt = usePoll<Bluetooth>("/api/audio/bluetooth", 3000);
  const [name, setName] = useState<string>();
  const b = bt.data;

  const saveName = () =>
    attempt(async () => {
      const r = await api<{ settings: Settings; warning?: string }>("/api/settings", {
        method: "PUT",
        body: { "bluetooth.name": name },
      });
      setName(undefined);
      if (r.warning) toast.warning(r.warning);
      else toast.success(`Visible as “${r.settings["bluetooth.name"]}”`);
      await bt.reload();
    });

  const pairing = (open: boolean) =>
    attempt(
      async () =>
        bt.setData(
          await api<Bluetooth>("/api/audio/bluetooth/pairing", open ? { method: "POST", body: { seconds: 180 } } : { method: "DELETE" }),
        ),
      open ? "Pairing open for 3 minutes" : "Pairing closed",
    );

  const forget = (address: string, label: string) =>
    attempt(
      async () => bt.setData(await api<Bluetooth>(`/api/audio/bluetooth/devices/${encodeURIComponent(address)}`, { method: "DELETE" })),
      `${label} forgotten`,
    );

  if (bt.error && !b)
    return (
      <Alert variant="destructive">
        <HugeiconsIcon icon={AlertCircleIcon} strokeWidth={2} />
        <AlertTitle>Bluetooth unavailable</AlertTitle>
        <AlertDescription>{bt.error}</AlertDescription>
      </Alert>
    );
  if (!b)
    return (
      <div className="space-y-6">
        <Skeleton className="h-8 w-40" />
        <Skeleton className="h-40 w-full rounded-xl" />
        <Skeleton className="h-64 w-full rounded-xl" />
      </div>
    );

  const dirty = name !== undefined && name.trim() !== "" && name !== b.name;

  return (
    <>
      <PageHeader
        title="Bluetooth"
        description="Phones and computers see Charlotte as a speaker. Headphones and speakers can receive the mix."
      />

      {!b.audio_class && (
        <Alert>
          <HugeiconsIcon icon={Alert02Icon} strokeWidth={2} />
          <AlertTitle>iPhones cannot see this speaker</AlertTitle>
          <AlertDescription>
            This machine announces itself as a {b.class_name}. Run <code>services/audio-server/scripts/setup-bluetooth.sh</code> once
            on the server.
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle>This speaker</CardTitle>
          <CardDescription className="font-mono">{b.address}</CardDescription>
          <CardAction className="flex gap-1.5">
            <Badge variant={b.powered ? "secondary" : "destructive"}>{b.powered ? "on" : "off"}</Badge>
            <Badge variant="outline">{b.class_name}</Badge>
          </CardAction>
        </CardHeader>
        <CardContent>
          <form
            className="flex flex-col gap-3 sm:flex-row sm:items-end"
            onSubmit={(e) => {
              e.preventDefault();
              void saveName();
            }}
          >
            <Field className="flex-1">
              <FieldLabel htmlFor="bt-name">Name shown to other devices</FieldLabel>
              <Input id="bt-name" value={name ?? b.name} disabled={!admin} maxLength={60} onChange={(e) => setName(e.target.value)} />
              {!admin && <FieldDescription>Only administrators can rename the speaker.</FieldDescription>}
            </Field>
            {admin && (
              <Button type="submit" disabled={!dirty}>
                Save name
              </Button>
            )}
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            Pair a new device
            {b.discoverable && <Badge>open</Badge>}
          </CardTitle>
          <CardDescription>
            {b.discoverable
              ? `“${b.name}” is visible now. On the phone or computer, open the Bluetooth settings and choose it.`
              : "New devices can pair only while pairing is open. Paired devices reconnect any time."}
          </CardDescription>
          {admin && (
            <CardAction>
              {b.discoverable ? (
                <Button variant="outline" onClick={() => void pairing(false)}>
                  Stop
                </Button>
              ) : (
                <Button onClick={() => void pairing(true)}>
                  <HugeiconsIcon icon={BluetoothSearchIcon} strokeWidth={2} />
                  Open pairing
                </Button>
              )}
            </CardAction>
          )}
        </CardHeader>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Devices</CardTitle>
          <CardDescription>
            {b.devices.filter((d) => d.connected).length} connected · {b.devices.length} known
          </CardDescription>
        </CardHeader>
        <CardContent>
          {b.devices.length === 0 ? (
            <Empty className="border border-dashed">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <HugeiconsIcon icon={BluetoothIcon} strokeWidth={2} />
                </EmptyMedia>
                <EmptyTitle>No devices yet</EmptyTitle>
                <EmptyDescription>Open pairing, then connect a phone or a computer.</EmptyDescription>
              </EmptyHeader>
            </Empty>
          ) : (
            <ItemGroup>
              {b.devices.map((d, i) => {
                const label = d.name || d.address;
                return (
                  <div key={d.address}>
                    {i > 0 && <ItemSeparator />}
                    <Item className="px-0">
                      <ItemMedia variant="icon">
                        <HugeiconsIcon icon={ICONS[d.icon] ?? ComputerIcon} strokeWidth={2} />
                      </ItemMedia>
                      <ItemContent>
                        <ItemTitle>{label}</ItemTitle>
                        <ItemDescription className="font-mono text-xs">{d.address}</ItemDescription>
                      </ItemContent>
                      <ItemActions>
                        {d.connected ? <Badge variant="secondary">connected</Badge> : <Badge variant="outline">offline</Badge>}
                        {d.trusted && <Badge variant="outline">trusted</Badge>}
                        {admin && (
                          <AlertDialog>
                            <AlertDialogTrigger asChild>
                              <Button variant="ghost" size="icon" aria-label={`Forget ${label}`}>
                                <HugeiconsIcon icon={Delete02Icon} strokeWidth={2} />
                              </Button>
                            </AlertDialogTrigger>
                            <AlertDialogContent>
                              <AlertDialogHeader>
                                <AlertDialogTitle>Forget {label}?</AlertDialogTitle>
                                <AlertDialogDescription>
                                  It is disconnected and must be paired again. Also remove “{b.name}” from the device’s own Bluetooth settings.
                                </AlertDialogDescription>
                              </AlertDialogHeader>
                              <AlertDialogFooter>
                                <AlertDialogCancel>Cancel</AlertDialogCancel>
                                <AlertDialogAction variant="destructive" onClick={() => void forget(d.address, label)}>
                                  Forget
                                </AlertDialogAction>
                              </AlertDialogFooter>
                            </AlertDialogContent>
                          </AlertDialog>
                        )}
                      </ItemActions>
                    </Item>
                  </div>
                );
              })}
            </ItemGroup>
          )}
        </CardContent>
      </Card>
    </>
  );
}
