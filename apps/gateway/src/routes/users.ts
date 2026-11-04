import type { FastifyInstance } from "fastify";
import { z } from "zod";
import { PASSWORD_RULE, hashPassword, requireAdmin } from "../auth.js";
import { audit } from "../db.js";
import type { AppContext } from "../index.js";

const Username = z
  .string()
  .trim()
  .toLowerCase()
  .regex(/^[a-z0-9][a-z0-9_.-]{2,31}$/, "3-32 characters: letters, digits, . _ -");
const Password = z.string().min(PASSWORD_RULE.min).max(PASSWORD_RULE.max);
const Role = z.enum(["admin", "member"]);

const CreateBody = z.object({
  username: Username,
  displayName: z.string().trim().max(60).default(""),
  password: Password,
  role: Role.default("member"),
});
const UpdateBody = z.object({
  displayName: z.string().trim().max(60).optional(),
  role: Role.optional(),
  password: Password.optional(),
});

const SELECT = `SELECT id, username, display_name AS "displayName", role, created_at AS "createdAt" FROM users`;

export async function userRoutes(app: FastifyInstance, ctx: AppContext) {
  app.get("/api/users", { preHandler: requireAdmin }, async () => {
    const { rows } = await ctx.db.query(`${SELECT} ORDER BY created_at`);
    return { users: rows };
  });

  app.post("/api/users", { preHandler: requireAdmin }, async (req, reply) => {
    const body = CreateBody.safeParse(req.body);
    if (!body.success) return reply.code(400).send({ error: body.error.issues[0]?.message ?? "invalid input" });
    const b = body.data;
    try {
      const { rows } = await ctx.db.query(
        `INSERT INTO users (username, display_name, password_hash, role) VALUES ($1, $2, $3, $4)
         RETURNING id, username, display_name AS "displayName", role, created_at AS "createdAt"`,
        [b.username, b.displayName, await hashPassword(b.password), b.role],
      );
      await audit(ctx.db, req.user!.id, "user.created", { username: b.username, role: b.role });
      return reply.code(201).send({ user: rows[0] });
    } catch (e) {
      if ((e as { code?: string }).code === "23505") return reply.code(409).send({ error: "username already exists" });
      throw e;
    }
  });

  app.patch<{ Params: { id: string } }>("/api/users/:id", { preHandler: requireAdmin }, async (req, reply) => {
    const body = UpdateBody.safeParse(req.body);
    if (!body.success) return reply.code(400).send({ error: body.error.issues[0]?.message ?? "invalid input" });
    const { id } = req.params;
    if (body.data.role === "member" && id === req.user!.id)
      return reply.code(400).send({ error: "you cannot remove your own administrator role" });
    const b = body.data;
    const { rows } = await ctx.db.query(
      `UPDATE users SET
         display_name = COALESCE($2, display_name),
         role = COALESCE($3, role),
         password_hash = COALESCE($4, password_hash),
         updated_at = now()
       WHERE id = $1
       RETURNING id, username, display_name AS "displayName", role, created_at AS "createdAt"`,
      [id, b.displayName ?? null, b.role ?? null, b.password ? await hashPassword(b.password) : null],
    );
    if (!rows[0]) return reply.code(404).send({ error: "user not found" });
    if (b.password) await ctx.db.query("DELETE FROM sessions WHERE user_id = $1", [id]);
    await audit(ctx.db, req.user!.id, "user.updated", { id, fields: Object.keys(b) });
    return { user: rows[0] };
  });

  app.delete<{ Params: { id: string } }>("/api/users/:id", { preHandler: requireAdmin }, async (req, reply) => {
    if (req.params.id === req.user!.id) return reply.code(400).send({ error: "you cannot delete yourself" });
    const { rowCount } = await ctx.db.query("DELETE FROM users WHERE id = $1", [req.params.id]);
    if (!rowCount) return reply.code(404).send({ error: "user not found" });
    await audit(ctx.db, req.user!.id, "user.deleted", { id: req.params.id });
    return { ok: true };
  });
}
