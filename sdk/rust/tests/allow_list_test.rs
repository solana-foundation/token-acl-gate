pub mod program_test;
use token_acl_gate_client::types::Mode;
use solana_sdk::signer::Signer;

use crate::program_test::TestContext;

#[tokio::test]
async fn fails_to_thaw_non_whitelisted_wallet() {
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
async fn thaws_whitelisted_wallet() {
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
