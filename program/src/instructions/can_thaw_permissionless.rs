use pinocchio::{ProgramResult, account_info::AccountInfo, pubkey::Pubkey};

use crate::{load, ABLError, ListConfig, WalletEntry};

use solana_curve25519::edwards::PodEdwardsPoint;


/// SECURITY ASSUMPTIONS OVER CAN THAW PERMISSIONLESS EXECUTION
///
/// 1- its called by the token-2022 program
/// 2- if some other program is calling it, we don't care as we don't write state here
/// 2- its inputs are already sanitized by the token-2022 program
/// 3- if some other program is calling it with invalid inputs, we don't care as we only read state and return ok/nok
/// 4- given all the above we can skip a lot of type and owner checks
pub struct CanThawPermissionless<'a> {
    pub authority: &'a AccountInfo,
    pub token_account: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub owner: &'a AccountInfo,
    pub extra_metas: &'a AccountInfo,
    pub remaining_accounts: &'a [AccountInfo],
}

impl<'a> CanThawPermissionless<'a> {
    pub const DISCRIMINATOR: u8 = 0x8;

    pub fn process(&self) -> ProgramResult {
        // SAFETY: token account is validated by the token-2022 program
        // after the current call finishes execution, the token acl program
        // calls into token-2022 to thaw the token account, which gets type checked
        // by the token-2022 program
        if !crate::state::has_immutable_owner_extension(self.token_account) {
            return Err(ABLError::ImmutableOwnerExtensionMissing.into());
        }

        // remaining accounts should be pairs of list and ab_wallet
        let mut remaining_accounts = self.remaining_accounts.iter();
        while let Some(list) = remaining_accounts.next() {
            let ab_wallet = remaining_accounts.next().unwrap();

            CanThawPermissionless::validate_thaw_list(list, self.owner, ab_wallet).inspect_err(
                |_| {
                    pinocchio_log::log!("Failed to pass validation for list {}", list.key());
                },
            )?;
        }

        Ok(())
    }

    fn validate_thaw_list(
        list: &AccountInfo,
        owner: &AccountInfo,
        wallet_entry: &AccountInfo,
    ) -> ProgramResult {

        if !list.is_owned_by(&crate::ID) {
            return Err(ABLError::InvalidListConfig.into());
        }

        let list_data: &[u8] = &list.try_borrow_data()?;
        let list_config = unsafe { load::<ListConfig>(list_data)? };

        // 3 operation modes
        // allow: only wallets that have been allowlisted can thaw, requires previously created ABWallet account
        // block: only wallets that have been blocklisted can't thaw, thawing requires ABWallet to not exist
        // allow with permissionless eoas: all wallets that can sign can thaw, otherwise requires previously created ABWallet account (for PDAs)
        match list_config.get_mode() {
            crate::Mode::Allow => {
                let ab_wallet_data: &[u8] = &wallet_entry.try_borrow_data()?;
                let wallet = unsafe {
                    load::<WalletEntry>(ab_wallet_data).map_err(|_| ABLError::AccountBlocked)?
                };

                if !wallet_entry.is_owned_by(&crate::ID) || wallet.list_config.ne(list.key()) {
                    return Err(ABLError::InvalidWalletEntry.into());
                }

                Ok(())
            }
            crate::Mode::AllowAllEoas => {
                let pt = PodEdwardsPoint(*owner.key());

                if !solana_curve25519::edwards::validate_edwards(&pt) {
                    let ab_wallet_data: &[u8] = &wallet_entry.try_borrow_data()?;
                    let wallet = unsafe {
                        load::<WalletEntry>(ab_wallet_data).map_err(|_| ABLError::AccountBlocked)?
                    };
                    
                    if !wallet_entry.is_owned_by(&crate::ID) || wallet.list_config.ne(list.key()) {
                        return Err(ABLError::InvalidWalletEntry.into());
                    }
                }

                Ok(())
            }
            crate::Mode::Block => {
                let ab_wallet_data: &[u8] = &wallet_entry.try_borrow_data()?;
                let res = unsafe { load::<WalletEntry>(ab_wallet_data) };

                // either the block exists and is owned by this program
                // or it doest exist. We want to avoid PDA derivation to waste more CUs
                if !wallet_entry.is_owned_by(&Pubkey::default()) && !wallet_entry.is_owned_by(&crate::ID) {
                    return Err(ABLError::InvalidWalletEntry.into());
                }

                if let Ok(wallet) = res {
                    if wallet.list_config.ne(list.key()) {
                        return Err(ABLError::InvalidWalletEntry.into());
                    }
                    Err(ABLError::AccountBlocked.into())
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl<'a> TryFrom<&'a [AccountInfo]> for CanThawPermissionless<'a> {
    type Error = ABLError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        /*
        GATE PROGRAM GETS CALLED WITH:
         1- authority
         2- token account
         3- mint
         4- owner
         5- flag account
         6- extra account metas
         (remaining accounts are pairs of list and wallet)
         */

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
