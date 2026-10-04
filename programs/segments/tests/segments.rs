#![allow(deprecated)] // solana-program-test 3.x marks its API unstable; still the right tool here.

//! End-to-end tests: the program runs natively, Token-2022 and the ATA program run as the real
//! bundled SBF binaries.

use anchor_lang::{
    prelude::{AccountInfo, Clock, Pubkey},
    solana_program::{bpf_loader_upgradeable, entrypoint::ProgramResult},
    AccountDeserialize, InstructionData, ToAccountMetas,
};
use anchor_spl::token_interface::spl_token_metadata_interface::state::TokenMetadata;
use segments::{
    constants::*,
    state::{Config, Series},
};
use solana_program_test::{processor, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::{instruction as system_instruction, program as system_program};
use spl_associated_token_account_interface::{
    address::get_associated_token_address_with_program_id as ata,
    instruction::create_associated_token_account_idempotent,
};
use spl_token_2022_interface::{
    extension::{BaseStateWithExtensions, StateWithExtensions},
    instruction as token_ix,
    state::{Account as TokenAccountState, Mint as MintState},
    ID as TOKEN_2022,
};

const DECIMALS: u8 = 8;

fn entry_wrapper(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Anchor's entrypoint ties every argument to one lifetime; leaking is fine in tests.
    let program_id: &Pubkey = Box::leak(Box::new(*program_id));
    let data: &[u8] = Box::leak(data.to_vec().into_boxed_slice());
    let accounts = Box::leak(Box::new(accounts.to_vec()));
    segments::entry(program_id, accounts, data)
}

struct Env {
    ctx: ProgramTestContext,
    admin: Keypair,
    guardian: Keypair,
    treasury: Keypair,
    user: Keypair,
    underlying: Pubkey,
}

fn config_pda() -> Pubkey {
    Pubkey::find_program_address(&[CONFIG_SEED], &segments::ID).0
}

fn series_pda(underlying: &Pubkey, id: u16) -> Pubkey {
    Pubkey::find_program_address(
        &[SERIES_SEED, underlying.as_ref(), &id.to_le_bytes()],
        &segments::ID,
    )
    .0
}

fn vault_pda(series: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[VAULT_SEED, series.as_ref()], &segments::ID).0
}

fn partial_pda(series: &Pubkey, index: u8) -> Pubkey {
    Pubkey::find_program_address(&[PARTIAL_SEED, series.as_ref(), &[index]], &segments::ID).0
}

fn program_data_pda() -> Pubkey {
    Pubkey::find_program_address(&[segments::ID.as_ref()], &bpf_loader_upgradeable::ID).0
}

/// Fake upgradeable-loader ProgramData so `init_config` can check the upgrade authority.
fn program_data_account(authority: &Pubkey) -> Account {
    let mut data = Vec::with_capacity(45);
    data.extend_from_slice(&3u32.to_le_bytes()); // UpgradeableLoaderState::ProgramData
    data.extend_from_slice(&0u64.to_le_bytes()); // slot
    data.push(1); // Some(
    data.extend_from_slice(authority.as_ref()); // authority)
    Account {
        lamports: 1_000_000_000,
        data,
        owner: bpf_loader_upgradeable::ID,
        executable: false,
        rent_epoch: 0,
    }
}

fn funded() -> Account {
    Account {
        lamports: 100_000_000_000,
        data: vec![],
        owner: system_program::ID,
        executable: false,
        rent_epoch: 0,
    }
}

impl Env {
    async fn new() -> Self {
        let admin = Keypair::new();
        let guardian = Keypair::new();
        let treasury = Keypair::new();
        let user = Keypair::new();
        let mut pt = ProgramTest::new("segments", segments::ID, processor!(entry_wrapper));
        pt.prefer_bpf(false);
        pt.add_account(program_data_pda(), program_data_account(&admin.pubkey()));
        for k in [&admin, &guardian, &user] {
            pt.add_account(k.pubkey(), funded());
        }
        let ctx = pt.start_with_context().await;
        let mut env = Env {
            ctx,
            admin,
            guardian,
            treasury,
            user,
            underlying: Pubkey::default(),
        };
        env.underlying = env.create_underlying().await;
        env
    }

    async fn send(
        &mut self,
        ixs: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<(), BanksClientError> {
        let payer = self.ctx.payer.insecure_clone();
        let mut all: Vec<&Keypair> = vec![&payer];
        all.extend_from_slice(signers);
        let blockhash = self.ctx.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), &all, blockhash);
        self.ctx.banks_client.process_transaction(tx).await
    }

    /// Stand-in for a tokenized stock: a plain Token-2022 mint with 8 decimals, 10 whole units to the user.
    async fn create_underlying(&mut self) -> Pubkey {
        let mint = Keypair::new();
        let rent = self.ctx.banks_client.get_rent().await.unwrap();
        let payer = self.ctx.payer.pubkey();
        let ixs = vec![
            system_instruction::create_account(
                &payer,
                &mint.pubkey(),
                rent.minimum_balance(MintState::LEN),
                MintState::LEN as u64,
                &TOKEN_2022,
            ),
            token_ix::initialize_mint2(&TOKEN_2022, &mint.pubkey(), &payer, None, DECIMALS)
                .unwrap(),
            create_associated_token_account_idempotent(
                &payer,
                &self.user.pubkey(),
                &mint.pubkey(),
                &TOKEN_2022,
            ),
            token_ix::mint_to(
                &TOKEN_2022,
                &mint.pubkey(),
                &ata(&self.user.pubkey(), &mint.pubkey(), &TOKEN_2022),
                &payer,
                &[],
                10 * 10u64.pow(DECIMALS as u32),
            )
            .unwrap(),
        ];
        self.send(&ixs, &[&mint]).await.unwrap();
        mint.pubkey()
    }

    async fn init_config(&mut self) {
        let ix = Instruction {
            program_id: segments::ID,
            accounts: segments::accounts::InitConfig {
                payer: self.admin.pubkey(),
                config: config_pda(),
                program_data: program_data_pda(),
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: segments::instruction::InitConfig {
                admin: self.admin.pubkey(),
                guardian: self.guardian.pubkey(),
                treasury: self.treasury.pubkey(),
            }
            .data(),
        };
        let admin = self.admin.insecure_clone();
        self.send(&[ix], &[&admin]).await.unwrap();
    }

    fn create_series_ix(&self, signer: &Pubkey, id: u16) -> Instruction {
        let series = series_pda(&self.underlying, id);
        Instruction {
            program_id: segments::ID,
            accounts: segments::accounts::CreateSeries {
                admin: *signer,
                config: config_pda(),
                underlying_mint: self.underlying,
                series,
                vault: vault_pda(&series),
                underlying_token_program: TOKEN_2022,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: segments::instruction::CreateSeries {
                series_id: id,
                metadata_uri: "https://example.com/googl-s1.json".into(),
            }
            .data(),
        }
    }

    fn add_partial_ix(&self, series: &Pubkey, index: u8, weight: u16, symbol: &str) -> Instruction {
        Instruction {
            program_id: segments::ID,
            accounts: segments::accounts::AddPartial {
                admin: self.admin.pubkey(),
                config: config_pda(),
                series: *series,
                partial_mint: partial_pda(series, index),
                token_program: TOKEN_2022,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
            data: segments::instruction::AddPartial {
                weight_bps: weight,
                name: format!("Alphabet {symbol}"),
                symbol: symbol.into(),
                uri: format!("https://example.com/{symbol}.json"),
            }
            .data(),
        }
    }

    fn series_admin_ix(&self, series: &Pubkey, data: Vec<u8>) -> Instruction {
        Instruction {
            program_id: segments::ID,
            accounts: segments::accounts::SeriesAdmin {
                admin: self.admin.pubkey(),
                config: config_pda(),
                series: *series,
            }
            .to_account_metas(None),
            data,
        }
    }

    fn finalize_ix(&self, series: &Pubkey) -> Instruction {
        Instruction {
            program_id: segments::ID,
            accounts: segments::accounts::FinalizeSeries {
                admin: self.admin.pubkey(),
                config: config_pda(),
                series: *series,
            }
            .to_account_metas(None),
            data: segments::instruction::FinalizeSeries {}.data(),
        }
    }

    /// Creates and finalizes a 3-partial series, and gives the user token accounts for each partial.
    async fn setup_series(&mut self) -> Pubkey {
        self.init_config().await;
        let series = series_pda(&self.underlying, 1);
        let admin = self.admin.insecure_clone();
        let ixs = vec![
            self.create_series_ix(&admin.pubkey(), 1),
            self.add_partial_ix(&series, 0, 5_000, "SRCH"),
            self.add_partial_ix(&series, 1, 3_000, "GCP"),
            self.add_partial_ix(&series, 2, 2_000, "BETS"),
            self.finalize_ix(&series),
        ];
        self.send(&ixs, &[&admin]).await.unwrap();
        let payer = self.ctx.payer.pubkey();
        let ixs: Vec<_> = (0..3)
            .map(|i| {
                create_associated_token_account_idempotent(
                    &payer,
                    &self.user.pubkey(),
                    &partial_pda(&series, i),
                    &TOKEN_2022,
                )
            })
            .collect();
        self.send(&ixs, &[]).await.unwrap();
        series
    }

    fn partial_metas(&self, series: &Pubkey, owner: &Pubkey, order: &[u8]) -> Vec<AccountMeta> {
        order
            .iter()
            .flat_map(|&i| {
                let mint = partial_pda(series, i);
                [
                    AccountMeta::new(mint, false),
                    AccountMeta::new(ata(owner, &mint, &TOKEN_2022), false),
                ]
            })
            .collect()
    }

    fn mint_ix(&self, series: &Pubkey, amount: u64, order: &[u8]) -> Instruction {
        let mut accounts = segments::accounts::MintSet {
            user: self.user.pubkey(),
            series: *series,
            underlying_mint: self.underlying,
            vault: vault_pda(series),
            user_underlying: ata(&self.user.pubkey(), &self.underlying, &TOKEN_2022),
            underlying_token_program: TOKEN_2022,
            token_program: TOKEN_2022,
        }
        .to_account_metas(None);
        accounts.extend(self.partial_metas(series, &self.user.pubkey(), order));
        Instruction {
            program_id: segments::ID,
            accounts,
            data: segments::instruction::MintSet { amount }.data(),
        }
    }

    fn redeem_ix(&self, series: &Pubkey, sets: u64, order: &[u8]) -> Instruction {
        let mut accounts = segments::accounts::RedeemSet {
            user: self.user.pubkey(),
            series: *series,
            underlying_mint: self.underlying,
            vault: vault_pda(series),
            user_underlying: ata(&self.user.pubkey(), &self.underlying, &TOKEN_2022),
            underlying_token_program: TOKEN_2022,
            token_program: TOKEN_2022,
        }
        .to_account_metas(None);
        accounts.extend(self.partial_metas(series, &self.user.pubkey(), order));
        Instruction {
            program_id: segments::ID,
            accounts,
            data: segments::instruction::RedeemSet { sets }.data(),
        }
    }

    async fn mint_set(&mut self, series: &Pubkey, amount: u64) -> Result<(), BanksClientError> {
        let ix = self.mint_ix(series, amount, &[0, 1, 2]);
        let user = self.user.insecure_clone();
        self.send(&[ix], &[&user]).await
    }

    async fn redeem_set(&mut self, series: &Pubkey, sets: u64) -> Result<(), BanksClientError> {
        let ix = self.redeem_ix(series, sets, &[0, 1, 2]);
        let user = self.user.insecure_clone();
        self.send(&[ix], &[&user]).await
    }

    async fn token_balance(&mut self, account: &Pubkey) -> u64 {
        let acc = self
            .ctx
            .banks_client
            .get_account(*account)
            .await
            .unwrap()
            .unwrap();
        StateWithExtensions::<TokenAccountState>::unpack(&acc.data)
            .unwrap()
            .base
            .amount
    }

    async fn supply(&mut self, mint: &Pubkey) -> u64 {
        let acc = self
            .ctx
            .banks_client
            .get_account(*mint)
            .await
            .unwrap()
            .unwrap();
        StateWithExtensions::<MintState>::unpack(&acc.data)
            .unwrap()
            .base
            .supply
    }

    async fn user_underlying(&mut self) -> u64 {
        let a = ata(&self.user.pubkey(), &self.underlying, &TOKEN_2022);
        self.token_balance(&a).await
    }

    async fn user_partial(&mut self, series: &Pubkey, i: u8) -> u64 {
        let a = ata(&self.user.pubkey(), &partial_pda(series, i), &TOKEN_2022);
        self.token_balance(&a).await
    }

    async fn series(&mut self, series: &Pubkey) -> Series {
        let acc = self
            .ctx
            .banks_client
            .get_account(*series)
            .await
            .unwrap()
            .unwrap();
        Series::try_deserialize(&mut acc.data.as_slice()).unwrap()
    }

    async fn warp_seconds(&mut self, secs: i64) {
        let mut clock: Clock = self.ctx.banks_client.get_sysvar().await.unwrap();
        clock.unix_timestamp += secs;
        self.ctx.set_sysvar(&clock);
    }

    /// The core invariant: every partial's supply equals outstanding sets, and the vault holds
    /// exactly outstanding sets plus unswept fees.
    async fn assert_invariants(&mut self, series: &Pubkey) {
        let s = self.series(series).await;
        for i in 0..s.partial_count {
            assert_eq!(
                self.supply(&partial_pda(series, i)).await,
                s.outstanding_sets,
                "partial {i} supply"
            );
        }
        let vault = self.token_balance(&vault_pda(series)).await;
        assert_eq!(vault, s.outstanding_sets + s.accrued_fees, "vault backing");
    }
}

#[tokio::test]
async fn mint_and_redeem_round_trip() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    let start = env.user_underlying().await;

    env.mint_set(&series, 300_000_000).await.unwrap();
    for i in 0..3 {
        assert_eq!(env.user_partial(&series, i).await, 300_000_000);
    }
    assert_eq!(env.user_underlying().await, start - 300_000_000);
    env.assert_invariants(&series).await;

    env.redeem_set(&series, 100_000_000).await.unwrap();
    for i in 0..3 {
        assert_eq!(env.user_partial(&series, i).await, 200_000_000);
    }
    assert_eq!(env.user_underlying().await, start - 200_000_000);
    env.assert_invariants(&series).await;
}

#[tokio::test]
async fn partials_have_metadata_and_no_freeze_authority() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    let acc = env
        .ctx
        .banks_client
        .get_account(partial_pda(&series, 1))
        .await
        .unwrap()
        .unwrap();
    let mint = StateWithExtensions::<MintState>::unpack(&acc.data).unwrap();
    assert_eq!(mint.base.decimals, DECIMALS);
    assert!(mint.base.freeze_authority.is_none());
    assert_eq!(
        Option::<Pubkey>::from(mint.base.mint_authority),
        Some(series)
    );
    let meta = mint.get_variable_len_extension::<TokenMetadata>().unwrap();
    assert_eq!(meta.name, "Alphabet GCP");
    assert_eq!(meta.symbol, "GCP");
    assert_eq!(Option::<Pubkey>::from(meta.update_authority), Some(series));
}

#[tokio::test]
async fn composition_is_locked_after_finalize() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    let ix = env.add_partial_ix(&series, 3, 0, "WAYMO");
    let admin = env.admin.insecure_clone();
    assert!(env.send(&[ix], &[&admin]).await.is_err());
    assert_eq!(env.series(&series).await.partial_count, 3);
}

#[tokio::test]
async fn finalize_requires_weights_to_sum_to_100_percent() {
    let mut env = Env::new().await;
    env.init_config().await;
    let series = series_pda(&env.underlying, 1);
    let admin = env.admin.insecure_clone();
    let ixs = vec![
        env.create_series_ix(&admin.pubkey(), 1),
        env.add_partial_ix(&series, 0, 5_000, "A"),
        env.add_partial_ix(&series, 1, 4_000, "B"),
    ];
    env.send(&ixs, &[&admin]).await.unwrap();
    let ix = env.finalize_ix(&series);
    assert!(env.send(&[ix], &[&admin]).await.is_err());
}

#[tokio::test]
async fn cannot_mint_before_finalize() {
    let mut env = Env::new().await;
    env.init_config().await;
    let series = series_pda(&env.underlying, 1);
    let admin = env.admin.insecure_clone();
    let ixs = vec![
        env.create_series_ix(&admin.pubkey(), 1),
        env.add_partial_ix(&series, 0, 5_000, "A"),
        env.add_partial_ix(&series, 1, 3_000, "B"),
        env.add_partial_ix(&series, 2, 2_000, "C"),
    ];
    env.send(&ixs, &[&admin]).await.unwrap();
    let payer = env.ctx.payer.pubkey();
    let ixs: Vec<_> = (0..3)
        .map(|i| {
            create_associated_token_account_idempotent(
                &payer,
                &env.user.pubkey(),
                &partial_pda(&series, i),
                &TOKEN_2022,
            )
        })
        .collect();
    env.send(&ixs, &[]).await.unwrap();
    assert!(env.mint_set(&series, 1_000).await.is_err());
}

#[tokio::test]
async fn fee_increase_waits_for_timelock_and_is_capped() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    let admin = env.admin.insecure_clone();

    let too_high = env.series_admin_ix(
        &series,
        segments::instruction::ScheduleFee {
            mint_fee_bps: MAX_FEE_BPS + 1,
            redeem_fee_bps: 0,
        }
        .data(),
    );
    assert!(env.send(&[too_high], &[&admin]).await.is_err());

    let schedule = env.series_admin_ix(
        &series,
        segments::instruction::ScheduleFee {
            mint_fee_bps: 30,
            redeem_fee_bps: 30,
        }
        .data(),
    );
    env.send(&[schedule], &[&admin]).await.unwrap();
    assert_eq!(env.series(&series).await.mint_fee_bps, 0);

    let apply = Instruction {
        program_id: segments::ID,
        accounts: segments::accounts::ApplyFee { series }.to_account_metas(None),
        data: segments::instruction::ApplyFee {}.data(),
    };
    assert!(
        env.send(std::slice::from_ref(&apply), &[]).await.is_err(),
        "applied before timelock"
    );

    env.warp_seconds(FEE_TIMELOCK_SECS + 1).await;
    env.send(&[apply], &[]).await.unwrap();
    let s = env.series(&series).await;
    assert_eq!((s.mint_fee_bps, s.redeem_fee_bps), (30, 30));

    // 30 bps of 1.0 token (1e8 units) = 300_000 units, kept in the vault.
    env.mint_set(&series, 100_000_000).await.unwrap();
    assert_eq!(env.user_partial(&series, 0).await, 99_700_000);
    env.assert_invariants(&series).await;

    let before = env.user_underlying().await;
    env.redeem_set(&series, 10_000_000).await.unwrap();
    assert_eq!(env.user_underlying().await - before, 10_000_000 - 30_000);
    env.assert_invariants(&series).await;
    assert_eq!(env.series(&series).await.accrued_fees, 330_000);

    // Fee decreases apply immediately.
    let lower = env.series_admin_ix(
        &series,
        segments::instruction::ScheduleFee {
            mint_fee_bps: 10,
            redeem_fee_bps: 10,
        }
        .data(),
    );
    env.send(&[lower], &[&admin]).await.unwrap();
    assert_eq!(env.series(&series).await.mint_fee_bps, 10);

    // Anyone can sweep fees, but only to the treasury.
    let payer = env.ctx.payer.pubkey();
    let treasury_ata = ata(&env.treasury.pubkey(), &env.underlying, &TOKEN_2022);
    let create = create_associated_token_account_idempotent(
        &payer,
        &env.treasury.pubkey(),
        &env.underlying,
        &TOKEN_2022,
    );
    let sweep = Instruction {
        program_id: segments::ID,
        accounts: segments::accounts::SweepFees {
            config: config_pda(),
            series,
            underlying_mint: env.underlying,
            vault: vault_pda(&series),
            treasury_account: treasury_ata,
            underlying_token_program: TOKEN_2022,
        }
        .to_account_metas(None),
        data: segments::instruction::SweepFees {}.data(),
    };
    env.send(&[create, sweep], &[]).await.unwrap();
    assert_eq!(env.token_balance(&treasury_ata).await, 330_000);
    env.assert_invariants(&series).await;
}

#[tokio::test]
async fn pause_stops_minting_but_never_redemption() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    env.mint_set(&series, 50_000_000).await.unwrap();

    let pause = Instruction {
        program_id: segments::ID,
        accounts: segments::accounts::SetMintPaused {
            authority: env.guardian.pubkey(),
            config: config_pda(),
            series,
        }
        .to_account_metas(None),
        data: segments::instruction::SetMintPaused { paused: true }.data(),
    };
    let guardian = env.guardian.insecure_clone();
    env.send(&[pause], &[&guardian]).await.unwrap();

    assert!(env.mint_set(&series, 1_000).await.is_err());
    env.redeem_set(&series, 50_000_000).await.unwrap();
    env.assert_invariants(&series).await;
    assert_eq!(env.series(&series).await.outstanding_sets, 0);
}

#[tokio::test]
async fn rejects_wrong_or_missing_partials() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    let user = env.user.insecure_clone();
    for order in [&[1u8, 0, 2][..], &[0, 1][..], &[0, 1, 1][..]] {
        let ix = env.mint_ix(&series, 1_000_000, order);
        assert!(
            env.send(&[ix], &[&user]).await.is_err(),
            "order {order:?} accepted"
        );
    }
}

#[tokio::test]
async fn redeem_needs_a_complete_set() {
    let mut env = Env::new().await;
    let series = env.setup_series().await;
    env.mint_set(&series, 10_000_000).await.unwrap();

    // Move away half of one partial; a full redeem must now fail.
    let other = Keypair::new();
    let mint = partial_pda(&series, 2);
    let payer = env.ctx.payer.pubkey();
    let ixs = vec![
        create_associated_token_account_idempotent(&payer, &other.pubkey(), &mint, &TOKEN_2022),
        token_ix::transfer_checked(
            &TOKEN_2022,
            &ata(&env.user.pubkey(), &mint, &TOKEN_2022),
            &mint,
            &ata(&other.pubkey(), &mint, &TOKEN_2022),
            &env.user.pubkey(),
            &[],
            5_000_000,
            DECIMALS,
        )
        .unwrap(),
    ];
    let user = env.user.insecure_clone();
    env.send(&ixs, &[&user]).await.unwrap();

    assert!(env.redeem_set(&series, 10_000_000).await.is_err());
    env.redeem_set(&series, 5_000_000).await.unwrap();
    env.assert_invariants(&series).await;
}

#[tokio::test]
async fn only_admin_creates_series_and_only_upgrade_authority_inits() {
    let mut env = Env::new().await;

    // A stranger can't initialise config.
    let stranger = Keypair::new();
    let fund =
        system_instruction::transfer(&env.ctx.payer.pubkey(), &stranger.pubkey(), 1_000_000_000);
    env.send(&[fund], &[]).await.unwrap();
    let ix = Instruction {
        program_id: segments::ID,
        accounts: segments::accounts::InitConfig {
            payer: stranger.pubkey(),
            config: config_pda(),
            program_data: program_data_pda(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: segments::instruction::InitConfig {
            admin: stranger.pubkey(),
            guardian: stranger.pubkey(),
            treasury: stranger.pubkey(),
        }
        .data(),
    };
    assert!(env.send(&[ix], &[&stranger]).await.is_err());

    env.init_config().await;
    let acc = env
        .ctx
        .banks_client
        .get_account(config_pda())
        .await
        .unwrap()
        .unwrap();
    let config = Config::try_deserialize(&mut acc.data.as_slice()).unwrap();
    assert_eq!(config.admin, env.admin.pubkey());

    let ix = env.create_series_ix(&stranger.pubkey(), 7);
    assert!(env.send(&[ix], &[&stranger]).await.is_err());
}
