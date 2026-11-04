import cookie from "@fastify/cookie";
import Fastify from "fastify";
import { AudioClient } from "./audio-client.js";
import { SESSION_COOKIE, userFromToken } from "./auth.js";
import { loadConfig, type Config } from "./config.js";
import { createDb, migrate, type Db } from "./db.js";
import { startReconciler } from "./reconciler.js";
import { audioRoutes } from "./routes/audio.js";
import { authRoutes } from "./routes/auth.js";
import { settingsRoutes } from "./routes/settings.js";
import { userRoutes } from "./routes/users.js";

export interface AppContext {
  config: Config;
  db: Db;
  audio: AudioClient;
  reconciler: ReturnType<typeof startReconciler>;
}

async function main() {
  const config = loadConfig();
  const app = Fastify({
    logger: { level: config.LOG_LEVEL },
    // The web app (Next.js) proxies to us from the same machine.
    trustProxy: "127.0.0.1",
    bodyLimit: 64 * 1024,
  });
  const db = createDb(config.DATABASE_URL);
  await migrate(db, (m) => app.log.info(m));

  const audio = new AudioClient(config.AUDIO_SERVER_URL);
  const ctx: AppContext = { config, db, audio, reconciler: startReconciler(db, audio, app.log) };

  await app.register(cookie);
  // Resolve the session cookie on every request.
  app.addHook("onRequest", async (req) => {
    const token = req.cookies[SESSION_COOKIE];
    if (token) req.user = await userFromToken(db, token, config.SESSION_DAYS);
  });

  app.get("/api/health", async () => {
    await db.query("SELECT 1");
    return { status: "ok", audioServer: (await audio.health()) ? "up" : "down" };
  });
  await authRoutes(app, ctx);
  await userRoutes(app, ctx);
  await settingsRoutes(app, ctx);
  await audioRoutes(app, ctx);

  // Housekeeping: drop expired sessions hourly.
  setInterval(() => void db.query("DELETE FROM sessions WHERE expires_at < now()"), 3_600_000).unref();

  const close = async () => {
    ctx.reconciler.stop();
    await app.close();
    await db.end();
    process.exit(0);
  };
  process.on("SIGTERM", close);
  process.on("SIGINT", close);

  await app.listen({ host: config.HOST, port: config.PORT });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
