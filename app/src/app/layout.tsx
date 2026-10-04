import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { Providers } from "@/components/Providers";
import { WalletButton } from "@/components/WalletButton";
import { NETWORK_LABEL } from "@/lib/config";
import { DEMO } from "@/lib/demo";
import "./globals.css";

export const metadata: Metadata = {
  title: "Segments",
  description: "Own the part of a company you believe in.",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <Providers>
          <header className="header">
            <Link href="/" className="brand">
              Segments
            </Link>
            <span className="badge">{DEMO ? "Demo" : NETWORK_LABEL}</span>
            <div className="spacer" />
            <WalletButton />
          </header>
          {DEMO && (
            <div className="demo-banner">
              Demo: built-in Alphabet data and a pretend wallet holding 10 GOOGLx. Nothing touches the blockchain.
            </div>
          )}
          <main className="main">{children}</main>
          <footer className="footer">
            Segment tokens are experimental and unaudited. Not available to US persons or in restricted
            jurisdictions. Nothing here is investment advice.
          </footer>
        </Providers>
      </body>
    </html>
  );
}
