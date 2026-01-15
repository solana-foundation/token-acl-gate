use litesvm::types::TransactionResult;
use litesvm::LiteSVM;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_system_interface::instruction::create_account;
use solana_system_interface::program::ID;
use spl_associated_token_account_client::address::get_associated_token_address_with_program_id;
use spl_associated_token_account_client::instruction::create_associated_token_account;
use spl_token_2022::extension::default_account_state::instruction::initialize_default_account_state;
use spl_token_2022::extension::ExtensionType;
use spl_token_2022::instruction::initialize_mint2;
use spl_token_2022::state::{AccountState, Mint};
use token_acl_gate_client::types::Mode;

pub struct TestContext {
    pub vm: LiteSVM,
    pub token: TokenContext,
    pub auth: Keypair,
}

pub struct TokenContext {
    pub mint: Pubkey,
    pub auth: Keypair,
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TestContext {
    pub fn new() -> Self {
        let mut vm = LiteSVM::new();

        // current path
        let current_dir = std::env::current_dir().unwrap();

        let res = vm.add_program_from_file(
            token_acl::ID,
            current_dir.join("tests/fixtures/token_acl.so"),
        );
        assert!(res.is_ok());

        let res = vm.add_program_from_file(
            token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID,
            current_dir.join("tests/fixtures/token_acl_gate_program.so"),
        );
        assert!(res.is_ok());

        let auth = Keypair::new();
        let auth_pubkey = auth.pubkey();

        let _ = vm.airdrop(&auth_pubkey, 1_000_000_000_000);

        let token = Self::create_token(&mut vm);

        Self { vm, token, auth }
    }

    pub fn create_token(vm: &mut LiteSVM) -> TokenContext {
        let auth = Keypair::new();
        let auth_pubkey = auth.pubkey();

        let res = vm.airdrop(&auth_pubkey, 1_000_000_000_000);
        assert!(res.is_ok());

        let mint_size =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::DefaultAccountState])
                .unwrap();
        let mint_kp = Keypair::new();
        let mint_pk = mint_kp.pubkey();
        let token_program_id = &spl_token_2022::ID;
        let payer_pk = auth.pubkey();

        let ix1 = create_account(
            &payer_pk,
            &mint_pk,
            vm.minimum_balance_for_rent_exemption(mint_size),
            mint_size as u64,
            token_program_id,
        );

        let ix2 =
            initialize_default_account_state(token_program_id, &mint_pk, &AccountState::Frozen)
                .unwrap();

        let ix3 = initialize_mint2(
            token_program_id,
            &mint_pk,
            &auth_pubkey,
            Some(&auth_pubkey),
            6,
        )
        .unwrap();

        let block_hash = vm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix1, ix2, ix3],
            Some(&payer_pk),
            &[auth.insecure_clone(), mint_kp],
            block_hash,
        );
        let res = vm.send_transaction(tx);
        assert!(res.is_ok());

        TokenContext {
            mint: mint_pk,
            auth,
        }
    }

    fn create_token_account_with_params(
        vm: &mut LiteSVM,
        mint: &Pubkey,
        owner: &Pubkey,
        payer: &Keypair,
        airdrop: bool,
    ) -> Pubkey {
        let token_program_id = &spl_token_2022::ID;
        let payer_pk = payer.pubkey();

        if airdrop {
            let res = vm.airdrop(&payer_pk, 1_000_000_000);
            assert!(res.is_ok());
        }

        let token_account =
            get_associated_token_address_with_program_id(&owner, mint, token_program_id);

        let ix = create_associated_token_account(&payer_pk, &owner, mint, token_program_id);

        let block_hash = vm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer_pk),
            &[payer.insecure_clone()],
            block_hash,
        );

        let res = vm.send_transaction(tx);
        assert!(res.is_ok());

        token_account
    }

    pub fn create_token_account(&mut self, owner: &Keypair) -> Pubkey {
        Self::create_token_account_with_params(
            &mut self.vm,
            &self.token.mint,
            &owner.pubkey(),
            owner,
            true,
        )
    }
    pub fn create_token_account_from_pubkey(&mut self, owner: &Pubkey) -> Pubkey {
        Self::create_token_account_with_params(
            &mut self.vm,
            &self.token.mint,
            owner,
            &self.auth,
            false,
        )
    }

    pub fn create_list(&mut self, mode: Mode) -> Pubkey {
        let seed = Pubkey::new_unique();

        let (list_config_address, _) =
            token_acl_gate_client::accounts::ListConfig::find_pda(&self.auth.pubkey(), &seed);

        let ix = token_acl_gate_client::instructions::CreateListBuilder::new()
            .authority(self.auth.pubkey())
            .list_config(list_config_address)
            .mode(mode)
            .seed(seed)
            .instruction();

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.auth.pubkey()),
            &[self.auth.insecure_clone()],
            self.vm.latest_blockhash(),
        );
        let res = self.vm.send_transaction(tx);
        assert!(res.is_ok());

        list_config_address
    }

    pub fn setup_extra_metas(&mut self, lists: &[Pubkey]) -> Pubkey {
        let (mint_cfg_pk, _) = token_acl_client::accounts::MintConfig::find_pda(&self.token.mint);

        let extra_metas = token_acl_interface::get_thaw_extra_account_metas_address(
            &self.token.mint,
            &token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID,
        );

        let ix = token_acl_gate_client::instructions::SetupExtraMetasBuilder::new()
            .authority(self.token.auth.pubkey())
            .payer(self.token.auth.pubkey())
            .mint(self.token.mint)
            .extra_metas(extra_metas)
            .token_acl_mint_config(mint_cfg_pk)
            .add_remaining_accounts(
                lists
                    .iter()
                    .map(|list| AccountMeta::new_readonly(*list, false))
                    .collect::<Vec<_>>()
                    .as_slice(),
            )
            .instruction();

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.token.auth.pubkey()),
            &[self.token.auth.insecure_clone()],
            self.vm.latest_blockhash(),
        );

        let res = self.vm.send_transaction(tx);
        assert!(res.is_ok());

        extra_metas
    }

    pub fn add_wallet_to_list(&mut self, list: &Pubkey, wallet_address: &Pubkey) -> Pubkey {
        let (wallet_entry, _) =
            token_acl_gate_client::accounts::WalletEntry::find_pda(&list, &wallet_address);

        let ix = token_acl_gate_client::instructions::AddWalletBuilder::new()
            .authority(self.auth.pubkey())
            .list_config(*list)
            .wallet(*wallet_address)
            .wallet_entry(wallet_entry)
            .instruction();

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.auth.pubkey()),
            &[self.auth.insecure_clone()],
            self.vm.latest_blockhash(),
        );
        let res = self.vm.send_transaction(tx);
        assert!(res.is_ok());

        wallet_entry
    }

    pub async fn get_thaw_permissionless_ix(
        &mut self,
        signer: &Pubkey,
        owner: &Pubkey,
        token_account: &Pubkey,
    ) -> Instruction {
        let (mint_cfg_pk, _) = token_acl_client::accounts::MintConfig::find_pda(&self.token.mint);

        token_acl_client::create_thaw_permissionless_instruction_with_extra_metas(
            signer,
            token_account,
            &self.token.mint,
            &mint_cfg_pk,
            &spl_token_2022::ID,
            owner,
            false,
            |pubkey| {
                let account = self.vm.get_account(&pubkey);

                async move {
                    match account {
                        Some(account) => Ok(Some(account.data)),
                        None => Ok(None),
                    }
                }
            },
        )
        .await
        .unwrap()
    }

    pub async fn thaw_permissionless(
        &mut self,
        owner: &Pubkey,
        token_account: &Pubkey,
    ) -> TransactionResult {
        let ix = self
            .get_thaw_permissionless_ix(&self.auth.pubkey(), owner, token_account)
            .await;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.auth.pubkey()),
            &[self.auth.insecure_clone()],
            self.vm.latest_blockhash(),
        );
        self.vm.send_transaction(tx)
    }

    pub fn setup_token_acl(&mut self) -> Pubkey {
        let (mint_cfg_pk, _) = token_acl_client::accounts::MintConfig::find_pda(&self.token.mint);

        let ix = token_acl_client::instructions::CreateConfigBuilder::new()
            .authority(self.token.auth.pubkey())
            .gating_program(token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID)
            .mint(self.token.mint)
            .mint_config(mint_cfg_pk)
            .payer(self.token.auth.pubkey())
            .system_program(ID)
            .token_program(spl_token_2022::ID)
            .instruction();

        let ix2 = token_acl_client::instructions::TogglePermissionlessInstructionsBuilder::new()
            .authority(self.token.auth.pubkey())
            .mint_config(mint_cfg_pk)
            .freeze_enabled(false)
            .thaw_enabled(true)
            .instruction();

        let tx = Transaction::new_signed_with_payer(
            &[ix, ix2],
            Some(&self.token.auth.pubkey()),
            &[self.token.auth.insecure_clone()],
            self.vm.latest_blockhash(),
        );
        let res = self.vm.send_transaction(tx);
        assert!(res.is_ok());

        mint_cfg_pk
    }
}
