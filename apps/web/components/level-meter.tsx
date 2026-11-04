import { cn } from "@/lib/utils";

/** Level meter from a linear peak (0..1), on a −60..0 dB scale. */
export function LevelMeter({ peak, dim, className }: { peak: number; dim?: boolean; className?: string }) {
  const db = peak > 1e-6 ? 20 * Math.log10(peak) : -60;
  const pct = Math.max(0, Math.min(100, ((db + 60) / 60) * 100));
  const tone = db > -3 ? "bg-destructive" : db > -12 ? "bg-warning" : "bg-success";
  return (
    <div
      role="meter"
      aria-valuemin={-60}
      aria-valuemax={0}
      aria-valuenow={Math.round(db)}
      aria-label="Level"
      className={cn("h-1.5 w-full overflow-hidden rounded-full bg-muted", className)}
    >
      <div className={cn("h-full rounded-full transition-[width] duration-100", tone, dim && "opacity-40")} style={{ width: `${pct}%` }} />
    </div>
  );
}
