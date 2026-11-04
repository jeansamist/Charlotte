import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import pg from "pg";

export type Db = pg.Pool;

export function createDb(url: string): Db {
  return new pg.Pool({ connectionString: url, max: 10 });
}

const MIGRATIONS_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../migrations");

/** Applies every `migrations/NNN_*.sql` not yet recorded, each in its own transaction. */
export async function migrate(db: Db, log: (msg: string) => void = console.log): Promise<void> {
  await db.query(`CREATE TABLE IF NOT EXISTS schema_migrations (
    name text PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now())`);
  const done = new Set(
    (await db.query<{ name: string }>("SELECT name FROM schema_migrations")).rows.map((r) => r.name),
  );
  const files = (await readdir(MIGRATIONS_DIR)).filter((f) => f.endsWith(".sql")).sort();
  for (const file of files) {
    if (done.has(file)) continue;
    const sql = await readFile(path.join(MIGRATIONS_DIR, file), "utf8");
    const client = await db.connect();
    try {
      await client.query("BEGIN");
      await client.query(sql);
      await client.query("INSERT INTO schema_migrations (name) VALUES ($1)", [file]);
      await client.query("COMMIT");
      log(`migration applied: ${file}`);
    } catch (e) {
      await client.query("ROLLBACK");
      throw new Error(`migration ${file} failed: ${(e as Error).message}`);
    } finally {
      client.release();
    }
  }
}

export async function audit(db: Db, userId: string | null, action: string, details: object = {}) {
  await db.query("INSERT INTO audit_log (user_id, action, details) VALUES ($1, $2, $3)", [
    userId,
    action,
    JSON.stringify(details),
  ]);
}
