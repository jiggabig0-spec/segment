import { clusterApiUrl, PublicKey } from "@solana/web3.js";
import idl from "@/idl/segments.json";

export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || clusterApiUrl("devnet");
export const NETWORK_LABEL = process.env.NEXT_PUBLIC_NETWORK_LABEL || "Devnet";
export const PROGRAM_ID = new PublicKey(process.env.NEXT_PUBLIC_PROGRAM_ID || idl.address);
