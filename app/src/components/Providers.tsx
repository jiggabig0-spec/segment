"use client";

import { ConnectionProvider, WalletProvider } from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import type { ReactNode } from "react";
import { RPC_URL } from "@/lib/config";
import "@solana/wallet-adapter-react-ui/styles.css";

// Wallets that implement the Wallet Standard (Phantom, Solflare, Backpack, ...) are detected
// automatically, so no wallet list is needed.
export function Providers({ children }: { children: ReactNode }) {
  return (
    <ConnectionProvider endpoint={RPC_URL}>
      <WalletProvider wallets={[]} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
