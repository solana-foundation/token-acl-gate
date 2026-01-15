use pinocchio::{program_error::ProgramError, pubkey::Pubkey, ProgramResult};

use super::{Discriminator, Transmutable};

#[repr(C)]
pub struct ListConfig {
    pub discriminator: u8,
    pub authority: Pubkey,
    pub seed: Pubkey,
    pub mode: u8,
    pub wallets_count: [u8; 8],
}

impl ListConfig {
    pub const SEED_PREFIX: &'static [u8] = b"list_config";

    pub fn get_mode(&self) -> Mode {
        match self.mode {
            0 => Mode::Allow,
            1 => Mode::AllowAllEoas,
            _ => Mode::Block,
        }
    }

    pub fn get_wallets_count(&self) -> u64 {
        u64::from_le_bytes(self.wallets_count)
    }

    pub fn increment_wallets_count(&mut self) -> ProgramResult {
        self.wallets_count = self
            .get_wallets_count()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .to_le_bytes();
        Ok(())
    }

    pub fn decrement_wallets_count(&mut self) -> ProgramResult {
        self.wallets_count = self
            .get_wallets_count()
            .checked_sub(1)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .to_le_bytes();
        Ok(())
    }
}

impl Transmutable for ListConfig {
    const LEN: usize = 1 + 32 + 32 + 8 + 1;
}

impl Discriminator for ListConfig {
    const DISCRIMINATOR: u8 = 0x01;

    fn is_initialized(&self) -> bool {
        self.discriminator == Self::DISCRIMINATOR
    }
}

#[repr(u8)]
pub enum Mode {
    Allow,
    AllowAllEoas,
    Block,
}
