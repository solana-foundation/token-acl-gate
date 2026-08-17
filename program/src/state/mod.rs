pub mod list_config;
pub mod wallet_entry;
pub use list_config::*;
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_token_2022_interface::{
    extension::{immutable_owner::ImmutableOwner, BaseStateWithExtensions, PodStateWithExtensions},
    pod::PodAccount,
};
pub use wallet_entry::*;

use crate::ABLError;

pub trait Transmutable {
    const LEN: usize;
}

pub trait Discriminator {
    const DISCRIMINATOR: u8;

    fn is_initialized(&self) -> bool;
}

macro_rules! assert_pod_layout {
    ($ty:ty) => {
        const _: () = {
            assert!(core::mem::size_of::<$ty>() == <$ty as $crate::Transmutable>::LEN);
            assert!(core::mem::align_of::<$ty>() == 1);
        };
    };
}
pub(crate) use assert_pod_layout;

/// Return a reference for an initialized `T` from the given bytes.
#[inline(always)]
pub fn load<T: bytemuck::Pod + Discriminator + Transmutable>(bytes: &[u8]) -> Result<&T, ABLError> {
    load_unchecked(bytes).and_then(|t: &T| {
        // checks if the data is initialized
        if t.is_initialized() {
            Ok(t)
        } else {
            Err(ABLError::InvalidAccountData)
        }
    })
}

/// Return a mutable reference for an initialized `T` from the given bytes.
#[inline(always)]
pub fn load_mut<T: bytemuck::Pod + Discriminator + Transmutable>(
    bytes: &mut [u8],
) -> Result<&mut T, ABLError> {
    load_mut_unchecked(bytes).and_then(|t: &mut T| {
        // checks if the data is initialized
        if t.is_initialized() {
            Ok(t)
        } else {
            Err(ABLError::InvalidAccountData)
        }
    })
}

/// Return a `T` reference from the given bytes.
///
/// This function does not check if the data is initialized.
#[inline(always)]
pub fn load_unchecked<T: bytemuck::Pod + Transmutable>(bytes: &[u8]) -> Result<&T, ABLError> {
    if bytes.len() != T::LEN {
        return Err(ABLError::InvalidAccountData);
    }
    bytemuck::try_from_bytes(bytes).map_err(|_| ABLError::InvalidAccountData)
}

/// Return a mutable `T` reference from the given bytes.
///
/// This function does not check if the data is initialized.
#[inline(always)]
pub fn load_mut_unchecked<T: bytemuck::Pod + Transmutable>(
    bytes: &mut [u8],
) -> Result<&mut T, ABLError> {
    if bytes.len() != T::LEN {
        return Err(ABLError::InvalidAccountData);
    }
    bytemuck::try_from_bytes_mut(bytes).map_err(|_| ABLError::InvalidAccountData)
}

pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    pinocchio_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Checks if the account is a token account with the immutable owner extension.
#[inline(always)]
pub fn has_immutable_owner_extension(token_account: &AccountInfo) -> Result<bool, ProgramError> {
    if !token_account.is_owned_by(&TOKEN_2022_PROGRAM_ID) {
        return Err(ABLError::InvalidAccountData.into());
    }

    let data = &token_account.try_borrow_data()?;
    let account = PodStateWithExtensions::<PodAccount>::unpack(data)
        .map_err(|_| ABLError::InvalidAccountData)?;

    Ok(account.get_extension::<ImmutableOwner>().is_ok())
}
