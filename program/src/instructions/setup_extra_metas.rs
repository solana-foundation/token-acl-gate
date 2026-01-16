use pinocchio::{
    account_info::AccountInfo, instruction::Signer, pubkey::{find_program_address, Pubkey}, seeds, syscalls::sol_memset_, sysvars::{rent::Rent, Sysvar}, ProgramResult
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, solana_pubkey::Pubkey as SolanaPubkey,
    state::ExtraAccountMetaList,
};

use crate::{load, ABLError, ListConfig, WalletEntry};

pub struct SetupExtraMetas<'a> {
    pub authority: &'a AccountInfo,
    pub payer: &'a AccountInfo,
    pub token_acl_mint_config: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub extra_metas: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub remaining_accounts: &'a [AccountInfo],
    pub extra_metas_bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for SetupExtraMetas<'a> {
    type Error = ABLError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, payer, token_acl_mint_config, mint, extra_metas, system_program, remaining_accounts @ ..] =
            accounts
        else {
            return Err(ABLError::NotEnoughAccounts);
        };

        if !authority.is_signer() {
            return Err(ABLError::InvalidAuthority);
        }

        // derive extra_metas account
        let (extra_metas_address, extra_metas_bump) = find_program_address(
            &[token_acl_interface::THAW_EXTRA_ACCOUNT_METAS_SEED, mint.key()],
            &crate::ID,
        );
        // need to check because we cannot rely on system program create instruction
        // as the account may already be initialized
        if extra_metas_address.ne(extra_metas.key()) {
            return Err(ABLError::InvalidExtraMetasAccount);
        }

        if !token_acl_mint_config.is_owned_by(token_acl_client::programs::TOKEN_ACL_ID.as_array()) {
            return Err(ABLError::InvalidConfigAccount);
        }

        // check if system program is valid
        if system_program.key().ne(&pinocchio_system::ID) {
            return Err(ABLError::InvalidSystemProgram);
        }

        Ok(Self {
            authority,
            payer,
            token_acl_mint_config,
            mint,
            extra_metas,
            system_program,
            remaining_accounts,
            extra_metas_bump,
        })
    }
}

impl<'a> SetupExtraMetas<'a> {
    pub const DISCRIMINATOR: u8 = 0x04;

    pub fn process(&self) -> ProgramResult {
        let mint_config_data = self.token_acl_mint_config.try_borrow_data()?;
        let mint_config = token_acl::state::load_mint_config(&mint_config_data)
        .map_err(|_| ABLError::InvalidTokenAclMintConfig)?;
    
        // only the selected freeze authority should be able to set the extra metas
        if mint_config.mint.as_array() != self.mint.key()
        || mint_config.freeze_authority.as_array() != self.authority.key()
        {
            return Err(ABLError::InvalidAuthority.into());
        }

        if mint_config.gating_program.as_array() != &crate::ID {
            return Err(ABLError::InvalidGatingProgram.into());
        }
        
        if self.remaining_accounts.len() > 5 {
            return Err(ABLError::InvalidData.into());
        }
        
        let mut lists = [Option::<&Pubkey>::None; 5];
        let mut i = 0;
        for account in self.remaining_accounts {
            if !account.is_owned_by(&crate::ID) {
                return Err(ABLError::InvalidConfigAccount.into());
            }
            let _ = unsafe { load::<ListConfig>(&account.try_borrow_data()?)? };
            lists[i] = Some(account.key());
            i += 1;
        }
        
        let lists_slice = &lists[..i];
        
        let data_len = get_extra_metas_size(lists_slice);
        let min_lamports = Rent::get()?.minimum_balance(data_len);
        
        if self.extra_metas.is_owned_by(&crate::ID) {
            let current_lamports = self.extra_metas.lamports();
            let payer_lamports = self.payer.lamports();
            
            // just resize and set everything to 0
            self.extra_metas.resize(data_len)?;
            unsafe {
                sol_memset_(
                    self.extra_metas.borrow_mut_data_unchecked().as_mut_ptr(),
                    0,
                    data_len as u64,
                );
            }
            
            if current_lamports < min_lamports {
                // transfer to extra
                let diff = min_lamports - current_lamports;
                pinocchio_system::instructions::Transfer {
                    from: self.payer,
                    to: self.extra_metas,
                    lamports: diff,
                }
                .invoke()?;
            } else if current_lamports > min_lamports {
                // transfer from extra
                let diff = current_lamports - min_lamports;
                unsafe {
                    *self.extra_metas.borrow_mut_lamports_unchecked() = min_lamports;
                    *self.payer.borrow_mut_lamports_unchecked() =
                    payer_lamports.checked_add(diff).unwrap();
                }
            }
        } else {
            // create new account
            let bump_seed = [self.extra_metas_bump];
            let seeds = seeds!(
                token_acl_interface::THAW_EXTRA_ACCOUNT_METAS_SEED,
                self.mint.key(),
                &bump_seed
            );
            let signer = [Signer::from(&seeds)];

            let current_lamports = self.extra_metas.lamports();
            if current_lamports < min_lamports {
                // transfer
                pinocchio_system::instructions::Transfer {
                    from: self.payer,
                    to: self.extra_metas,
                    lamports: min_lamports - current_lamports,
                }
                .invoke()?;
            }

            // allocate
            pinocchio_system::instructions::Allocate {
                account: self.extra_metas,
                space: data_len as u64,
            }
            .invoke_signed(&signer)?;

            // assign
            pinocchio_system::instructions::Assign {
                account: self.extra_metas,
                owner: &crate::ID,
            }
            .invoke_signed(&signer)?;
        }

        let mut extra_metas_data = self.extra_metas.try_borrow_mut_data()?;
        let (metas, len) = get_extra_metas(lists_slice);

        ExtraAccountMetaList::init::<token_acl_interface::instruction::CanThawPermissionlessInstruction>(&mut extra_metas_data, &metas[..len]).unwrap();
        Ok(())
    }
}

fn get_extra_metas(lists: &[Option<&Pubkey>]) -> ([ExtraAccountMeta; 10], usize) {
    let mut metas = [ExtraAccountMeta::default(); 10];

    let mut index: usize = 0;
    for list in lists {
        metas[index] = ExtraAccountMeta::new_with_pubkey(
            &SolanaPubkey::new_from_array(*list.unwrap()),
            false,
            false,
        )
        .unwrap();
        metas[index + 1] = ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: WalletEntry::SEED_PREFIX.to_vec(),
                },
                Seed::AccountKey {
                    index: index as u8 + 6,
                },
                Seed::AccountData {
                    account_index: 1, // token account
                    data_index: 32,   // ta owner
                    length: 32,
                },
            ],
            false,
            false,
        )
        .unwrap();
        index += 2;
    }

    (metas, index)
}

fn get_extra_metas_size(lists: &[Option<&Pubkey>]) -> usize {
    ExtraAccountMetaList::size_of(2 * lists.len()).unwrap()
}
