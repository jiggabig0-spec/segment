import { describe, expect, it } from "vitest";
import { demoBalances, demoMint, demoRedeem, DEMO_PARTIALS, DEMO_SERIES } from "./demo";

describe("demo wallet", () => {
  it("has the 7-segment Alphabet series with Waymo", () => {
    expect(DEMO_PARTIALS.map((p) => p.symbol)).toContain("WAYMO");
    expect(DEMO_SERIES.weightsBps.reduce((a, b) => a + b, 0)).toBe(10_000);
  });

  it("mints and redeems with the same fee rounding as the program", () => {
    demoMint(BigInt(100_000_000));
    expect(demoBalances().completeSets).toBe(BigInt(99_900_000));
    demoRedeem(BigInt(99_900_000));
    const b = demoBalances();
    expect(b.completeSets).toBe(BigInt(0));
    expect(b.underlying).toBe(BigInt(1_000_000_000) - BigInt(100_000_000) + BigInt(99_800_100));
  });
});
