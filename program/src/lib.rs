#![no_std]

use pinocchio::{
    account_info::AccountInfo, default_allocator, program_entrypoint, program_error::ProgramError,
    pubkey::Pubkey, ProgramResult,
};
use pinocchio_pubkey::declare_id;
use spl_discriminator::SplDiscriminate;
use token_acl_interface::instruction::{
    CanFreezePermissionlessInstruction, CanThawPermissionlessInstruction,
};

program_entrypoint!(process_instruction, 16);

// need allocator due to dependency on spl_tlv_account_resolution
//no_allocator!();
default_allocator!();

pub mod instructions;
pub use instructions::*;
pub mod error;
pub use error::*;
pub mod state;
pub use state::*;

declare_id!("GATEzzqxhJnsWF6vHRsgtixxSB8PaQdcqGEVTEHWiULz");

#[inline(always)]
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data {
        CanThawPermissionlessInstruction::SPL_DISCRIMINATOR_SLICE => {
            CanThawPermissionless::try_from(accounts)?.process()
        }
        CanFreezePermissionlessInstruction::SPL_DISCRIMINATOR_SLICE => {
            CanFreezePermissionless::try_from(accounts)?.process()
        }
        [disc, remaining_data @ ..] => match *disc {
            CreateList::DISCRIMINATOR => CreateList::try_from(accounts)?.process(remaining_data),
            DeleteList::DISCRIMINATOR => DeleteList::try_from(accounts)?.process(),
            AddWallet::DISCRIMINATOR => AddWallet::try_from(accounts)?.process(),
            RemoveWallet::DISCRIMINATOR => RemoveWallet::try_from(accounts)?.process(),
            SetupExtraMetas::DISCRIMINATOR => {
                SetupExtraMetas::try_from(accounts)?.process(ExtraMetasVariant::Thaw)
            }
            SetupFreezeExtraMetas::DISCRIMINATOR => {
                SetupExtraMetas::try_from(accounts)?.process(ExtraMetasVariant::Freeze)
            }
            _ => Err(ProgramError::InvalidInstructionData),
        },
        _ => Err(ABLError::InvalidInstruction.into()),
    }
}
