use anchor_lang::prelude::*;

#[error_code]
pub enum SegmentsError {
    #[msg("Fee exceeds the hard cap")]
    FeeTooHigh,
    #[msg("No fee change is pending")]
    NoPendingFee,
    #[msg("The fee timelock has not elapsed")]
    FeeTimelockActive,
    #[msg("Series is already finalized and cannot be changed")]
    SeriesFinalized,
    #[msg("Series is not finalized yet")]
    SeriesNotFinalized,
    #[msg("Series has too many partials")]
    TooManyPartials,
    #[msg("Series needs at least two partials")]
    TooFewPartials,
    #[msg("Partial index must equal the current partial count")]
    BadPartialIndex,
    #[msg("Partial weights must sum to 10,000 bps")]
    BadWeights,
    #[msg("Minting is paused for this series")]
    MintPaused,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Amount is too small to cover the fee")]
    AmountTooSmall,
    #[msg("Wrong number of partial accounts")]
    WrongPartialAccounts,
    #[msg("Partial mint does not match the series")]
    WrongPartialMint,
    #[msg("Token account has the wrong mint or owner")]
    WrongTokenAccount,
    #[msg("String is too long")]
    StringTooLong,
    #[msg("Signer is not allowed to do this")]
    Unauthorized,
    #[msg("No admin transfer is pending")]
    NoPendingAdmin,
    #[msg("Arithmetic overflow")]
    MathOverflow,
}
