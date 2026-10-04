# Segments web app

Next.js app for the `segments` program: browse series, mint and redeem complete sets, see your
segment balances, and jump to Jupiter to trade any segment token.

```sh
cp .env.example .env.local   # set the RPC URL and program id
npm install
npm run dev                  # http://localhost:3000
npm test                     # amount math and error messages
npm run build
```

## Demo mode

`NEXT_PUBLIC_DEMO=1` runs the site on built-in Alphabet data (7 segments, Waymo included) with a
pretend wallet holding 10 GOOGLx, so it can be shown before the program is deployed. Nothing
touches the chain, and the Trade buttons are disabled.

```sh
npm run build:demo           # static site in out/, host it anywhere
```

Wallets that support the Wallet Standard (Phantom, Solflare, Backpack and most others) show up
automatically.

## How it talks to the chain

- `src/lib/segments.ts` reads `Series` accounts and segment metadata, and builds the `mint_set`
  and `redeem_set` instructions (partials passed as remaining accounts, in series order).
- First-time minters get a one-off setup transaction that creates their segment token accounts.
- Mint and redeem raise the compute limit to 600k, since a 7-segment series makes 14 token calls.
- `src/lib/amounts.ts` mirrors the program's fee rounding so previews match what you receive.

`src/idl/` is generated: run `python3 scripts/build_idl.py` from the repo root after changing
the program.

## Not done yet

- One-click "buy just this segment" (mint a set, sell the rest in one go). Today "Trade" opens
  Jupiter, which only works once pools exist for the segment tokens.
- Prices and charts (needs pools and an indexer).
