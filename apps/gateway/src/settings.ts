import { z } from "zod";
import type { Db } from "./db.js";

/** Every setting the GUI can edit, with its validation. */
export const SettingsSchema = z.object({
  "assistant.name": z.string().trim().min(1).max(40),
  "assistant.language": z.enum(["en", "fr"]),
  "bluetooth.name": z.string().trim().min(1).max(60),
});
export type Settings = z.infer<typeof SettingsSchema>;
export type SettingKey = keyof Settings;

export async function getSettings(db: Db): Promise<Settings> {
  const { rows } = await db.query<{ key: string; value: unknown }>("SELECT key, value FROM settings");
  const obj = Object.fromEntries(rows.map((r) => [r.key, r.value]));
  return SettingsSchema.parse({
    "assistant.name": "Charlotte",
    "assistant.language": "en",
    "bluetooth.name": "Charlotte",
    ...obj,
  });
}

export async function updateSettings(db: Db, patch: Partial<Settings>, userId: string): Promise<Settings> {
  for (const [key, value] of Object.entries(patch)) {
    await db.query(
      `INSERT INTO settings (key, value, updated_at, updated_by) VALUES ($1, $2, now(), $3)
       ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now(), updated_by = EXCLUDED.updated_by`,
      [key, JSON.stringify(value), userId],
    );
  }
  return getSettings(db);
}
