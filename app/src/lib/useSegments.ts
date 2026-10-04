"use client";

import { AnchorProvider } from "@anchor-lang/core";
import { useAnchorWallet, useConnection, useWallet } from "@solana/wallet-adapter-react";
import { Transaction, type TransactionInstruction } from "@solana/web3.js";
import { useCallback, useMemo } from "react";
import { segmentsProgram } from "./segments";

/** The program client, signing with the connected wallet when there is one. */
export function useProgram() {
  const { connection } = useConnection();
  const wallet = useAnchorWallet();
  return useMemo(
    () =>
      segmentsProgram(
        wallet ? new AnchorProvider(connection, wallet, { commitment: "confirmed" }) : { connection },
      ),
    [connection, wallet],
  );
}

/** Sends instructions as one transaction through the wallet and waits for confirmation. */
export function useSend() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();
  return useCallback(
    async (ixs: TransactionInstruction[]) => {
      if (!publicKey) throw new Error("Connect a wallet first.");
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash("confirmed");
      const tx = new Transaction({ feePayer: publicKey, blockhash, lastValidBlockHeight }).add(...ixs);
      const signature = await sendTransaction(tx, connection);
      const res = await connection.confirmTransaction({ signature, blockhash, lastValidBlockHeight }, "confirmed");
      if (res.value.err) throw new Error(`Transaction failed: ${JSON.stringify(res.value.err)}`);
      return signature;
    },
    [connection, publicKey, sendTransaction],
  );
}
