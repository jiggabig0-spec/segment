"use client";

import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { useCallback, useEffect, useMemo, useState, type CSSProperties, type ReactNode } from "react";
import { formatBps } from "@/lib/amounts";
import { DEMO, DEMO_PARTIALS, DEMO_SERIES, demoBalances } from "@/lib/demo";
import { errorMessage } from "@/lib/errors";
import { fetchBalances, fetchPartials, fetchSeries, tradeUrl, type PartialInfo, type SeriesInfo } from "@/lib/segments";
import { useProgram } from "@/lib/useSegments";
import { AnimatedNumber } from "./AnimatedNumber";
import { MintRedeem } from "./MintRedeem";

const delay = (d: number) => ({ "--d": d }) as CSSProperties;

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
    if (DEMO) {
      setSeries({ ...DEMO_SERIES });
      setPartials(DEMO_PARTIALS);
      setBalances(demoBalances());
      return;
    }
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
        <h1 className="reveal">Series {series.seriesId}</h1>
        <p className="muted reveal" style={delay(1)}>
          Underlying stock token {DEMO ? <strong>GOOGLx</strong> : <code>{series.underlyingMint.toBase58()}</code>}
          {series.metadataUri && (
            <>
              {" · "}
              <a href={series.metadataUri} target="_blank" rel="noreferrer">
                Segment methodology
              </a>
            </>
          )}
        </p>
        <div className="stats reveal" style={delay(2)}>
          <Stat label="Sets outstanding" value={<AnimatedNumber value={series.outstandingSets} decimals={d} maxFraction={4} />} />
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
        <section className="panel reveal" style={delay(3)}>
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
                <tr key={p.mint.toBase58()} className="reveal" style={delay(4 + p.index * 0.5)}>
                  <td>
                    <strong>{p.symbol}</strong>
                    <div className="muted small">{p.name}</div>
                  </td>
                  <td className="num">
                    <span className="weight">
                      <span className="weight-bar" style={{ "--w": p.weightBps / 10_000, ...delay(5 + p.index * 0.5) } as CSSProperties} />
                      {formatBps(p.weightBps)}
                    </span>
                  </td>
                  {balances && (
                    <td className="num">
                      <AnimatedNumber value={balances.partials[p.index]} decimals={d} maxFraction={4} />
                    </td>
                  )}
                  <td className="num">
                    {DEMO ? (
                      <span className="link-button disabled" title="Trading opens once pools exist">
                        Trade
                      </span>
                    ) : (
                      <a href={tradeUrl(p.mint)} target="_blank" rel="noreferrer" className="link-button">
                        Trade
                      </a>
                    )}
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

function Stat({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div className="stat">
      <div className="muted small">{label}</div>
      <div className="stat-value">{value}</div>
    </div>
  );
}
