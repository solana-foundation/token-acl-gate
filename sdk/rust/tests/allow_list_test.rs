pub mod program_test;
use solana_signer::Signer;
use token_acl_gate_client::types::Mode;

use crate::program_test::TestContext;

mod thaw {
    use super::*;

    #[tokio::test]
    async fn non_whitelisted_wallet_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn whitelisted_wallet_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&list_config, &user_pubkey);
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }
}

mod freeze {
    use super::*;

    use litesvm::types::FailedTransactionMetadata;
    use solana_account::Account;
    use solana_signer::Signer;
    use solana_transaction::Transaction;
    use solana_transaction::TransactionError::InstructionError;
    use token_acl_gate_client::{
        accounts::{WalletEntry, WALLET_ENTRY_DISCRIMINATOR},
        types::Mode,
    };

    fn overwrite_wallet_entry(
        context: &mut TestContext,
        wallet_entry: &solana_pubkey::Pubkey,
        entry: WalletEntry,
        owner: solana_pubkey::Pubkey,
    ) {
        let account = context
            .vm
            .get_account(wallet_entry)
            .expect("wallet entry account should exist");

        context
            .vm
            .set_account(
                *wallet_entry,
                Account {
                    lamports: account.lamports,
                    data: borsh::to_vec(&entry).unwrap(),
                    owner,
                    executable: account.executable,
                    rent_epoch: account.rent_epoch,
                },
            )
            .unwrap();
    }

    #[tokio::test]
    async fn wallet_not_on_allowlist_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn allowlisted_wallet_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&list_config, &user_pubkey);
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn wallet_entry_with_different_address_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let wallet_entry = context.add_wallet_to_list(&list_config, &wallet.pubkey());
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        overwrite_wallet_entry(
            &mut context,
            &wallet_entry,
            WalletEntry {
                discriminator: WALLET_ENTRY_DISCRIMINATOR,
                wallet_address: solana_pubkey::Pubkey::new_unique(),
                list_config,
            },
            token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID,
        );

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn valid_wallet_entry_with_different_list_config_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let different_list_config = context.create_list(Mode::Allow);
        assert_ne!(list_config, different_list_config);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let wallet_entry = context.add_wallet_to_list(&list_config, &wallet.pubkey());
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        overwrite_wallet_entry(
            &mut context,
            &wallet_entry,
            WalletEntry {
                discriminator: WALLET_ENTRY_DISCRIMINATOR,
                wallet_address: wallet_entry,
                list_config: different_list_config,
            },
            token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID,
        );

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;

        match res {
            Err(FailedTransactionMetadata { err, meta: _ }) => {
                // 15 = 0xF = InvalidWalletEntry
                assert_eq!(
                    err,
                    InstructionError(0, solana_transaction::InstructionError::Custom(15))
                );
            }
            other => panic!("unexpected result: {:?}", other),
        };
    }

    #[tokio::test]
    async fn non_existent_wallet_entry_with_different_list_config_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let different_list_config = context.create_list(Mode::Allow);
        assert_ne!(list_config, different_list_config);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let (wrong_wallet_entry, _) = token_acl_gate_client::accounts::WalletEntry::find_pda(
            &different_list_config,
            &wallet.pubkey(),
        );
        // Build the normal token-acl freeze_permissionless instruction, then
        // replace the resolved wallet-entry account with one derived from a
        // different list config to verify token-acl rejects it during account
        // resolution before invoking the gate program.
        let mut ix = context
            .get_freeze_permissionless_ix(&context.auth.pubkey(), &wallet.pubkey(), &ta)
            .await;
        assert_eq!(ix.program_id, token_acl::ID);
        let wallet_entry_account = ix
            .accounts
            .last_mut()
            .expect("freeze ix should include a wallet entry account");
        wallet_entry_account.pubkey = wrong_wallet_entry;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&context.auth.pubkey()),
            &[context.auth.insecure_clone()],
            context.vm.latest_blockhash(),
        );
        let res = context.vm.send_transaction(tx);

        // #[error("Incorrect account provided")]
        // IncorrectAccount = 2_724_315_840,
        // https://github.com/solana-program/libraries/blob/1b60dd54705070f862dad4f8337cbb2d2f19ddb5/tlv-account-resolution/src/error.rs#L18-L19
        match res {
            Err(FailedTransactionMetadata { err, meta: _ }) => {
                assert_eq!(
                    err,
                    InstructionError(
                        0,
                        solana_transaction::InstructionError::Custom(2_724_315_840)
                    )
                );
            }
            other => panic!("unexpected result: {:?}", other),
        };
    }

    #[tokio::test]
    async fn wallet_entry_owned_by_different_program_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let wallet_entry = context.add_wallet_to_list(&list_config, &wallet.pubkey());
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        overwrite_wallet_entry(
            &mut context,
            &wallet_entry,
            WalletEntry {
                discriminator: WALLET_ENTRY_DISCRIMINATOR,
                wallet_address: wallet_entry,
                list_config,
            },
            solana_system_interface::program::ID,
        );

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }
}
