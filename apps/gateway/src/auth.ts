import { createHash, randomBytes } from "node:crypto";
import { hash, verify } from "@node-rs/argon2";
import type { FastifyReply, FastifyRequest } from "fastify";
import type { Db } from "./db.js";

export const SESSION_COOKIE = "charlotte_session";

export type Role = "admin" | "member";
export interface SessionUser {
  id: string;
  username: string;
  displayName: string;
  role: Role;
}

declare module "fastify" {
  interface FastifyRequest {
    user?: SessionUser;
  }
}

// Argon2id with OWASP-recommended parameters (19 MiB, 2 iterations).
const ARGON = { memoryCost: 19456, timeCost: 2, parallelism: 1 } as const;

export const hashPassword = (password: string) => hash(password, ARGON);

// Used to spend the same time when the username does not exist.
let dummyHash: Promise<string> | undefined;

export async function checkPassword(storedHash: string | undefined, password: string): Promise<boolean> {
  if (!storedHash) {
    dummyHash ??= hashPassword(randomBytes(16).toString("hex"));
    await verify(await dummyHash, password).catch(() => false);
    return false;
  }
  return verify(storedHash, password).catch(() => false);
}

export const PASSWORD_RULE = { min: 10, max: 128 } as const;

export const sha256 = (s: string) => createHash("sha256").update(s).digest("hex");

export async function createSession(
  db: Db,
  userId: string,
  days: number,
  meta: { userAgent: string; ip: string },
): Promise<{ token: string; expiresAt: Date }> {
  const token = randomBytes(32).toString("base64url");
  const expiresAt = new Date(Date.now() + days * 86_400_000);
  await db.query(
    "INSERT INTO sessions (token_hash, user_id, expires_at, user_agent, ip) VALUES ($1, $2, $3, $4, $5)",
    [sha256(token), userId, expiresAt, meta.userAgent.slice(0, 300), meta.ip],
  );
  return { token, expiresAt };
}

export async function deleteSession(db: Db, token: string) {
  await db.query("DELETE FROM sessions WHERE token_hash = $1", [sha256(token)]);
}

/**
 * Resolves the cookie to a user. The expiry slides, but the session row is
 * rewritten at most every 5 minutes (the GUI polls several times per second).
 */
export async function userFromToken(db: Db, token: string, days: number): Promise<SessionUser | undefined> {
  const { rows } = await db.query<{ id: string; username: string; display_name: string; role: Role }>(
    `WITH s AS (
       SELECT u.id, u.username, u.display_name, u.role
         FROM sessions s JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = $1 AND s.expires_at > now()
     ), touch AS (
       UPDATE sessions SET last_seen_at = now(), expires_at = now() + make_interval(days => $2)
        WHERE token_hash = $1 AND last_seen_at < now() - interval '5 minutes' AND EXISTS (SELECT 1 FROM s)
     )
     SELECT id, username, display_name, role FROM s`,
    [sha256(token), days],
  );
  const r = rows[0];
  return r && { id: r.id, username: r.username, displayName: r.display_name, role: r.role };
}

export function requireUser(req: FastifyRequest, reply: FastifyReply, done: () => void) {
  if (!req.user) {
    reply.code(401).send({ error: "not signed in" });
    return;
  }
  done();
}

export function requireAdmin(req: FastifyRequest, reply: FastifyReply, done: () => void) {
  if (!req.user) {
    reply.code(401).send({ error: "not signed in" });
    return;
  }
  if (req.user.role !== "admin") {
    reply.code(403).send({ error: "administrator only" });
    return;
  }
  done();
}

/** Simple in-memory limiter for login attempts (per IP). */
export class AttemptLimiter {
  private hits = new Map<string, { count: number; resetAt: number }>();
  constructor(private max = 10, private windowMs = 5 * 60_000) {}

  /** Returns seconds to wait, or 0 if allowed. */
  check(key: string): number {
    const now = Date.now();
    const h = this.hits.get(key);
    if (!h || h.resetAt < now) return 0;
    return h.count >= this.max ? Math.ceil((h.resetAt - now) / 1000) : 0;
  }
  fail(key: string) {
    const now = Date.now();
    const h = this.hits.get(key);
    if (!h || h.resetAt < now) this.hits.set(key, { count: 1, resetAt: now + this.windowMs });
    else h.count++;
  }
  reset(key: string) {
    this.hits.delete(key);
  }
}
