"use client";

import dynamic from "next/dynamic";
import { DEMO } from "@/lib/demo";

// The wallet button reads browser-only state, so render it on the client only.
const WalletMultiButton = dynamic(
  async () => (await import("@solana/wallet-adapter-react-ui")).WalletMultiButton,
  { ssr: false },
);

export function WalletButton() {
  if (DEMO) return <span className="demo-wallet">Demo wallet</span>;
  return <WalletMultiButton />;
}
