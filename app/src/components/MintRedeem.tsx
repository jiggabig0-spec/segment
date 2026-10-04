"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { useState } from "react";
import { formatAmount, mintPreview, parseAmount, redeemPreview } from "@/lib/amounts";
import { RPC_URL } from "@/lib/config";
import { errorMessage } from "@/lib/errors";
import {
  explorerUrl,
  mintSetInstructions,
  redeemSetInstructions,
  setupInstructions,
  tradeUrl,
  type SeriesInfo,
} from "@/lib/segments";
import { useProgram, useSend } from "@/lib/useSegments";
import type { Balances } from "./SeriesView";

type Mode = "mint" | "redeem";

export function MintRedeem({
  series,
  balances,
  onDone,
}: {
  series: SeriesInfo;
  balances: Balances | null;
  onDone: () => void;
}) {
  const program = useProgram();
  const send = useSend();
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const [mode, setMode] = useState<Mode>("mint");
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ kind: "ok" | "error"; text: string; sig?: string } | null>(null);

  const d = series.decimals;
  const amount = parseAmount(input, d);
  const available = balances ? (mode === "mint" ? balances.underlying : balances.completeSets) : null;
  const tooMuch = amount !== null && available !== null && amount > available;
  const preview =
    amount && amount > BigInt(0)
      ? mode === "mint"
        ? { pay: amount, get: mintPreview(amount, series.mintFeeBps).sets, fee: mintPreview(amount, series.mintFeeBps).fee }
        : { pay: amount, get: redeemPreview(amount, series.redeemFeeBps).withdrawn, fee: redeemPreview(amount, series.redeemFeeBps).fee }
      : null;
  const blocked = mode === "mint" && series.mintPaused;
  const canSubmit = !!publicKey && !!preview && preview.get > BigInt(0) && !tooMuch && !busy && !blocked;

  async function submit() {
    if (!publicKey || !amount) return;
    setBusy(true);
    setStatus(null);
    try {
      if (mode === "mint") {
        // First-time users need a token account per segment; that's a separate, one-off transaction.
        const setup = await setupInstructions(connection, series, publicKey);
        if (setup.length) await send(setup);
      }
      const ixs =
        mode === "mint"
          ? await mintSetInstructions(program, series, publicKey, amount)
          : await redeemSetInstructions(program, series, publicKey, amount);
      const sig = await send(ixs);
      setStatus({ kind: "ok", text: mode === "mint" ? "Minted." : "Redeemed.", sig });
      setInput("");
      onDone();
    } catch (e) {
      setStatus({ kind: "error", text: errorMessage(e) });
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="panel">
      <div className="tabs">
        {(["mint", "redeem"] as const).map((m) => (
          <button
            key={m}
            className={m === mode ? "tab active" : "tab"}
            onClick={() => {
              setMode(m);
              setStatus(null);
            }}
          >
            {m === "mint" ? "Mint a set" : "Redeem a set"}
          </button>
        ))}
      </div>
      <p className="muted small">
        {mode === "mint"
          ? "Deposit the stock token and receive the same amount of every segment token."
          : "Return the same amount of every segment token and get the stock token back."}
      </p>

      <label className="field">
        <span>{mode === "mint" ? "Stock tokens to deposit" : "Sets to redeem"}</span>
        <input
          inputMode="decimal"
          placeholder="0.0"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          disabled={busy}
        />
      </label>
      {available !== null && (
        <div className="row small">
          <span className="muted">
            Available: {formatAmount(available, d)}
            {mode === "redeem" && " complete sets"}
          </span>
          <button className="link" onClick={() => setInput(formatAmount(available, d).replace(/,/g, ""))}>
            Max
          </button>
        </div>
      )}
      {input && amount === null && <p className="error small">Enter a number with up to {d} decimals.</p>}
      {tooMuch && <p className="error small">That&apos;s more than you have.</p>}
      {blocked && <p className="error small">Minting is paused for this series.</p>}

      {preview && (
        <dl className="preview">
          <dt>Fee</dt>
          <dd>{formatAmount(preview.fee, d)}</dd>
          <dt>{mode === "mint" ? "You get, of each segment" : "You get, in stock tokens"}</dt>
          <dd>{formatAmount(preview.get, d)}</dd>
        </dl>
      )}

      {publicKey ? (
        <button className="primary" disabled={!canSubmit} onClick={submit}>
          {busy ? "Confirm in your wallet…" : mode === "mint" ? "Mint" : "Redeem"}
        </button>
      ) : (
        <p className="muted">Connect a wallet to mint or redeem.</p>
      )}

      {status && (
        <p className={status.kind === "ok" ? "ok" : "error"}>
          {status.text}{" "}
          {status.sig && (
            <a href={explorerUrl(status.sig, RPC_URL)} target="_blank" rel="noreferrer">
              View transaction
            </a>
          )}
        </p>
      )}

      {mode === "mint" && (
        <p className="muted small">
          Don&apos;t have the stock token?{" "}
          <a href={tradeUrl(series.underlyingMint)} target="_blank" rel="noreferrer">
            Buy it on Jupiter
          </a>
          .
        </p>
      )}
    </section>
  );
}
