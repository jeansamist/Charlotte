"use client";
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode, SelectHTMLAttributes } from "react";

export function cx(...c: (string | false | null | undefined)[]) {
  return c.filter(Boolean).join(" ");
}

export function Card({ title, actions, children, className }: { title?: ReactNode; actions?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={cx("rounded-2xl border border-line bg-panel p-5 shadow-sm", className)}>
      {(title || actions) && (
        <header className="mb-4 flex items-center justify-between gap-3">
          {title && <h2 className="text-base font-semibold">{title}</h2>}
          {actions}
        </header>
      )}
      {children}
    </section>
  );
}

type Variant = "primary" | "ghost" | "danger";
export function Button({ variant = "primary", className, ...p }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  const v = {
    primary: "bg-accent text-white hover:opacity-90",
    ghost: "border border-line hover:bg-accent-soft",
    danger: "bg-danger text-white hover:opacity-90",
  }[variant];
  return (
    <button
      {...p}
      className={cx("inline-flex items-center justify-center gap-2 rounded-xl px-3.5 py-2 text-sm font-medium transition disabled:opacity-50", v, className)}
    />
  );
}

export function Input({ label, className, ...p }: InputHTMLAttributes<HTMLInputElement> & { label?: string }) {
  return (
    <label className="block text-sm">
      {label && <span className="mb-1 block text-muted">{label}</span>}
      <input
        {...p}
        className={cx("w-full rounded-xl border border-line bg-bg px-3 py-2 outline-none focus:border-accent", className)}
      />
    </label>
  );
}

export function Select({ label, children, ...p }: SelectHTMLAttributes<HTMLSelectElement> & { label?: string }) {
  return (
    <label className="block text-sm">
      {label && <span className="mb-1 block text-muted">{label}</span>}
      <select {...p} className="w-full rounded-xl border border-line bg-bg px-3 py-2 outline-none focus:border-accent">
        {children}
      </select>
    </label>
  );
}

export function Badge({ tone = "muted", children }: { tone?: "ok" | "warn" | "danger" | "muted" | "accent"; children: ReactNode }) {
  const t = {
    ok: "bg-ok/15 text-ok",
    warn: "bg-warn/15 text-warn",
    danger: "bg-danger/15 text-danger",
    muted: "bg-line text-muted",
    accent: "bg-accent-soft text-accent",
  }[tone];
  return <span className={cx("inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium", t)}>{children}</span>;
}

export function Alert({ tone = "danger", children }: { tone?: "danger" | "warn" | "ok"; children: ReactNode }) {
  const t = { danger: "border-danger/40 bg-danger/10", warn: "border-warn/40 bg-warn/10", ok: "border-ok/40 bg-ok/10" }[tone];
  return <div className={cx("rounded-xl border px-3 py-2 text-sm", t)}>{children}</div>;
}

/** Level meter from a linear peak (0..1), shown on a -60..0 dB scale. */
export function Meter({ peak, dim }: { peak: number; dim?: boolean }) {
  const db = peak > 1e-6 ? 20 * Math.log10(peak) : -60;
  const pct = Math.max(0, Math.min(100, ((db + 60) / 60) * 100));
  const color = db > -3 ? "bg-danger" : db > -12 ? "bg-warn" : "bg-ok";
  return (
    <div className="h-2 w-full overflow-hidden rounded-full bg-line">
      <div className={cx("h-full transition-[width] duration-100", color, dim && "opacity-40")} style={{ width: `${pct}%` }} />
    </div>
  );
}

export function Slider({ value, max = 1, onChange, disabled }: { value: number; max?: number; onChange: (v: number) => void; disabled?: boolean }) {
  return (
    <input
      type="range"
      min={0}
      max={max}
      step={0.01}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(Number(e.target.value))}
      className="w-full"
    />
  );
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label?: string }) {
  return (
    <button type="button" onClick={() => onChange(!checked)} className="inline-flex items-center gap-2 text-sm" aria-pressed={checked}>
      <span className={cx("relative h-5 w-9 rounded-full transition", checked ? "bg-accent" : "bg-line")}>
        <span className={cx("absolute top-0.5 h-4 w-4 rounded-full bg-white shadow transition-all", checked ? "left-4.5" : "left-0.5")} />
      </span>
      {label}
    </button>
  );
}

export function Spinner() {
  return <div className="h-5 w-5 animate-spin rounded-full border-2 border-line border-t-accent" />;
}
