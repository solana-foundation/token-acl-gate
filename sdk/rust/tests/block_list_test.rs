pub mod program_test;
use solana_keypair::Signer;
use token_acl_gate_client::types::Mode;

use crate::program_test::TestContext;

mod thaw {
    use super::*;

    #[tokio::test]
    async fn non_blocked_wallet_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Block);
        let _ = context.setup_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn blocked_wallet_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Block);
        let _ = context.setup_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&list_config, &user_pubkey);
        let ta = context.create_token_account(&wallet);

        let res = context.thaw_permissionless(&wallet.pubkey(), &ta).await;
        println!("res: {:?}", res);
        assert!(res.is_err());
    }
}

mod freeze {
    use super::*;

    #[tokio::test]
    async fn blocked_wallet_succeeds() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Block);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let user_pubkey = wallet.pubkey();
        let _ = context.add_wallet_to_list(&list_config, &user_pubkey);
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn non_blocked_wallet_fails() {
        let mut context = TestContext::new();

        let _ = context.setup_token_acl();
        let list_config = context.create_list(Mode::Block);
        let _ = context.setup_freeze_extra_metas(&[list_config]);

        let wallet = solana_keypair::Keypair::new();
        let ta = context.create_token_account(&wallet);
        context.thaw(&ta);

        let res = context.freeze_permissionless(&wallet.pubkey(), &ta).await;
        assert!(res.is_err());
    }
}
