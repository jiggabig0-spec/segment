# segments

Solana program that splits a tokenized stock into a fixed set of segment tokens ("partials"),
and back. First target: Alphabet (GOOGLx), see `series/googl-s1.json`.

- **Mint a set:** deposit N units of the stock token, get N units of every partial.
- **Redeem a set:** burn N units of every partial, get N units of the stock token back.
- Partials trade on existing Solana AMM pools; arbitrage keeps the set price near the stock.

## Safety properties (enforced in code and covered by tests)

| Property | Where |
|---|---|
| A series' partials are locked at `finalize_series`; no instruction adds, removes or retires one | `instructions/series.rs` |
| Redemption has no pause or admin check | `instructions/mint_redeem.rs` (`RedeemSet`) |
| Guardian can pause **minting only** | `instructions/fees.rs` (`set_mint_paused`) |
| Fees hard-capped at 30 bps; increases wait 7 days, decreases apply at once | `constants.rs`, `instructions/fees.rs` |
| Fees accrue in the vault; only `sweep_fees` moves them, and only to the treasury | `instructions/fees.rs` |
| Partial mints: mint authority = series PDA, no freeze authority, no permanent delegate | `AddPartial` |
| Only the program's upgrade authority can run `init_config` (no front-running the deploy) | `instructions/admin.rs` |
| Invariant: each partial's supply = outstanding sets; vault = outstanding sets + unswept fees | `tests/segments.rs::assert_invariants` |

Upgrade authority, timelock and eventual immutability are deployment settings, not code; see
`/mnt/project-files/platform/build-plan.md` §3.6.

## Instructions

`init_config`, `propose_admin`, `accept_admin`, `set_guardian`, `set_treasury`,
`create_series`, `add_partial`, `finalize_series`, `set_series_uri`,
`schedule_fee`, `cancel_fee`, `apply_fee`, `set_mint_paused`, `sweep_fees`,
`mint_set`, `redeem_set`.

`mint_set` / `redeem_set` take the partials as remaining accounts, in series order:
`[partial_mint_0, token_account_0, partial_mint_1, token_account_1, ...]`.

## Build and test

Requires Rust. Uses Anchor 1.2.

```sh
cargo test          # 3 unit + 11 end-to-end tests
```

Tests run the program natively inside `solana-program-test`, with the real Token-2022 and
Associated Token Account programs. Anchor 1.x can't do CPIs off-chain, so `vendor/solana-invoke`
patches **host builds only** to route CPIs through the test runtime; the on-chain code path is
upstream's. Once the Solana toolchain is set up (`anchor build`), the next step is to also run the
tests against the compiled `.so` with LiteSVM and drop the patch.

To deploy you need the Solana CLI and Anchor CLI, then `anchor keys sync` to replace the
placeholder program id with your own keypair's.

## Web app

`app/` is the Next.js front end: series list, mint and redeem, balances, and trade links. See
`app/README.md`.

## IDL

`idl/segments.json` (and its TypeScript type) is generated without the Anchor CLI:

```sh
python3 scripts/build_idl.py
```

## Status

Scaffold. Not audited. Do not deploy to mainnet.
