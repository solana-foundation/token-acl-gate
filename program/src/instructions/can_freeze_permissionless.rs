use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::{load, ABLError, ListConfig, WalletEntry as WalletEntryState};

/// SECURITY ASSUMPTIONS OVER CAN FREEZE PERMISSIONLESS EXECUTION
///
/// 1- it is called by the token-acl program
/// 2- if some other program is calling it, we don't care as we don't write state here
/// 3- its inputs are already sanitized by the token-acl program
/// 4- if some other program is calling it with invalid inputs, we don't care as we only read state and return ok/nok
/// 5- given all the above we can skip a lot of type and owner checks
pub struct CanFreezePermissionless<'a> {
    pub authority: &'a AccountInfo,
    pub token_account: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub owner: &'a AccountInfo,
    pub extra_metas: &'a AccountInfo,
    pub remaining_accounts: &'a [AccountInfo],
}

/// Outcome of a freeze request for a single (list, wallet) pair
enum FreezeDecision {
    Freeze,
    Skip,
}

/// Whether a valid wallet entry exists for a given (list, wallet) pair
enum WalletEntry {
    /// A matching, validated entry exists for this list and wallet
    Present,
    /// No entry exists (account uninitialized)
    Missing,
}

impl<'a> CanFreezePermissionless<'a> {
    pub const DISCRIMINATOR: u8 = 0xd6;

    pub fn process(&self) -> ProgramResult {
        // Remaining accounts should be pairs of list and ab_wallet
        // Freeze logic is the inverse of thaw:
        // - thaw requires every list to approve,
        // - freeze succeeds as soon as any list approves
        let mut remaining_accounts = self.remaining_accounts.iter();
        while let Some(list) = remaining_accounts.next() {
            let wallet_entry = remaining_accounts.next().unwrap();
            match CanFreezePermissionless::validate_freeze_list(list, wallet_entry) {
                Ok(FreezeDecision::Freeze) => return Ok(()),
                Ok(FreezeDecision::Skip) => {}
                Err(err) => {
                    pinocchio_log::log!("Failed to pass validation for list {}", list.key());
                    return Err(err);
                }
            }
        }

        Err(ABLError::AccountAllowed.into())
    }

    fn validate_freeze_list(
        list: &AccountInfo,
        wallet_entry: &AccountInfo,
    ) -> Result<FreezeDecision, ProgramError> {
        if !list.is_owned_by(&crate::ID) {
            return Err(ABLError::InvalidListConfig.into());
        }

        let list_data: &[u8] = &list.try_borrow_data()?;
        let list_config = load::<ListConfig>(list_data)?;

        // Freeze semantics are the inverse of thaw:
        // - Allow: freeze if wallet is NOT on the list
        // - Block: freeze if wallet IS on the list
        match list_config.get_mode() {
            crate::Mode::Allow => Ok(
                match Self::check_wallet_entry_state(list.key(), wallet_entry)? {
                    WalletEntry::Present => FreezeDecision::Skip,
                    WalletEntry::Missing => FreezeDecision::Freeze,
                },
            ),
            crate::Mode::Block => Ok(
                match Self::check_wallet_entry_state(list.key(), wallet_entry)? {
                    WalletEntry::Present => FreezeDecision::Freeze,
                    WalletEntry::Missing => FreezeDecision::Skip,
                },
            ),
            _ => Err(ABLError::InvalidListConfig.into()),
        }
    }

    fn check_wallet_entry_state(
        list_config: &Pubkey,
        wallet_entry: &AccountInfo,
    ) -> Result<WalletEntry, ProgramError> {
        // either the list exists and is owned by this program or it doesn't exist.
        // by checking owners, we can avoid expensive PDA derivation.
        if !wallet_entry.is_owned_by(&Pubkey::default()) && !wallet_entry.is_owned_by(&crate::ID) {
            return Err(ABLError::InvalidWalletEntry.into());
        }

        let ab_wallet_data: &[u8] = &wallet_entry.try_borrow_data()?;
        let res = load::<WalletEntryState>(ab_wallet_data);

        match res {
            Ok(wallet) => {
                if wallet.list_config.ne(list_config) {
                    return Err(ABLError::InvalidWalletEntry.into());
                }
                Ok(WalletEntry::Present)
            }
            Err(_) => Ok(WalletEntry::Missing),
        }
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for CanFreezePermissionless<'a> {
    type Error = ABLError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, token_account, mint, owner, _flag_account, extra_metas, remaining_accounts @ ..] =
            accounts
        else {
            return Err(ABLError::NotEnoughAccounts);
        };

        if remaining_accounts.len() % 2 != 0 {
            return Err(ABLError::InvalidRemainingAccounts);
        }

        Ok(Self {
            authority,
            token_account,
            mint,
            owner,
            extra_metas,
            remaining_accounts,
        })
    }
}
