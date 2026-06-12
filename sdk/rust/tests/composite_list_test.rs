pub mod program_test;
use solana_keypair::Signer;
use token_acl_gate_client::types::Mode;

use crate::program_test::TestContext;

mod thaw {
    use super::*;

    #[tokio::test]
    async fn eoa_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[block_list, allow_all_eoas_list]);

        let wallet = solana_keypair::Keypair::new();
        //let user_pubkey = wallet.pubkey();
        //let _ = context.add_wallet_to_list(&list_config, &user_pubkey);
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn blocked_eoa_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[block_list, allow_all_eoas_list]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&block_list, &user_pubkey);
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn non_allowed_eoa_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let allow_list = context.create_list(Mode::Allow);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[allow_list, allow_all_eoas_list]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn non_eoa_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[block_list, allow_all_eoas_list]);

        // block_list is acting as the ta owner for test simplicity
        // as this is one of the off-the-curve available pubkeys
        let ta = context.create_token_account_from_pubkey(&block_list);

        let res = context.thaw_permissionless(&block_list, &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn non_eoa_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let allow_all_eoas_with_pda_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_extra_metas(&[
            block_list,
            allow_all_eoas_list,
            allow_all_eoas_with_pda_list,
        ]);

        // block_list is acting as the ta owner for test simplicity
        // as this is one of the off-the-curve available pubkeys
        let ta = context.create_token_account_from_pubkey(&block_list);
        let _ = context.add_wallet_to_list(&allow_all_eoas_list, &block_list);
        let _ = context.add_wallet_to_list(&allow_all_eoas_with_pda_list, &block_list);

        let res = context.thaw_permissionless(&block_list, &ta).await;
        assert!(res.is_ok());
    }
}

mod freeze {
    use super::*;

    #[tokio::test]
    async fn eoa_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[block_list, allow_all_eoas_list]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn blocked_eoa_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[block_list, allow_all_eoas_list]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&block_list, &user_pubkey);
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn non_allowed_eoa_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let allow_list = context.create_list(Mode::Allow);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[allow_list, allow_all_eoas_list]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn non_eoa_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[block_list, allow_all_eoas_list]);

        // block_list is acting as the ta owner for test simplicity
        // as this is one of the off-the-curve available pubkeys
        let ta = context.create_token_account_from_pubkey(&block_list);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&block_list, &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn non_eoa_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let block_list = context.create_list(Mode::Block);
        let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
        let allow_all_eoas_with_pda_list = context.create_list(Mode::AllowAllEoas);
        let _ = context.setup_freeze_extra_metas(&[
            block_list,
            allow_all_eoas_list,
            allow_all_eoas_with_pda_list,
        ]);

        // block_list is acting as the ta owner for test simplicity
        // as this is one of the off-the-curve available pubkeys
        let ta = context.create_token_account_from_pubkey(&block_list);
        let _ = context.add_wallet_to_list(&allow_all_eoas_list, &block_list);
        let _ = context.add_wallet_to_list(&allow_all_eoas_with_pda_list, &block_list);
        context.thaw(&ta);

        // wallet is on all allowlist and not on blocklist, so it hits fallback return
        let res = context.freeze_permissionless(&block_list, &ta).await;
        assert!(res.is_err());
    }
}
