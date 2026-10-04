"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { useConnection } from "@solana/wallet-adapter-react";
import { formatAmount } from "@/lib/amounts";
import { DEMO, DEMO_PARTIALS, DEMO_SERIES } from "@/lib/demo";
import { errorMessage } from "@/lib/errors";
import { fetchAllSeries, fetchPartials, type PartialInfo, type SeriesInfo } from "@/lib/segments";
import { useProgram } from "@/lib/useSegments";

type Row = { series: SeriesInfo; partials: PartialInfo[] };

export default function Home() {
  const program = useProgram();
  const { connection } = useConnection();
  const [rows, setRows] = useState<Row[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    if (DEMO) {
      setRows([{ series: DEMO_SERIES, partials: DEMO_PARTIALS }]);
      return;
    }
    (async () => {
      try {
        const all = (await fetchAllSeries(program)).filter((s) => s.finalized);
        const withPartials = await Promise.all(
          all.map(async (series) => ({ series, partials: await fetchPartials(connection, series) })),
        );
        if (live) setRows(withPartials);
      } catch (e) {
        if (live) setError(errorMessage(e));
      }
    })();
    return () => {
      live = false;
    };
  }, [program, connection]);

  return (
    <>
      <section className="hero">
        <h1>Buy the part of a company you believe in.</h1>
        <p>
          Deposit a tokenized stock and get one token for each of its business segments. Keep the segments you
          like, sell the rest, or put a full set back together to get the stock back at any time.
        </p>
      </section>
      {error && <p className="error">Couldn&apos;t load series: {error}</p>}
      {!rows && !error && <p className="muted">Loading series…</p>}
      {rows && rows.length === 0 && <p className="muted">No series are live on this network yet.</p>}
      <div className="grid">
        {rows?.map(({ series, partials }) => (
          <Link key={series.address.toBase58()} href={`/series/${series.address.toBase58()}`} className="card">
            <h2>Series {series.seriesId}</h2>
            <div className="chips">
              {partials.map((p) => (
                <span key={p.symbol} className="chip">
                  {p.symbol}
                </span>
              ))}
            </div>
            <p className="muted">
              {formatAmount(series.outstandingSets, series.decimals, 2)} sets outstanding
              {series.mintPaused && " · minting paused"}
            </p>
          </Link>
        ))}
      </div>
    </>
  );
}
