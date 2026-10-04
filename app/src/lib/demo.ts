// Demo mode (NEXT_PUBLIC_DEMO=1): the site runs on built-in Alphabet data with a simulated
// wallet, so it can be shown before the program is deployed. Nothing touches the chain.
import { PublicKey } from "@solana/web3.js";
import { mintPreview, redeemPreview } from "./amounts";
import type { PartialInfo, SeriesInfo } from "./segments";

export const DEMO = process.env.NEXT_PUBLIC_DEMO === "1";

const key = (seed: string) => PublicKey.findProgramAddressSync([Buffer.from(seed)], PublicKey.default)[0];

const SEGMENTS: [string, string, number][] = [
  ["SRCH", "Alphabet Search", 5_350],
  ["GCP", "Alphabet Cloud", 2_600],
  ["YT", "Alphabet YouTube", 850],
  ["SUBS", "Alphabet Subscriptions", 500],
  ["WAYMO", "Alphabet Waymo", 400],
  ["BETS", "Alphabet Other Bets", 100],
  ["NETW", "Alphabet Network", 200],
];

export const DEMO_SERIES: SeriesInfo = {
  address: key("demo-series-googl-1"),
  seriesId: 1,
  underlyingMint: key("demo-googlx"),
  underlyingTokenProgram: key("demo-token-program"),
  decimals: 8,
  vault: key("demo-vault"),
  partialMints: SEGMENTS.map(([s]) => key(`demo-${s}`)),
  weightsBps: SEGMENTS.map(([, , w]) => w),
  finalized: true,
  mintPaused: false,
  mintFeeBps: 10,
  redeemFeeBps: 10,
  pendingFee: null,
  outstandingSets: BigInt(1_250) * BigInt(100_000_000),
  metadataUri: "",
};

export const DEMO_PARTIALS: PartialInfo[] = SEGMENTS.map(([symbol, name, weightBps], index) => ({
  mint: DEMO_SERIES.partialMints[index],
  index,
  weightBps,
  name,
  symbol,
  uri: "",
}));

// The simulated wallet starts with 10 GOOGLx and no segment tokens.
const wallet = {
  underlying: BigInt(10) * BigInt(100_000_000),
  partials: SEGMENTS.map(() => BigInt(0)),
};

export function demoBalances() {
  const completeSets = wallet.partials.reduce((a, b) => (b < a ? b : a));
  return { underlying: wallet.underlying, partials: [...wallet.partials], completeSets };
}

export function demoMint(amount: bigint) {
  if (amount > wallet.underlying) throw new Error("Not enough balance for this transaction.");
  const { sets } = mintPreview(amount, DEMO_SERIES.mintFeeBps);
  wallet.underlying -= amount;
  wallet.partials = wallet.partials.map((p) => p + sets);
  DEMO_SERIES.outstandingSets += sets;
}

export function demoRedeem(sets: bigint) {
  if (wallet.partials.some((p) => p < sets)) throw new Error("Not enough balance for this transaction.");
  const { withdrawn } = redeemPreview(sets, DEMO_SERIES.redeemFeeBps);
  wallet.partials = wallet.partials.map((p) => p - sets);
  wallet.underlying += withdrawn;
  DEMO_SERIES.outstandingSets -= sets;
}
