pub mod add_wallet;
pub mod can_freeze_permissionless;
pub mod can_thaw_permissionless;
pub mod create_list;
pub mod delete_list;
pub mod remove_wallet;
pub mod setup_extra_metas;
pub mod setup_freeze_extra_metas;

pub use add_wallet::*;
pub use can_freeze_permissionless::*;
pub use can_thaw_permissionless::*;
pub use create_list::*;
pub use delete_list::*;
pub use remove_wallet::*;
pub use setup_extra_metas::*;
pub use setup_freeze_extra_metas::*;

use codama::CodamaInstructions;
use pinocchio::pubkey::Pubkey;

use crate::Mode;

#[derive(CodamaInstructions)]
#[repr(C, u8)]
pub enum Instruction {
    /// Creates a new list configuration account
    #[codama(account(name = "authority", signer))]
    #[codama(account(name = "payer", signer, writable))]
    #[codama(account(name = "list_config", writable))]
    #[codama(account(name = "system_program", default_value = program("system")))]
    CreateList { mode: Mode, seed: Pubkey } = 1,

    /// Adds a wallet to a list
    #[codama(account(name = "authority", signer))]
    #[codama(account(name = "payer", signer, writable))]
    #[codama(account(name = "list_config", writable))]
    #[codama(account(name = "wallet"))]
    #[codama(account(name = "wallet_entry", writable))]
    #[codama(account(name = "system_program", default_value = program("system")))]
    AddWallet {} = 2,

    /// Removes a wallet from a list
    #[codama(account(name = "authority", signer, writable))]
    #[codama(account(name = "list_config", writable))]
    #[codama(account(name = "wallet_entry", writable))]
    RemoveWallet {} = 3,

    /// Sets up extra account metadata for token-2022 integration.
    ///
    /// Remaining accounts: up to 5 `ListConfig` accounts owned by this program.
    #[codama(account(name = "authority", signer))]
    #[codama(account(name = "payer", signer, writable))]
    #[codama(account(name = "token_acl_mint_config"))]
    #[codama(account(name = "mint"))]
    #[codama(account(name = "extra_metas", writable))]
    #[codama(account(name = "system_program", default_value = program("system")))]
    SetupExtraMetas {} = 4,

    /// Closes a list configuration account
    #[codama(account(name = "authority", signer, writable))]
    #[codama(account(name = "list_config", writable))]
    DeleteList {} = 5,

    /// Sets up freeze extra account metadata for token-acl integration.
    ///
    /// Remaining accounts: up to 5 `ListConfig` accounts owned by this program.
    #[codama(account(name = "authority", signer))]
    #[codama(account(name = "payer", signer, writable))]
    #[codama(account(name = "token_acl_mint_config"))]
    #[codama(account(name = "mint"))]
    #[codama(account(name = "extra_metas", writable))]
    #[codama(account(name = "system_program", default_value = program("system")))]
    SetupFreezeExtraMetas {} = 6,

    /// Validates whether a wallet's token account can be thawed. This program uses 1 byte
    /// instruction discriminators. The token-acl interface defines an 8 byte discriminator,
    /// where 0x08 is the first byte.
    ///
    /// Remaining accounts: pairs of (`ListConfig`, `WalletEntry`) accounts.
    #[codama(account(name = "authority"))]
    #[codama(account(name = "token_account"))]
    #[codama(account(name = "mint"))]
    #[codama(account(name = "owner"))]
    #[codama(account(name = "flag_account"))]
    #[codama(account(name = "extra_metas"))]
    CanThawPermissionless {} = 8,

    /// Validates whether a wallet's token account can be frozen. This program uses 1 byte
    /// instruction discriminators. The token-acl interface defines an 8 byte discriminator,
    /// where 0xd6 is the first byte.
    ///
    /// Remaining accounts: pairs of (`ListConfig`, `WalletEntry`) accounts.
    #[codama(account(name = "authority"))]
    #[codama(account(name = "token_account"))]
    #[codama(account(name = "mint"))]
    #[codama(account(name = "owner"))]
    #[codama(account(name = "flag_account"))]
    #[codama(account(name = "extra_metas"))]
    CanFreezePermissionless {} = 214,
}
