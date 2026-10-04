import { describe, expect, it } from "vitest";
import { feeCeil, formatAmount, formatBps, mintPreview, parseAmount, redeemPreview } from "./amounts";

describe("feeCeil", () => {
  // Same cases as the Rust tests in programs/segments/src/math.rs.
  it("matches the program", () => {
    expect(feeCeil(BigInt(1_000_000), 0)).toBe(BigInt(0));
    expect(feeCeil(BigInt(1), 30)).toBe(BigInt(1));
    expect(feeCeil(BigInt(10_000), 30)).toBe(BigInt(30));
    expect(feeCeil(BigInt(10_001), 30)).toBe(BigInt(31));
  });
});

describe("previews", () => {
  it("mint: 1 GOOGLx at 30 bps", () => {
    expect(mintPreview(BigInt(100_000_000), 30)).toEqual({ fee: BigInt(300_000), sets: BigInt(99_700_000) });
  });
  it("redeem: fee comes out of the payout", () => {
    expect(redeemPreview(BigInt(10_000_000), 30)).toEqual({ fee: BigInt(30_000), withdrawn: BigInt(9_970_000) });
  });
});

describe("parseAmount", () => {
  it("parses decimals", () => {
    expect(parseAmount("1.25", 8)).toBe(BigInt(125_000_000));
    expect(parseAmount(".5", 8)).toBe(BigInt(50_000_000));
    expect(parseAmount("3", 8)).toBe(BigInt(300_000_000));
  });
  it("rejects junk and too many decimals", () => {
    expect(parseAmount("", 8)).toBeNull();
    expect(parseAmount("abc", 8)).toBeNull();
    expect(parseAmount("1.2.3", 8)).toBeNull();
    expect(parseAmount("0.000000001", 8)).toBeNull();
  });
});

describe("format", () => {
  it("formats raw amounts", () => {
    expect(formatAmount(BigInt(125_000_000), 8)).toBe("1.25");
    expect(formatAmount(BigInt(1_234_500_000_000), 8)).toBe("12,345");
    expect(formatAmount(BigInt(0), 8)).toBe("0");
  });
  it("formats bps", () => {
    expect(formatBps(5_350)).toBe("53.5%");
    expect(formatBps(400)).toBe("4%");
    expect(formatBps(30)).toBe("0.3%");
  });
});
