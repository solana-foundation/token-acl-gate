use codama::CodamaErrors;
use pinocchio::program_error::ProgramError;

/// Errors returned by the token-acl gate program.
///
/// Errors start at 4096 (0x1000) to prevent overlapping with token-acl's
/// own error codes starting at 0.
#[derive(Clone, Debug, Eq, PartialEq, CodamaErrors)]
pub enum ABLError {
    #[codama(error("Invalid instruction discriminator"))]
    InvalidInstruction = 4096,
    #[codama(error("Invalid authority"))]
    InvalidAuthority = 4097,
    #[codama(error("Account is blocked by a list"))]
    AccountBlocked = 4098,
    #[codama(error("Not enough accounts provided"))]
    NotEnoughAccounts = 4099,
    #[codama(error("Invalid account data"))]
    InvalidAccountData = 4100,
    #[codama(error("Invalid system program"))]
    InvalidSystemProgram = 4101,
    #[codama(error("Invalid gating program"))]
    InvalidGatingProgram = 4102,
    #[codama(error("Invalid config account"))]
    InvalidConfigAccount = 4103,
    #[codama(error("Account is not writable"))]
    AccountNotWritable = 4104,
    #[codama(error("Invalid extra account metas account"))]
    InvalidExtraMetasAccount = 4105,
    #[codama(error("Token account is missing the immutable owner extension"))]
    ImmutableOwnerExtensionMissing = 4106,
    #[codama(error("Invalid instruction data"))]
    InvalidData = 4107,
    #[codama(error("Invalid token-acl mint config account"))]
    InvalidTokenAclMintConfig = 4108,
    #[codama(error("List still contains wallet entries"))]
    ListNotEmpty = 4109,
    #[codama(error("Invalid remaining accounts"))]
    InvalidRemainingAccounts = 4110,
    #[codama(error("Invalid wallet entry account"))]
    InvalidWalletEntry = 4111,
    #[codama(error("Invalid list config account"))]
    InvalidListConfig = 4112,
    #[codama(error("Account is allowed by every list"))]
    AccountAllowed = 4113,
}

impl From<ABLError> for ProgramError {
    fn from(e: ABLError) -> Self {
        ProgramError::Custom(e as u32)
    }
}