pub mod program_test;
use solana_keypair::Signer;
use token_acl_gate_client::types::Mode;

use crate::program_test::TestContext;

#[tokio::test]
async fn setups_composite_lists() {
    let mut context = TestContext::new();

    let _ = context.setup_token_acl();
    let allow_list = context.create_list(Mode::Allow);
    let block_list = context.create_list(Mode::Block);
    let _ = context.setup_extra_metas(&[allow_list, block_list]);
}

mod thaw {
    use super::*;

    #[tokio::test]
    async fn wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_list = context.create_list(Mode::Allow);
        let _ = context.setup_extra_metas(&[block_list, allow_list]);

        let wallet = solana_keypair::Keypair::new();
        let _ = context.add_wallet_to_list(&allow_list, &wallet.pubkey());
        //let user_pubkey = wallet.pubkey();
        //let _ = context.add_wallet_to_list(&list_config, &user_pubkey);
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn blocked_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_list = context.create_list(Mode::Allow);
        let _ = context.setup_extra_metas(&[block_list, allow_list]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();

        let _ = context.add_wallet_to_list(&allow_list, &wallet.pubkey());
        let _ = context.add_wallet_to_list(&block_list, &user_pubkey);
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn non_allowed_wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        //let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let allow_list = context.create_list(Mode::Allow);
        let _ = context.setup_extra_metas(&[block_list, allow_list]);

        let wallet = solana_keypair::Keypair::new();

        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }
}

mod freeze {
    use super::*;

    #[tokio::test]
    async fn wallet_in_composite_lists_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let block_list = context.create_list(Mode::Block);
        let allow_list = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[block_list, allow_list]);

        let wallet = solana_keypair::Keypair::new();
        let _ = context.add_wallet_to_list(&allow_list, &wallet.pubkey());
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn blocked_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let block_list = context.create_list(Mode::Block);
        let allow_list = context.create_list(Mode::Allow);
        let _ = context.setup_freeze_extra_metas(&[block_list, allow_list]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&block_list, &user_pubkey);
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn non_allowed_wallet_in_composite_lists_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let allow_list = context.create_list(Mode::Allow);
        let block_list = context.create_list(Mode::Block);
        let _ = context.setup_freeze_extra_metas(&[allow_list, block_list]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }
}
