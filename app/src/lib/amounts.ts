// Pure helpers for raw token amounts. Raw amounts are bigint so 8-decimal stocks never lose precision.

const BPS = BigInt(10_000);

/** Mirrors `math::fee_ceil` in the program: fee rounded up, zero rate is free. */
export function feeCeil(amount: bigint, bps: number): bigint {
  if (bps === 0 || amount === BigInt(0)) return BigInt(0);
  return (amount * BigInt(bps) + BPS - BigInt(1)) / BPS;
}

/** What `mint_set` gives for depositing `amount` (assuming no transfer fee on the stock token). */
export function mintPreview(amount: bigint, mintFeeBps: number) {
  const fee = feeCeil(amount, mintFeeBps);
  return { fee, sets: amount - fee };
}

/** What `redeem_set` pays out for burning `sets` complete sets. */
export function redeemPreview(sets: bigint, redeemFeeBps: number) {
  const fee = feeCeil(sets, redeemFeeBps);
  return { fee, withdrawn: sets - fee };
}

/** Parses a user-typed decimal like "1.25" into raw units. Returns null if invalid. */
export function parseAmount(input: string, decimals: number): bigint | null {
  const s = input.trim();
  if (!/^\d*\.?\d*$/.test(s) || s === "" || s === ".") return null;
  const [whole, frac = ""] = s.split(".");
  if (frac.length > decimals) return null;
  return BigInt(whole || "0") * BigInt(10) ** BigInt(decimals) + BigInt(frac.padEnd(decimals, "0") || "0");
}

/** Formats raw units for display, trimming trailing zeros. */
export function formatAmount(raw: bigint, decimals: number, maxFraction = decimals): string {
  const base = BigInt(10) ** BigInt(decimals);
  const whole = raw / base;
  let frac = (raw % base).toString().padStart(decimals, "0").slice(0, maxFraction).replace(/0+$/, "");
  return frac ? `${whole.toLocaleString("en-US")}.${frac}` : whole.toLocaleString("en-US");
}

export function formatBps(bps: number): string {
  return `${(bps / 100).toFixed(2).replace(/\.?0+$/, "")}%`;
}
