import type { NextConfig } from "next";

// The browser only talks to this app; /api/* is forwarded to the gateway on the
// same machine, so the session cookie stays first-party.
const gateway = process.env.GATEWAY_URL ?? "http://127.0.0.1:4000";

const config: NextConfig = {
  output: "standalone",
  poweredByHeader: false,
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${gateway}/api/:path*` }];
  },
};

export default config;
