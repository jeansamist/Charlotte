import type { FastifyInstance } from "fastify";
import { z } from "zod";
import {
  AttemptLimiter,
  PASSWORD_RULE,
  SESSION_COOKIE,
  checkPassword,
  createSession,
  deleteSession,
  hashPassword,
  requireUser,
  sha256,
} from "../auth.js";
import { audit } from "../db.js";
import type { AppContext } from "../index.js";

const LoginBody = z.object({
  username: z.string().trim().toLowerCase().min(1).max(64),
  password: z.string().min(1).max(PASSWORD_RULE.max),
});

const PasswordBody = z.object({
  current: z.string().min(1).max(PASSWORD_RULE.max),
  next: z.string().min(PASSWORD_RULE.min).max(PASSWORD_RULE.max),
});

export async function authRoutes(app: FastifyInstance, ctx: AppContext) {
  const limiter = new AttemptLimiter();

  app.post("/api/auth/login", async (req, reply) => {
    const wait = limiter.check(req.ip);
    if (wait > 0) {
      reply.header("retry-after", wait);
      return reply.code(429).send({ error: `too many attempts, retry in ${wait}s` });
    }
    const body = LoginBody.safeParse(req.body);
    if (!body.success) return reply.code(400).send({ error: "username and password are required" });

    const { rows } = await ctx.db.query<{ id: string; password_hash: string }>(
      "SELECT id, password_hash FROM users WHERE username = $1",
      [body.data.username],
    );
    const ok = await checkPassword(rows[0]?.password_hash, body.data.password);
    if (!ok || !rows[0]) {
      limiter.fail(req.ip);
      await audit(ctx.db, null, "login.failed", { username: body.data.username, ip: req.ip });
      return reply.code(401).send({ error: "wrong username or password" });
    }
    limiter.reset(req.ip);
    const { token, expiresAt } = await createSession(ctx.db, rows[0].id, ctx.config.SESSION_DAYS, {
      userAgent: req.headers["user-agent"] ?? "",
      ip: req.ip,
    });
    await audit(ctx.db, rows[0].id, "login", { ip: req.ip });
    reply.setCookie(SESSION_COOKIE, token, {
      path: "/",
      httpOnly: true,
      sameSite: "lax",
      secure: ctx.config.COOKIE_SECURE,
      expires: expiresAt,
    });
    const me = await ctx.db.query("SELECT id, username, display_name AS \"displayName\", role FROM users WHERE id = $1", [
      rows[0].id,
    ]);
    return { user: me.rows[0] };
  });

  app.post("/api/auth/logout", async (req, reply) => {
    const token = req.cookies[SESSION_COOKIE];
    if (token) await deleteSession(ctx.db, token);
    reply.clearCookie(SESSION_COOKIE, { path: "/" });
    return { ok: true };
  });

  app.get("/api/auth/me", { preHandler: requireUser }, async (req) => ({ user: req.user }));

  app.post("/api/auth/password", { preHandler: requireUser }, async (req, reply) => {
    const body = PasswordBody.safeParse(req.body);
    if (!body.success)
      return reply.code(400).send({ error: `new password must be ${PASSWORD_RULE.min}-${PASSWORD_RULE.max} characters` });
    const user = req.user!;
    const { rows } = await ctx.db.query<{ password_hash: string }>("SELECT password_hash FROM users WHERE id = $1", [
      user.id,
    ]);
    if (!(await checkPassword(rows[0]?.password_hash, body.data.current)))
      return reply.code(400).send({ error: "current password is wrong" });
    await ctx.db.query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2", [
      await hashPassword(body.data.next),
      user.id,
    ]);
    // Sign out every other session.
    const token = req.cookies[SESSION_COOKIE]!;
    await ctx.db.query("DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2", [user.id, sha256(token)]);
    await audit(ctx.db, user.id, "password.changed");
    return { ok: true };
  });
}
