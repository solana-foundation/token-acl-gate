use pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult};
use solana_curve25519::edwards::PodEdwardsPoint;

use crate::{load, ABLError, ListConfig, WalletEntry};

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

impl<'a> CanFreezePermissionless<'a> {
    pub const DISCRIMINATOR: u8 = 0xd6;

    pub fn process(&self) -> ProgramResult {
        // SAFETY: token account is validated by the token-2022 program
        // after the current call finishes execution, the token acl program
        // calls into token-2022 to freeze the token account, which gets type checked
        // by the token-2022 program
        if !crate::state::has_immutable_owner_extension(self.token_account) {
            return Err(ABLError::ImmutableOwnerExtensionMissing.into());
        }

        // remaining accounts should be pairs of list and ab_wallet
        let mut remaining_accounts = self.remaining_accounts.iter();
        while let Some(list) = remaining_accounts.next() {
            let wallet_entry = remaining_accounts.next().unwrap();
            CanFreezePermissionless::validate_freeze_list(list, self.owner, wallet_entry)?;
        }

        Ok(())
    }

    fn validate_freeze_list(
        list: &AccountInfo,
        owner: &AccountInfo,
        wallet_entry: &AccountInfo,
    ) -> ProgramResult {
        if !list.is_owned_by(&crate::ID) {
            return Err(ABLError::InvalidListConfig.into());
        }

        let list_data: &[u8] = &list.try_borrow_data()?;
        let list_config = unsafe { load::<ListConfig>(list_data)? };

        // Freeze semantics are the inverse of thaw:
        // - Allow: freeze if wallet is NOT on the list
        // - AllowAllEoas: freeze if wallet is not an EOA AND not on the list
        // - Block: freeze if wallet IS on the list
        match list_config.get_mode() {
            crate::Mode::Allow => {
                Self::require_missing_allowlist_wallet_entry(list.key(), owner.key(), wallet_entry)
            }
            crate::Mode::AllowAllEoas => {
                let pt = PodEdwardsPoint(*owner.key());
                if solana_curve25519::edwards::validate_edwards(&pt) {
                    return Err(ABLError::InvalidWalletEntry.into());
                }

                Self::require_missing_allowlist_wallet_entry(list.key(), owner.key(), wallet_entry)
            }
            crate::Mode::Block => {
                let ab_wallet_data: &[u8] = &wallet_entry.try_borrow_data()?;
                let wallet = unsafe {
                    load::<WalletEntry>(ab_wallet_data).map_err(|_| ABLError::InvalidWalletEntry)?
                };

                if !wallet_entry.is_owned_by(&crate::ID) || wallet.list_config.ne(list.key()) {
                    return Err(ABLError::InvalidWalletEntry.into());
                }

                Ok(())
            }
        }
    }

    fn require_missing_allowlist_wallet_entry(
        list_config: &Pubkey,
        owner: &Pubkey,
        wallet_entry: &AccountInfo,
    ) -> ProgramResult {
        // either the list exists and is owned by this program or it doest exist.
        // by checking owners, we can avoid expensive PDA derivation.
        if !wallet_entry.is_owned_by(&Pubkey::default()) && !wallet_entry.is_owned_by(&crate::ID) {
            return Err(ABLError::InvalidWalletEntry.into());
        }

        let ab_wallet_data: &[u8] = &wallet_entry.try_borrow_data()?;
        let res = unsafe { load::<WalletEntry>(ab_wallet_data) };

        match res {
            // A loadable wallet entry means the account is initialized. For allowlists,
            // that entry approves a target wallet. If the list and wallet match the provided
            // values, then the wallet is approved and should not be frozen.
            Ok(wallet) => {
                if wallet.list_config.ne(list_config) || wallet.wallet_address.ne(owner) {
                    return Err(ABLError::InvalidWalletEntry.into());
                }
                Err(ABLError::AccountAllowed.into())
            }
            // A missing account means that there is no allowlist entry for a given wallet.
            // We can skip deriving the wallet_entry PDA because the previously setup
            // extra metas via the freeze authority doesn't allow dynamic account injection.
            Err(_) => Ok(()),
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
