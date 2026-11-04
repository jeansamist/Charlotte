import type { FastifyInstance, FastifyReply, FastifyRequest } from "fastify";
import { requireAdmin, requireUser } from "../auth.js";
import type { AppContext } from "../index.js";

/**
 * Authenticated proxy to the audio-server's local API. Only the routes below
 * are reachable; Bluetooth pairing management is for administrators.
 */
const ROUTES: { method: "GET" | "PUT" | "POST" | "DELETE"; path: string; admin?: boolean }[] = [
  { method: "GET", path: "/health" },
  { method: "GET", path: "/metrics" },
  { method: "GET", path: "/mixer" },
  { method: "PUT", path: "/mixer/channels/:slot/volume" },
  { method: "PUT", path: "/mixer/channels/:slot/mute" },
  { method: "PUT", path: "/mixer/master/volume" },
  { method: "PUT", path: "/mixer/master/mute" },
  { method: "PUT", path: "/mixer/ducking" },
  { method: "GET", path: "/outputs" },
  { method: "PUT", path: "/outputs/current" },
  { method: "GET", path: "/bluetooth" },
  { method: "POST", path: "/bluetooth/pairing", admin: true },
  { method: "DELETE", path: "/bluetooth/pairing", admin: true },
  { method: "DELETE", path: "/bluetooth/devices/:address", admin: true },
];

export async function audioRoutes(app: FastifyInstance, ctx: AppContext) {
  for (const r of ROUTES) {
    app.route({
      method: r.method,
      url: `/api/audio${r.path}`,
      preHandler: r.admin ? requireAdmin : requireUser,
      handler: async (req: FastifyRequest, reply: FastifyReply) => {
        const target = req.url.replace(/^\/api\/audio/, "/api").split("?")[0]!;
        try {
          const body = r.method === "PUT" || r.method === "POST" ? (req.body ?? {}) : undefined;
          const res = await ctx.audio.request(r.method, target, body);
          return reply.code(res.status).send(res.data);
        } catch (e) {
          req.log.warn({ err: e }, "audio-server unreachable");
          return reply.code(502).send({ error: "the audio server is not running" });
        }
      },
    });
  }
}
