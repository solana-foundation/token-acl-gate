pub mod list_config;
pub mod wallet_entry;
pub use list_config::*;
use pinocchio::account_info::AccountInfo;
pub use wallet_entry::*;

use crate::ABLError;

pub trait Transmutable {
    const LEN: usize;
}

pub trait Discriminator {
    const DISCRIMINATOR: u8;

    fn is_initialized(&self) -> bool;
}

/// Return a reference for an initialized `T` from the given bytes.
///
/// # Safety
///
/// The caller must ensure that `bytes` contains a valid representation of `T`.
#[inline(always)]
pub unsafe fn load<T: Discriminator + Transmutable>(bytes: &[u8]) -> Result<&T, ABLError> {
    load_unchecked(bytes).and_then(|t: &T| {
        // checks if the data is initialized
        if t.is_initialized() {
            Ok(t)
        } else {
            Err(ABLError::InvalidAccountData)
        }
    })
}

/// Return a reference for an initialized `T` from the given bytes.
///
/// # Safety
///
/// The caller must ensure that `bytes` contains a valid representation of `T`.
#[inline(always)]
pub unsafe fn load_mut<T: Discriminator + Transmutable>(
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
///
/// # Safety
///
/// The caller must ensure that `bytes` contains a valid representation of `T`.
#[inline(always)]
pub unsafe fn load_unchecked<T: Transmutable>(bytes: &[u8]) -> Result<&T, ABLError> {
    if bytes.len() != T::LEN {
        return Err(ABLError::InvalidAccountData);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

/// Return a mutable `T` reference from the given bytes.
///
/// This function does not check if the data is initialized.
///
/// # Safety
///
/// The caller must ensure that `bytes` contains a valid representation of `T`.
#[inline(always)]
pub unsafe fn load_mut_unchecked<T: Transmutable>(bytes: &mut [u8]) -> Result<&mut T, ABLError> {
    if bytes.len() != T::LEN {
        return Err(ABLError::InvalidAccountData);
    }
    Ok(&mut *(bytes.as_mut_ptr() as *mut T))
}

const IMMUTABLE_OWNER_EXTENSION_ID: u16 = 7;
const TOKEN_ACCOUNT_LEN: usize = 165;
const EXTENSION_START_PADDING: usize = 1;
const EXTENSION_LEN_BYTES_LEN: usize = 2;
const EXTENSION_TYPE_BYTES_LEN: usize = 2;
const EXTENSION_HEADER_LEN: usize = EXTENSION_LEN_BYTES_LEN + EXTENSION_TYPE_BYTES_LEN;
const EXTENSION_DATA_START_INDEX: usize = TOKEN_ACCOUNT_LEN + EXTENSION_START_PADDING;

/// Checks if the token account has the immutable owner extension
///
/// # Safety
///
/// The caller must ensure that `token_account` is a valid token account.)
#[inline(always)]
pub fn has_immutable_owner_extension(token_account: &AccountInfo) -> bool {
    let data = token_account.try_borrow_data();
    if data.is_err() {
        return false;
    }
    let data = data.unwrap();

    if data.len() < EXTENSION_DATA_START_INDEX {
        return false;
    }

    let extension_bytes = &data[EXTENSION_DATA_START_INDEX..];

    let mut start = 0;
    let end = extension_bytes.len();

    while start < end {
        let extension_type = u16::from_le_bytes(
            extension_bytes[start..start + EXTENSION_TYPE_BYTES_LEN]
                .try_into()
                .unwrap(),
        );
        if extension_type == IMMUTABLE_OWNER_EXTENSION_ID {
            return true;
        }

        let extension_len = u16::from_le_bytes(
            extension_bytes[start + EXTENSION_TYPE_BYTES_LEN..start + EXTENSION_HEADER_LEN]
                .try_into()
                .unwrap(),
        );
        start += EXTENSION_HEADER_LEN + extension_len as usize;
    }
    false
}
