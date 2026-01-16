use pinocchio::{account_info::AccountInfo, ProgramResult};

use crate::{load, ABLError, ListConfig};

pub struct DeleteList<'a> {
    pub authority: &'a AccountInfo,
    pub list_config: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for DeleteList<'a> {
    type Error = ABLError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, list_config] = accounts else {
            return Err(ABLError::NotEnoughAccounts);
        };

        if !list_config.is_owned_by(&crate::ID) {
            return Err(ABLError::InvalidConfigAccount);
        }

        if !authority.is_signer() || !authority.is_writable() {
            return Err(ABLError::InvalidAuthority);
        }

        Ok(Self {
            authority,
            list_config,
        })
    }
}

impl<'a> DeleteList<'a> {
    pub const DISCRIMINATOR: u8 = 0x05;

    pub fn process(&self) -> ProgramResult {
        {
            let list_config =
                unsafe { load::<ListConfig>(self.list_config.borrow_data_unchecked())? };

            if list_config.authority.ne(self.authority.key()) {
                return Err(ABLError::InvalidAuthority.into());
            }

            if list_config.get_wallets_count() > 0 {
                return Err(ABLError::ListNotEmpty.into());
            }
        }

        let list_config_lamports = unsafe { self.list_config.borrow_mut_lamports_unchecked() };
        let authority_lamports = unsafe { self.authority.borrow_mut_lamports_unchecked() };

        *authority_lamports += *list_config_lamports;

        // close will set lamports to 0
        self.list_config.close()?;

        Ok(())
    }
}
