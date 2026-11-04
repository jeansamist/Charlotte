import { z } from "zod";

const Env = z.object({
  DATABASE_URL: z.string().min(1, "DATABASE_URL is required"),
  HOST: z.string().default("127.0.0.1"),
  PORT: z.coerce.number().int().min(1).max(65535).default(4000),
  AUDIO_SERVER_URL: z.string().url().default("http://127.0.0.1:8080"),
  COOKIE_SECURE: z
    .enum(["true", "false"])
    .default("false")
    .transform((v) => v === "true"),
  SESSION_DAYS: z.coerce.number().int().min(1).max(365).default(30),
  LOG_LEVEL: z.enum(["trace", "debug", "info", "warn", "error"]).default("info"),
});

export type Config = z.infer<typeof Env>;

export function loadConfig(env: NodeJS.ProcessEnv = process.env): Config {
  const parsed = Env.safeParse(env);
  if (!parsed.success) {
    const issues = parsed.error.issues.map((i) => `${i.path.join(".")}: ${i.message}`).join("; ");
    throw new Error(`invalid gateway configuration: ${issues}`);
  }
  return parsed.data;
}
