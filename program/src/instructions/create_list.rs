use pinocchio::{
    account_info::AccountInfo,
    instruction::Signer,
    pubkey::find_program_address,
    seeds,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

use crate::{load_mut_unchecked, ABLError, Discriminator, ListConfig, Transmutable};

pub struct CreateList<'a> {
    pub authority: &'a AccountInfo,
    pub list_config: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for CreateList<'a> {
    type Error = ABLError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, list_config, system_program] = accounts else {
            return Err(ABLError::NotEnoughAccounts);
        };

        // check if system program is valid
        if system_program.key().ne(&pinocchio_system::ID) {
            return Err(ABLError::InvalidSystemProgram);
        }

        if !authority.is_signer() {
            return Err(ABLError::InvalidAuthority.into());
        }

        Ok(Self {
            authority,
            list_config,
            system_program,
        })
    }
}

impl<'a> CreateList<'a> {
    pub const DISCRIMINATOR: u8 = 0x01;

    pub fn process(&self, remaining_data: &[u8]) -> ProgramResult {
        let [mode, seed @ ..] = remaining_data else {
            return Err(ABLError::InvalidData.into());
        };

        if seed.len() != 32 {
            return Err(ABLError::InvalidData.into());
        }

        if *mode > 2u8 {
            return Err(ABLError::InvalidData.into());
        }

        let lamports = Rent::get()?.minimum_balance(ListConfig::LEN);

        // find canonical bump to prepare signer seeds for cpi
        let seed = TryInto::<&[u8; 32]>::try_into(seed).unwrap();
        let (_, config_bump) = find_program_address(
            &[ListConfig::SEED_PREFIX, self.authority.key(), seed],
            &crate::ID,
        );

        // prepare signer seeds for cpi
        let bump_seed = [config_bump];
        let seeds = seeds!(
            ListConfig::SEED_PREFIX,
            self.authority.key(),
            seed,
            &bump_seed
        );
        let signer = Signer::from(&seeds);

        pinocchio_system::instructions::CreateAccount {
            from: self.authority,
            to: self.list_config,
            lamports,
            space: ListConfig::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&[signer])?;

        let mut data = self.list_config.try_borrow_mut_data()?;
        let list = unsafe { load_mut_unchecked::<ListConfig>(&mut data)? };
        list.discriminator = ListConfig::DISCRIMINATOR;
        list.authority = *self.authority.key();
        list.seed = *seed;
        list.mode = *mode;

        Ok(())
    }
}
