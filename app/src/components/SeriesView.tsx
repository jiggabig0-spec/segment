"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { useCallback, useEffect, useMemo, useState } from "react";
import { formatAmount, formatBps } from "@/lib/amounts";
import { errorMessage } from "@/lib/errors";
import { fetchBalances, fetchPartials, fetchSeries, tradeUrl, type PartialInfo, type SeriesInfo } from "@/lib/segments";
import { useProgram } from "@/lib/useSegments";
import { MintRedeem } from "./MintRedeem";

export type Balances = Awaited<ReturnType<typeof fetchBalances>>;

export function SeriesView({ address }: { address: string }) {
  const program = useProgram();
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const key = useMemo(() => {
    try {
      return new PublicKey(address);
    } catch {
      return null;
    }
  }, [address]);

  const [series, setSeries] = useState<SeriesInfo | null>(null);
  const [partials, setPartials] = useState<PartialInfo[]>([]);
  const [balances, setBalances] = useState<Balances | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!key) return;
    try {
      const s = await fetchSeries(program, key);
      setSeries(s);
      setPartials(await fetchPartials(connection, s));
      setBalances(publicKey ? await fetchBalances(connection, s, publicKey) : null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [program, connection, key, publicKey]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  if (!key) return <p className="error">That isn&apos;t a valid series address.</p>;
  if (error) return <p className="error">Couldn&apos;t load this series: {error}</p>;
  if (!series) return <p className="muted">Loading…</p>;

  const d = series.decimals;
  return (
    <>
      <section className="series-head">
        <h1>Series {series.seriesId}</h1>
        <p className="muted">
          Underlying stock token <code>{series.underlyingMint.toBase58()}</code>
          {series.metadataUri && (
            <>
              {" · "}
              <a href={series.metadataUri} target="_blank" rel="noreferrer">
                Segment methodology
              </a>
            </>
          )}
        </p>
        <div className="stats">
          <Stat label="Sets outstanding" value={formatAmount(series.outstandingSets, d, 4)} />
          <Stat label="Mint fee" value={formatBps(series.mintFeeBps)} />
          <Stat label="Redeem fee" value={formatBps(series.redeemFeeBps)} />
        </div>
        {series.pendingFee && (
          <p className="notice">
            Fees change to {formatBps(series.pendingFee.mintFeeBps)} mint / {formatBps(series.pendingFee.redeemFeeBps)}{" "}
            redeem on {new Date(series.pendingFee.effectiveAt * 1000).toLocaleString()}.
          </p>
        )}
        {series.mintPaused && (
          <p className="notice">Minting is paused for this series. Redeeming still works as always.</p>
        )}
      </section>

      <div className="layout">
        <section className="panel">
          <h2>Segments</h2>
          <table className="table">
            <thead>
              <tr>
                <th>Token</th>
                <th className="num">Seed weight</th>
                {balances && <th className="num">You hold</th>}
                <th />
              </tr>
            </thead>
            <tbody>
              {partials.map((p) => (
                <tr key={p.mint.toBase58()}>
                  <td>
                    <strong>{p.symbol}</strong>
                    <div className="muted small">{p.name}</div>
                  </td>
                  <td className="num">{formatBps(p.weightBps)}</td>
                  {balances && <td className="num">{formatAmount(balances.partials[p.index], d, 4)}</td>}
                  <td className="num">
                    <a href={tradeUrl(p.mint)} target="_blank" rel="noreferrer" className="link-button">
                      Trade
                    </a>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">
            Seed weights are the starting split of one stock token across segments. Prices after launch are set by
            the market.
          </p>
        </section>

        <MintRedeem series={series} balances={balances} onDone={refresh} />
      </div>
    </>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat">
      <div className="muted small">{label}</div>
      <div className="stat-value">{value}</div>
    </div>
  );
}
