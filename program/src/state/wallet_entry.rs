use codama::CodamaAccount;
use pinocchio::pubkey::Pubkey;

use super::{Discriminator, Transmutable};

#[derive(CodamaAccount)]
#[repr(C)]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string, value = "wallet_entry"))]
#[codama(seed(name = "list_config", type = public_key))]
#[codama(seed(name = "wallet", type = public_key))]
pub struct WalletEntry {
    #[codama(default_value = 2)]
    pub discriminator: u8,
    pub wallet_address: Pubkey,
    pub list_config: Pubkey,
}

impl WalletEntry {
    pub const SEED_PREFIX: &'static [u8] = b"wallet_entry";
}

impl Transmutable for WalletEntry {
    const LEN: usize = 1 + 32 + 32;
}

impl Discriminator for WalletEntry {
    const DISCRIMINATOR: u8 = 0x02;

    fn is_initialized(&self) -> bool {
        self.discriminator == Self::DISCRIMINATOR
    }
}
