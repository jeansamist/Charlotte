import type { FastifyBaseLogger } from "fastify";
import type { AudioClient } from "./audio-client.js";
import type { Db } from "./db.js";
import { getSettings } from "./settings.js";

/**
 * The database is the source of truth for settings. When the audio-server
 * (re)starts, push the settings it needs (Bluetooth name) to it again.
 */
export function startReconciler(db: Db, audio: AudioClient, log: FastifyBaseLogger, everyMs = 15_000) {
  let appliedFor: number | undefined;
  const tick = async () => {
    const health = await audio.health();
    if (!health) {
      if (appliedFor !== undefined) log.warn("audio-server unreachable");
      appliedFor = undefined;
      return;
    }
    if (health.started_at === appliedFor) return;
    try {
      const s = await getSettings(db);
      await audio.setBluetoothName(s["bluetooth.name"]);
      appliedFor = health.started_at;
      log.info({ bluetoothName: s["bluetooth.name"] }, "settings applied to audio-server");
    } catch (e) {
      log.warn({ err: e }, "cannot apply settings to audio-server");
    }
  };
  void tick();
  const timer = setInterval(() => void tick(), everyMs);
  return {
    stop: () => clearInterval(timer),
    /** Forces a re-apply on the next tick (after a settings change). */
    invalidate: () => {
      appliedFor = undefined;
      void tick();
    },
  };
}
