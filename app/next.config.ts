import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // STATIC_EXPORT=1 builds plain HTML files (used for the demo site); otherwise a normal Next.js server.
  ...(process.env.STATIC_EXPORT ? { output: "export" as const, trailingSlash: true } : {}),
};

export default nextConfig;
