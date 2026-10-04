import { describe, expect, it } from "vitest";
import { errorMessage } from "./errors";

describe("errorMessage", () => {
  it("maps program error codes to their message", () => {
    // 6009 = MintPaused
    expect(errorMessage(new Error("failed: custom program error: 0x1779"))).toBe("Minting is paused for this series");
  });
  it("explains wallet cancellations", () => {
    expect(errorMessage(new Error("User rejected the request."))).toBe("You cancelled the transaction in your wallet.");
  });
});
