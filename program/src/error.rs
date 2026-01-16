use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ABLError {
    InvalidInstruction,
    InvalidAuthority,
    AccountBlocked,
    NotEnoughAccounts,
    InvalidAccountData,
    InvalidSystemProgram,
    InvalidGatingProgram,
    InvalidConfigAccount,
    AccountNotWritable,
    InvalidExtraMetasAccount,
    ImmutableOwnerExtensionMissing,
    InvalidData,
    InvalidTokenAclMintConfig,
    ListNotEmpty,
    InvalidRemainingAccounts,
    InvalidWalletEntry,
}

impl From<ABLError> for ProgramError {
    fn from(e: ABLError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
