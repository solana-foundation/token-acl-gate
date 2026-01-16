pub mod program_test;
use solana_sdk::signer::Signer;
use token_acl_gate_client::types::Mode;

use crate::program_test::TestContext;

#[tokio::test]
async fn setups_composite_lists() {
    let mut context = TestContext::new();

    let _ = context.setup_token_acl();
    let allow_list = context.create_list(Mode::Allow);
    let block_list = context.create_list(Mode::Block);
    let allow_all_eoas_list = context.create_list(Mode::AllowAllEoas);
    let _ = context.setup_extra_metas(&[allow_list, block_list, allow_all_eoas_list]);
}

#[tokio::test]
async fn thaws_eoa_wallet_in_composite_lists() {
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
async fn fails_thaw_blocked_eoa_wallet_in_composite_lists() {
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
async fn fails_thaw_non_allowed_eoa_wallet_in_composite_lists() {
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
async fn fails_thaw_non_eoa_wallet_in_composite_lists() {
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
async fn thaws_non_eoa_wallet_in_composite_lists() {
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
    let _ = context.add_wallet_to_list(&allow_all_eoas_with_pda_list, &block_list);

    let res = context.thaw_permissionless(&block_list, &ta).await;
    assert!(res.is_err());
}

/*#[tokio::test]
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
}*/
