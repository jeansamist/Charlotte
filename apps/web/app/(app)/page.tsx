"use client";
import {
  AlertCircleIcon,
  AudioWave01Icon,
  HeadphonesIcon,
  MusicNote01Icon,
  SparklesIcon,
  Speaker01Icon,
  VolumeHighIcon,
  VolumeOffIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { LevelMeter } from "@/components/level-meter";
import { PageHeader } from "@/components/page-header";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from "@/components/ui/empty";
import { Field, FieldLabel } from "@/components/ui/field";
import { Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemMedia, ItemSeparator, ItemTitle } from "@/components/ui/item";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { api, type Mixer, type Outputs } from "@/lib/api";
import { attempt } from "@/lib/errors";
import { usePoll, useThrottle } from "@/lib/hooks";

type MixerState = Mixer & { ducking: { enabled: boolean; level_db: number } };

export default function AudioPage() {
  const mixer = usePoll<MixerState>("/api/audio/mixer", 500);
  const outputs = usePoll<Outputs>("/api/audio/outputs", 5000);
  const m = mixer.data;

  const put = (path: string, body: unknown) =>
    attempt(async () => mixer.setData(await api<MixerState>(`/api/audio${path}`, { method: "PUT", body })));
  const setVolume = useThrottle((path: string, volume: number) => void put(path, { volume }));
  const setDuckLevel = useThrottle((level_db: number) => void put("/mixer/ducking", { level_db }));

  const chooseOutput = (name: string) =>
    attempt(async () => outputs.setData(await api<Outputs>("/api/audio/outputs/current", { method: "PUT", body: { name } })), "Output changed");

  if (mixer.error && !m)
    return (
      <Alert variant="destructive">
        <HugeiconsIcon icon={AlertCircleIcon} strokeWidth={2} />
        <AlertTitle>Audio server unavailable</AlertTitle>
        <AlertDescription>{mixer.error}</AlertDescription>
      </Alert>
    );
  if (!m) return <AudioSkeleton />;

  return (
    <>
      <PageHeader title="Audio" description="Every device streaming to Charlotte over Bluetooth is mixed into one output." />

      <Card>
        <CardHeader>
          <CardTitle>Output</CardTitle>
          <CardDescription>Where the mix is played.</CardDescription>
          <CardAction>
            <Button
              variant={m.master.muted ? "destructive" : "outline"}
              onClick={() => void put("/mixer/master/mute", { muted: !m.master.muted })}
            >
              <HugeiconsIcon icon={m.master.muted ? VolumeOffIcon : VolumeHighIcon} strokeWidth={2} />
              {m.master.muted ? "Muted" : "Mute all"}
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent className="space-y-6">
          <Field>
            <FieldLabel>Play the mix on</FieldLabel>
            <Select value={outputs.data?.current ?? undefined} onValueChange={(v) => void chooseOutput(v)}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Choose an output" />
              </SelectTrigger>
              <SelectContent>
                {outputs.data?.outputs.map((o) => (
                  <SelectItem key={o.id} value={o.name}>
                    <HugeiconsIcon icon={o.kind === "bluetooth" ? HeadphonesIcon : Speaker01Icon} strokeWidth={2} />
                    {o.description}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <div className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="font-medium">Master volume</span>
              <span className="tabular-nums text-muted-foreground">{Math.round(m.master.volume * 100)}%</span>
            </div>
            <Slider value={[m.master.volume]} max={1} step={0.01} onValueChange={([v]) => setVolume("/mixer/master/volume", v)} />
            <LevelMeter peak={m.master.peak} dim={m.master.muted} />
            {m.master.limiter_gain < 0.98 && <p className="text-xs text-warning">Limiter active: preventing distortion</p>}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HugeiconsIcon icon={SparklesIcon} strokeWidth={2} className="size-4 text-primary" />
            Smart mixing
          </CardTitle>
          <CardDescription>
            When a device starts playing, the others are lowered so it can be heard, then restored when it stops.
          </CardDescription>
          <CardAction>
            <Switch checked={m.ducking.enabled} onCheckedChange={(enabled) => void put("/mixer/ducking", { enabled })} aria-label="Smart mixing" />
          </CardAction>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between text-sm">
            <span className="font-medium">Others are lowered to</span>
            <span className="tabular-nums text-muted-foreground">{m.ducking.level_db.toFixed(0)} dB</span>
          </div>
          <Slider
            value={[m.ducking.level_db]}
            min={-40}
            max={-3}
            step={1}
            disabled={!m.ducking.enabled}
            onValueChange={([v]) => setDuckLevel(v)}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Inputs</CardTitle>
          <CardDescription>
            {m.channels.length} device{m.channels.length === 1 ? "" : "s"} connected
          </CardDescription>
        </CardHeader>
        <CardContent>
          {m.channels.length === 0 ? (
            <Empty className="border border-dashed">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  <HugeiconsIcon icon={MusicNote01Icon} strokeWidth={2} />
                </EmptyMedia>
                <EmptyTitle>Nothing is streaming</EmptyTitle>
                <EmptyDescription>Connect a phone or a computer to “Charlotte” over Bluetooth and play something.</EmptyDescription>
              </EmptyHeader>
            </Empty>
          ) : (
            <ItemGroup>
              {m.channels.map((c, i) => (
                <div key={c.slot}>
                  {i > 0 && <ItemSeparator />}
                  <Item className="flex-wrap px-0">
                    <ItemMedia variant="icon">
                      <HugeiconsIcon icon={c.kind === "simulated" ? AudioWave01Icon : MusicNote01Icon} strokeWidth={2} />
                    </ItemMedia>
                    <ItemContent>
                      <ItemTitle>
                        {c.name}
                        {c.kind === "simulated" && <Badge variant="outline">test tone</Badge>}
                      </ItemTitle>
                      <ItemDescription className="flex flex-wrap gap-1.5">
                        {c.active ? <Badge variant="secondary">playing</Badge> : <Badge variant="outline">silent</Badge>}
                        {c.ducked && c.active && (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <Badge variant="outline" className="border-warning/50 text-warning">
                                lowered
                              </Badge>
                            </TooltipTrigger>
                            <TooltipContent>Another device is playing (smart mixing)</TooltipContent>
                          </Tooltip>
                        )}
                        {c.muted && <Badge variant="destructive">muted</Badge>}
                      </ItemDescription>
                    </ItemContent>
                    <ItemActions>
                      <span className="w-10 text-right text-xs tabular-nums text-muted-foreground">{Math.round(c.volume * 100)}%</span>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            size="icon"
                            variant={c.muted ? "destructive" : "ghost"}
                            aria-label={c.muted ? "Unmute" : "Mute"}
                            onClick={() => void put(`/mixer/channels/${c.slot}/mute`, { muted: !c.muted })}
                          >
                            <HugeiconsIcon icon={c.muted ? VolumeOffIcon : VolumeHighIcon} strokeWidth={2} />
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent>{c.muted ? "Unmute" : "Mute"}</TooltipContent>
                      </Tooltip>
                    </ItemActions>
                    <div className="basis-full space-y-3 pl-11">
                      <LevelMeter peak={c.peak * c.volume * (c.ducked ? c.duck_gain : 1)} dim={c.muted} />
                      <Slider value={[c.volume]} max={2} step={0.01} onValueChange={([v]) => setVolume(`/mixer/channels/${c.slot}/volume`, v)} />
                    </div>
                  </Item>
                </div>
              ))}
            </ItemGroup>
          )}
        </CardContent>
      </Card>
    </>
  );
}

function AudioSkeleton() {
  return (
    <div className="space-y-6">
      <Skeleton className="h-8 w-40" />
      <Skeleton className="h-48 w-full rounded-xl" />
      <Skeleton className="h-32 w-full rounded-xl" />
      <Skeleton className="h-40 w-full rounded-xl" />
    </div>
  );
}
