import type { FastifyInstance } from "fastify";
import { requireAdmin, requireUser } from "../auth.js";
import { audit } from "../db.js";
import type { AppContext } from "../index.js";
import { SettingsSchema, getSettings, updateSettings } from "../settings.js";

export async function settingsRoutes(app: FastifyInstance, ctx: AppContext) {
  app.get("/api/settings", { preHandler: requireUser }, async () => ({ settings: await getSettings(ctx.db) }));

  app.put("/api/settings", { preHandler: requireAdmin }, async (req, reply) => {
    const patch = SettingsSchema.partial().strict().safeParse(req.body);
    if (!patch.success) return reply.code(400).send({ error: patch.error.issues[0]?.message ?? "invalid settings" });
    const settings = await updateSettings(ctx.db, patch.data, req.user!.id);
    await audit(ctx.db, req.user!.id, "settings.updated", patch.data);

    let warning: string | undefined;
    if (patch.data["bluetooth.name"] !== undefined) {
      try {
        await ctx.audio.setBluetoothName(settings["bluetooth.name"]);
      } catch {
        warning = "saved, but the audio server is unreachable: the name will be applied when it is back";
      }
      ctx.reconciler.invalidate();
    }
    return { settings, warning };
  });
}
