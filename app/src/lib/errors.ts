import idl from "@/idl/segments.json";

const PROGRAM_ERRORS = new Map(idl.errors.map((e) => [e.code, e.msg]));

/** Turns wallet, RPC and program failures into one readable sentence. */
export function errorMessage(err: unknown): string {
  const text = err instanceof Error ? err.message : String(err);
  const logs = (err as { logs?: string[] })?.logs?.join("\n") ?? "";
  const hex = /custom program error: 0x([0-9a-f]+)/i.exec(text + "\n" + logs);
  if (hex) {
    const msg = PROGRAM_ERRORS.get(parseInt(hex[1], 16));
    if (msg) return msg;
  }
  if (/User rejected/i.test(text)) return "You cancelled the transaction in your wallet.";
  if (/insufficient funds|0x1\b/i.test(text + logs)) return "Not enough balance for this transaction.";
  return text.length > 200 ? `${text.slice(0, 200)}…` : text;
}
