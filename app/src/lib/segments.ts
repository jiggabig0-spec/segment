// Client for the segments program: reading series and building mint/redeem transactions.
import { BN, Program, type Provider } from "@anchor-lang/core";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountIdempotentInstruction,
  getAssociatedTokenAddressSync,
  getTokenMetadata,
  TOKEN_2022_PROGRAM_ID,
  unpackAccount,
} from "@solana/spl-token";
import {
  ComputeBudgetProgram,
  Connection,
  PublicKey,
  type AccountMeta,
  type TransactionInstruction,
} from "@solana/web3.js";
import idl from "@/idl/segments.json";
import type { Segments } from "@/idl/segments";
import { PROGRAM_ID } from "./config";

export type SeriesInfo = {
  address: PublicKey;
  seriesId: number;
  underlyingMint: PublicKey;
  underlyingTokenProgram: PublicKey;
  decimals: number;
  vault: PublicKey;
  partialMints: PublicKey[];
  weightsBps: number[];
  finalized: boolean;
  mintPaused: boolean;
  mintFeeBps: number;
  redeemFeeBps: number;
  pendingFee: { mintFeeBps: number; redeemFeeBps: number; effectiveAt: number } | null;
  outstandingSets: bigint;
  metadataUri: string;
};

export type PartialInfo = {
  mint: PublicKey;
  index: number;
  weightBps: number;
  name: string;
  symbol: string;
  uri: string;
};

// 7 partials means 14 token CPIs in one instruction; the default 200k CU is not enough.
const COMPUTE_UNITS = 600_000;

export function segmentsProgram(provider: Provider | { connection: Connection }): Program<Segments> {
  return new Program<Segments>({ ...(idl as Segments), address: PROGRAM_ID.toBase58() }, provider as Provider);
}

const toBig = (n: BN) => BigInt(n.toString());

export async function fetchAllSeries(program: Program<Segments>): Promise<SeriesInfo[]> {
  const rows = await program.account.series.all();
  return rows.map((r) => toSeriesInfo(r.publicKey, r.account)).sort((a, b) => a.seriesId - b.seriesId);
}

export async function fetchSeries(program: Program<Segments>, address: PublicKey): Promise<SeriesInfo> {
  return toSeriesInfo(address, await program.account.series.fetch(address));
}

type RawSeries = Awaited<ReturnType<Program<Segments>["account"]["series"]["fetch"]>>;

function toSeriesInfo(address: PublicKey, s: RawSeries): SeriesInfo {
  const n = s.partialCount;
  return {
    address,
    seriesId: s.seriesId,
    underlyingMint: s.underlyingMint,
    underlyingTokenProgram: s.underlyingTokenProgram,
    decimals: s.underlyingDecimals,
    vault: s.vault,
    partialMints: s.partialMints.slice(0, n),
    weightsBps: s.weightsBps.slice(0, n),
    finalized: s.finalized,
    mintPaused: s.mintPaused,
    mintFeeBps: s.mintFeeBps,
    redeemFeeBps: s.redeemFeeBps,
    pendingFee: s.pendingFee
      ? {
          mintFeeBps: s.pendingFee.mintFeeBps,
          redeemFeeBps: s.pendingFee.redeemFeeBps,
          effectiveAt: s.pendingFee.effectiveAt.toNumber(),
        }
      : null,
    outstandingSets: toBig(s.outstandingSets),
    metadataUri: s.metadataUri,
  };
}

/** Name and symbol come from each partial's Token-2022 metadata extension. */
export async function fetchPartials(connection: Connection, series: SeriesInfo): Promise<PartialInfo[]> {
  return Promise.all(
    series.partialMints.map(async (mint, index) => {
      const meta = await getTokenMetadata(connection, mint, "confirmed", TOKEN_2022_PROGRAM_ID).catch(() => null);
      return {
        mint,
        index,
        weightBps: series.weightsBps[index],
        name: meta?.name ?? `Partial ${index + 1}`,
        symbol: meta?.symbol ?? `P${index + 1}`,
        uri: meta?.uri ?? "",
      };
    }),
  );
}

/** Raw balances for the stock token and every partial. Missing accounts count as zero. */
export async function fetchBalances(connection: Connection, series: SeriesInfo, owner: PublicKey) {
  const underlyingAta = getAssociatedTokenAddressSync(series.underlyingMint, owner, true, series.underlyingTokenProgram);
  const partialAtas = series.partialMints.map((m) => getAssociatedTokenAddressSync(m, owner, true, TOKEN_2022_PROGRAM_ID));
  const infos = await connection.getMultipleAccountsInfo([underlyingAta, ...partialAtas], "confirmed");
  const amount = (i: number, address: PublicKey, program: PublicKey) => {
    const info = infos[i];
    return info ? unpackAccount(address, info, program).amount : BigInt(0);
  };
  const underlying = amount(0, underlyingAta, series.underlyingTokenProgram);
  const partials = partialAtas.map((a, i) => amount(i + 1, a, TOKEN_2022_PROGRAM_ID));
  // A complete set needs one of every partial, so the smallest balance is what can be redeemed.
  const completeSets = partials.length ? partials.reduce((a, b) => (b < a ? b : a)) : BigInt(0);
  return { underlying, partials, completeSets };
}

function partialRemainingAccounts(series: SeriesInfo, owner: PublicKey): AccountMeta[] {
  return series.partialMints.flatMap((mint) => [
    { pubkey: mint, isSigner: false, isWritable: true },
    {
      pubkey: getAssociatedTokenAddressSync(mint, owner, true, TOKEN_2022_PROGRAM_ID),
      isSigner: false,
      isWritable: true,
    },
  ]);
}

/** Instructions creating any partial token accounts the user doesn't have yet. Empty if none. */
export async function setupInstructions(
  connection: Connection,
  series: SeriesInfo,
  owner: PublicKey,
): Promise<TransactionInstruction[]> {
  const atas = series.partialMints.map((m) => getAssociatedTokenAddressSync(m, owner, true, TOKEN_2022_PROGRAM_ID));
  const infos = await connection.getMultipleAccountsInfo(atas, "confirmed");
  return series.partialMints
    .filter((_, i) => !infos[i])
    .map((mint) =>
      createAssociatedTokenAccountIdempotentInstruction(
        owner,
        getAssociatedTokenAddressSync(mint, owner, true, TOKEN_2022_PROGRAM_ID),
        owner,
        mint,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
    );
}

export async function mintSetInstructions(
  program: Program<Segments>,
  series: SeriesInfo,
  owner: PublicKey,
  amount: bigint,
): Promise<TransactionInstruction[]> {
  const ix = await program.methods
    .mintSet(new BN(amount.toString()))
    .accountsPartial({
      user: owner,
      series: series.address,
      underlyingMint: series.underlyingMint,
      vault: series.vault,
      userUnderlying: getAssociatedTokenAddressSync(series.underlyingMint, owner, true, series.underlyingTokenProgram),
      underlyingTokenProgram: series.underlyingTokenProgram,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .remainingAccounts(partialRemainingAccounts(series, owner))
    .instruction();
  return [ComputeBudgetProgram.setComputeUnitLimit({ units: COMPUTE_UNITS }), ix];
}

export async function redeemSetInstructions(
  program: Program<Segments>,
  series: SeriesInfo,
  owner: PublicKey,
  sets: bigint,
): Promise<TransactionInstruction[]> {
  const userUnderlying = getAssociatedTokenAddressSync(series.underlyingMint, owner, true, series.underlyingTokenProgram);
  const ix = await program.methods
    .redeemSet(new BN(sets.toString()))
    .accountsPartial({
      user: owner,
      series: series.address,
      underlyingMint: series.underlyingMint,
      vault: series.vault,
      userUnderlying,
      underlyingTokenProgram: series.underlyingTokenProgram,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .remainingAccounts(partialRemainingAccounts(series, owner))
    .instruction();
  return [
    ComputeBudgetProgram.setComputeUnitLimit({ units: COMPUTE_UNITS }),
    // The user may have sold all their stock token, so make sure the payout account exists.
    createAssociatedTokenAccountIdempotentInstruction(
      owner,
      userUnderlying,
      owner,
      series.underlyingMint,
      series.underlyingTokenProgram,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    ),
    ix,
  ];
}

/** Swap link for a token on Jupiter, priced in USDC. Pools are on existing AMMs, not ours. */
export function tradeUrl(mint: PublicKey): string {
  return `https://jup.ag/swap/USDC-${mint.toBase58()}`;
}

export function explorerUrl(sig: string, rpcUrl: string): string {
  const cluster = rpcUrl.includes("devnet") ? "?cluster=devnet" : "";
  return `https://explorer.solana.com/tx/${sig}${cluster}`;
}
