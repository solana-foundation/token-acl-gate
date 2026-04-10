pub mod program_test;
use solana_keypair::{Keypair, Signer};
use solana_program_option::COption;
use solana_program_pack::Pack;
use solana_transaction::Transaction;
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent;
use spl_token_2022_interface::state::{Account, AccountState};
use token_acl_gate_client::types::Mode;

use crate::program_test::TestContext;

mod thaw {

    use super::*;

    #[tokio::test]
    async fn non_eoa_wallet_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[list_config]);

        let ta = context.create_token_account_from_pubkey(&list_config);

        let res = context.thaw_permissionless(&list_config, &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn non_eoa_added_owner_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[list_config]);

        // list_config is acting as the ta owner for test simplicity
        // as this is one of the off-the-curve available pubkeys
        let _ = context.add_wallet_to_list(&list_config, &list_config);

        let ta = context.create_token_account_from_pubkey(&list_config);

        let res = context.thaw_permissionless(&list_config, &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn eoa_wallet_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn eoa_wallet_on_ata_creation_succeeds() {
        let mut context = TestContext::new();

        let mint_cfg_pk = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[list_config]);

        let user = Keypair::new();
        let user_pubkey = user.pubkey();

        let mut instructions = Vec::new();

        let res = context.vm.airdrop(&user.pubkey(), 1_000_000_000);
        assert!(res.is_ok());

        let token_account = get_associated_token_address_with_program_id(
            &user_pubkey,
            &context.token.mint,
            &spl_token_2022_interface::ID,
        );

        let ix = create_associated_token_account_idempotent(
            &user_pubkey,
            &user_pubkey,
            &context.token.mint,
            &spl_token_2022_interface::ID,
        );
        instructions.push(ix);

        let acc = Account {
            mint: context.token.mint,
            owner: user_pubkey,
            amount: 0,
            delegate: COption::None,
            state: AccountState::Frozen,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };

        let mut data = vec![0u8; Account::LEN];
        let res = Account::pack(acc, &mut data);
        assert!(res.is_ok());

        let ix = token_acl_client::create_thaw_permissionless_instruction_with_extra_metas(
            &user_pubkey,
            &token_account,
            &context.token.mint,
            &mint_cfg_pk,
            &spl_token_2022_interface::ID,
            &user_pubkey,
            false,
            |pubkey| {
                let data = data.clone();
                let data2 = context.vm.get_account(&pubkey);
                async move {
                    if pubkey == token_account {
                        return Ok(Some(data));
                    }
                    Ok(data2.map(|a| a.data.clone()))
                }
            },
        )
        .await
        .unwrap();

        instructions.push(ix);

        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&user_pubkey),
            &[user.insecure_clone()],
            context.vm.latest_blockhash(),
        );
        let res = context.vm.send_transaction(tx);
        assert!(res.is_ok());
    }
}

mod freeze {
    use super::*;

    #[tokio::test]
    async fn eoa_wallet_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn non_eoa_wallet_not_on_list_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let ta = context.create_token_account_from_pubkey(&list_config);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&list_config, &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn non_eoa_wallet_on_list_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        // list_config is acting as the ta owner for test simplicity
        // as this is one of the off-the-curve available pubkeys
        let _ = context.add_wallet_to_list(&list_config, &list_config);
        let ta = context.create_token_account_from_pubkey(&list_config);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&list_config, &ta).await;
        assert!(res.is_err());
    }
}
