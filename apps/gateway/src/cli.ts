/**
 * Admin CLI (used by the installer):
 *   node dist/cli.js migrate
 *   node dist/cli.js create-admin <username>      (password from CHARLOTTE_ADMIN_PASSWORD or prompt)
 *   node dist/cli.js reset-password <username>    (same)
 *   node dist/cli.js set <key> <value>            (e.g. set bluetooth.name "Living room")
 *   node dist/cli.js has-users                    (exit 0 if at least one user exists)
 */
import { createInterface } from "node:readline/promises";
import { stdin, stdout } from "node:process";
import { PASSWORD_RULE, hashPassword } from "./auth.js";
import { loadConfig } from "./config.js";
import { createDb, migrate } from "./db.js";
import { SettingsSchema } from "./settings.js";

async function readPassword(): Promise<string> {
  const fromEnv = process.env.CHARLOTTE_ADMIN_PASSWORD;
  if (fromEnv) return fromEnv;
  if (!stdin.isTTY) throw new Error("no TTY: set CHARLOTTE_ADMIN_PASSWORD");
  const ask = async (q: string) => {
    const rl = createInterface({ input: stdin, output: stdout, terminal: true });
    // Hide typed characters.
    (rl as unknown as { _writeToOutput: (s: string) => void })._writeToOutput = (s: string) => {
      if (s.includes(q)) stdout.write(q);
    };
    const v = await rl.question(q);
    rl.close();
    stdout.write("\n");
    return v;
  };
  for (;;) {
    const a = await ask(`Password (min ${PASSWORD_RULE.min} characters): `);
    if (a.length < PASSWORD_RULE.min) {
      console.log("Too short.");
      continue;
    }
    if (a !== (await ask("Repeat password: "))) {
      console.log("Passwords do not match.");
      continue;
    }
    return a;
  }
}

async function main() {
  const [cmd, ...args] = process.argv.slice(2);
  const db = createDb(loadConfig().DATABASE_URL);
  try {
    await migrate(db);
    switch (cmd) {
      case "migrate":
        break;
      case "has-users": {
        const { rows } = await db.query("SELECT 1 FROM users LIMIT 1");
        process.exitCode = rows.length ? 0 : 1;
        break;
      }
      case "create-admin":
      case "reset-password": {
        const username = (args[0] ?? "").toLowerCase();
        if (!/^[a-z0-9][a-z0-9_.-]{2,31}$/.test(username)) throw new Error("invalid username");
        const password = await readPassword();
        if (password.length < PASSWORD_RULE.min) throw new Error("password too short");
        const h = await hashPassword(password);
        if (cmd === "create-admin") {
          await db.query(
            `INSERT INTO users (username, display_name, password_hash, role) VALUES ($1, $1, $2, 'admin')
             ON CONFLICT (username) DO UPDATE SET password_hash = EXCLUDED.password_hash, role = 'admin', updated_at = now()`,
            [username, h],
          );
        } else {
          const r = await db.query("UPDATE users SET password_hash = $2, updated_at = now() WHERE username = $1", [
            username,
            h,
          ]);
          if (!r.rowCount) throw new Error(`no user ${username}`);
          await db.query("DELETE FROM sessions WHERE user_id = (SELECT id FROM users WHERE username = $1)", [username]);
        }
        console.log(`${cmd === "create-admin" ? "administrator" : "password"} ${username} ok`);
        break;
      }
      case "set": {
        const [key, value] = args;
        const parsed = SettingsSchema.partial().strict().parse({ [key ?? ""]: value });
        for (const [k, v] of Object.entries(parsed)) {
          await db.query(
            `INSERT INTO settings (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()`,
            [k, JSON.stringify(v)],
          );
          console.log(`${k} = ${JSON.stringify(v)}`);
        }
        break;
      }
      default:
        console.log("usage: cli.js migrate | has-users | create-admin <user> | reset-password <user> | set <key> <value>");
        process.exitCode = 2;
    }
  } finally {
    await db.end();
  }
}

main().catch((e) => {
  console.error(`error: ${(e as Error).message}`);
  process.exit(1);
});
