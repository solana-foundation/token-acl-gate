use solana_commitment_config::CommitmentConfig;
use solana_instruction::AccountMeta;
use solana_keypair::{Keypair, Signer};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::Message;
use solana_transaction::Transaction;
use {
    clap::{crate_description, crate_name, crate_version, Arg, Command},
    solana_clap_v3_utils::{
        input_parsers::{
            parse_url_or_moniker,
            signer::{SignerSource, SignerSourceParserBuilder},
        },
        input_validators::normalize_to_url_if_moniker,
        keypair::signer_from_path,
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    std::{error::Error, process::exit, rc::Rc, sync::Arc},
};

struct Config {
    commitment_config: CommitmentConfig,
    payer: Arc<dyn Signer>,
    json_rpc_url: String,
    verbose: bool,
}

/// Best-effort preflight for the thaw-side setup: an Allow list only
/// restricts token accounts that are created Frozen, so surface the two
/// operational hazards of installing one — a mint whose DefaultAccountState
/// is not Frozen (new accounts start usable and never need a gated thaw),
/// and pre-existing Initialized accounts (freezing the default is not
/// retroactive). Never blocks the setup: this exists to support migrating
/// existing mints, and RPC or decoding failures are ignored.
async fn warn_if_allow_list_policy_not_enforcing(
    rpc_client: &Arc<RpcClient>,
    mint_address: &Pubkey,
    lists: &[Pubkey],
) {
    const MODE_ALLOW: u8 = 0;

    let mut has_allow_list = false;
    let Ok(accounts) = rpc_client.get_multiple_accounts(lists).await else {
        return;
    };

    for account in accounts {
        let Some(acc) = account else {
            return;
        };
        let Ok(config) = token_acl_gate_client::accounts::ListConfig::from_bytes(&acc.data) else {
            return;
        };
        if config.mode == MODE_ALLOW {
            has_allow_list = true;
            break;
        }
    }
    if !has_allow_list {
        return;
    }

    let Ok(mint_account) = rpc_client.get_account(mint_address).await else {
        return;
    };
    let default_is_frozen = spl_token_2022_interface::extension::PodStateWithExtensions::<
        spl_token_2022_interface::pod::PodMint,
    >::unpack(&mint_account.data)
    .ok()
    .and_then(|mint| {
        spl_token_2022_interface::extension::BaseStateWithExtensions::get_extension::<
            spl_token_2022_interface::extension::default_account_state::DefaultAccountState,
        >(&mint)
        .ok()
        .map(|ext| ext.state == spl_token_2022_interface::state::AccountState::Frozen as u8)
    })
    .unwrap_or(false);

    if !default_is_frozen {
        eprintln!(
            "WARNING: this thaw policy contains an Allow list, but the mint's \
             DefaultAccountState is not Frozen. New token accounts are created \
             usable and never require a gated thaw, so the allow list does NOT \
             restrict them. Set DefaultAccountState to Frozen to activate the \
             allow list for newly created accounts."
        );
    }
    eprintln!(
        "NOTE: DefaultAccountState::Frozen only applies to token accounts created \
         after it is set. Token accounts that already exist as Initialized remain \
         usable without a gated thaw - freeze or migrate unlisted holders before \
         relying on the allow list for pre-existing balances."
    );
}

async fn process_setup_extra_metas(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    mint_address: &Pubkey,
    lists: &[Pubkey],
) -> Result<Signature, Box<dyn Error>> {
    warn_if_allow_list_policy_not_enforcing(rpc_client, mint_address, lists).await;

    let token_acl_mint_config = token_acl_client::accounts::MintConfig::find_pda(mint_address).0;
    let extra_metas = token_acl_interface::get_thaw_extra_account_metas_address(
        mint_address,
        &token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID,
    );
    let ix = token_acl_gate_client::instructions::SetupExtraMetasBuilder::new()
        .authority(payer.pubkey())
        .payer(payer.pubkey())
        .token_acl_mint_config(token_acl_mint_config)
        .mint(*mint_address)
        .extra_metas(extra_metas)
        .add_remaining_accounts(
            lists
                .iter()
                .map(|list| AccountMeta::new_readonly(*list, false))
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .instruction();

    let mut transaction = Transaction::new_unsigned(Message::new(&[ix], Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    Ok(signature)
}

async fn process_setup_freeze_extra_metas(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    mint_address: &Pubkey,
    lists: &[Pubkey],
) -> Result<Signature, Box<dyn Error>> {
    let token_acl_mint_config = token_acl_client::accounts::MintConfig::find_pda(mint_address).0;
    let extra_metas = token_acl_interface::get_freeze_extra_account_metas_address(
        mint_address,
        &token_acl_gate_client::programs::TOKEN_ACL_GATE_PROGRAM_ID,
    );
    let ix = token_acl_gate_client::instructions::SetupFreezeExtraMetasBuilder::new()
        .authority(payer.pubkey())
        .payer(payer.pubkey())
        .token_acl_mint_config(token_acl_mint_config)
        .mint(*mint_address)
        .extra_metas(extra_metas)
        .add_remaining_accounts(
            lists
                .iter()
                .map(|list| AccountMeta::new_readonly(*list, false))
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .instruction();

    let mut transaction = Transaction::new_unsigned(Message::new(&[ix], Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    Ok(signature)
}

async fn process_create_list(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    mode: token_acl_gate_client::types::Mode,
) -> Result<Signature, Box<dyn Error>> {
    let seed = Keypair::new().pubkey();
    let list_config =
        token_acl_gate_client::accounts::ListConfig::find_pda(&payer.pubkey(), &seed).0;
    let ix = token_acl_gate_client::instructions::CreateListBuilder::new()
        .authority(payer.pubkey())
        .payer(payer.pubkey())
        .seed(seed)
        .mode(mode)
        .list_config(list_config)
        .instruction();

    let mut transaction = Transaction::new_unsigned(Message::new(&[ix], Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    println!("list_config: {:?}", list_config);
    println!("seed: {:?}", seed);

    Ok(signature)
}

async fn process_delete_list(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    list_address: &Pubkey,
) -> Result<Signature, Box<dyn Error>> {
    let ix = token_acl_gate_client::instructions::DeleteListBuilder::new()
        .authority(payer.pubkey())
        .list_config(*list_address)
        .instruction();

    let mut transaction = Transaction::new_unsigned(Message::new(&[ix], Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    Ok(signature)
}

async fn process_add_wallet(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    wallet_address: &Pubkey,
    list_address: &Pubkey,
) -> Result<Signature, Box<dyn Error>> {
    let ix = token_acl_gate_client::instructions::AddWalletBuilder::new()
        .authority(payer.pubkey())
        .payer(payer.pubkey())
        .list_config(*list_address)
        .wallet(*wallet_address)
        .wallet_entry(
            token_acl_gate_client::accounts::WalletEntry::find_pda(list_address, wallet_address).0,
        )
        .instruction();

    let mut transaction = Transaction::new_unsigned(Message::new(&[ix], Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    Ok(signature)
}

async fn process_remove_wallet(
    rpc_client: &Arc<RpcClient>,
    payer: &Arc<dyn Signer>,
    wallet_address: &Pubkey,
    list_address: &Pubkey,
) -> Result<Signature, Box<dyn Error>> {
    let ix = token_acl_gate_client::instructions::RemoveWalletBuilder::new()
        .authority(payer.pubkey())
        .list_config(*list_address)
        .wallet_entry(
            token_acl_gate_client::accounts::WalletEntry::find_pda(list_address, wallet_address).0,
        )
        .instruction();

    let mut transaction = Transaction::new_unsigned(Message::new(&[ix], Some(&payer.pubkey())));

    let blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|err| format!("error: unable to get latest blockhash: {}", err))?;

    transaction
        .try_sign(&[payer], blockhash)
        .map_err(|err| format!("error: failed to sign transaction: {}", err))?;

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)
        .await
        .map_err(|err| format!("error: send transaction: {}", err))?;

    Ok(signature)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let app_matches = Command::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg({
            let arg = Arg::new("config_file")
                .short('C')
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::new("payer")
                .long("payer")
                .short('k')
                .value_name("KEYPAIR")
                .value_parser(SignerSourceParserBuilder::default().allow_all().build())
                .takes_value(true)
                .global(true)
                .help("Filepath or URL to a keypair [default: client keypair]"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::new("json_rpc_url")
                .short('u')
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .value_parser(parse_url_or_moniker)
                .help("JSON RPC URL for the cluster [default: value from configuration file]"),
        )
        .subcommand(
            Command::new("create-list")
                .about("Creates a new list")
                .arg(
                    Arg::new("mode")
                        .value_name("MODE")
                        .takes_value(true)
                        .short('m')
                        .long("mode")
                        .possible_values(["allow", "block"])
                        .required(true)
                        .help("Specify the mode"),
                )
        )
        .subcommand(
            Command::new("delete-list")
                .about("Deletes a list")
                .arg(
                    Arg::new("list_address")
                        .value_name("LIST_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(1)
                        .help("Specify the list address"),
        ))
        .subcommand(
            Command::new("add-wallet")
                .about("Adds a wallet to a list")
                .arg(
                    Arg::new("list_address")
                        .value_name("LIST_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(1)
                        .help("Specify the list address"),
                )
                .arg(
                    Arg::new("wallet_address")
                        .value_name("WALLET_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(2)
                        .help("Specify the wallet address to add"),
                )
        )
        .subcommand(
            Command::new("remove-wallet")
                .about("Removes a wallet from a list")
                .arg(
                    Arg::new("list_address")
                        .value_name("LIST_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(1)
                        .help("Specify the list address"),
                )
                .arg(
                    Arg::new("wallet_address")
                        .value_name("WALLET_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(2)
                        .help("Specify the wallet address to remove"),
                )
        )
        .subcommand(
            Command::new("apply-lists-to-mint")
                .about("Configures the extra metas for the mint permissionless thaw. This sets up which lists are used during the permisisonless thaw operation.")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("lists")
                        .value_name("LISTS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .multiple_occurrences(true)
                        .index(2)
                        .help("Specify the list(s) address(es)"),
                )
                ,
        )
        .subcommand(
            Command::new("apply-lists-to-mint-freeze")
                .about("Configures the extra metas for the mint permissionless freeze. This sets up which lists are used during the permissionless freeze operation.")
                .arg(
                    Arg::new("mint_address")
                        .value_name("MINT_ADDRESS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .index(1)
                        .help("Specify the mint address"),
                )
                .arg(
                    Arg::new("lists")
                        .value_name("LISTS")
                        .value_parser(SignerSourceParserBuilder::default().allow_pubkey().build())
                        .takes_value(true)
                        .multiple_occurrences(true)
                        .index(2)
                        .help("Specify the list(s) address(es)"),
                )
        )
        .get_matches();

    let (command, matches) = app_matches.subcommand().unwrap();
    let mut wallet_manager: Option<Rc<RemoteWalletManager>> = None;

    let config = {
        let cli_config = if let Some(config_file) = matches.try_get_one::<String>("config_file")? {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        let payer = if let Ok(Some((signer, _))) =
            SignerSource::try_get_signer(matches, "payer", &mut wallet_manager)
        {
            Box::new(signer)
        } else {
            signer_from_path(
                matches,
                &cli_config.keypair_path,
                "payer",
                &mut wallet_manager,
            )?
        };

        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .get_one::<String>("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );

        Config {
            commitment_config: CommitmentConfig::confirmed(),
            payer: Arc::from(payer),
            json_rpc_url,
            verbose: matches.try_contains_id("verbose")?,
        }
    };
    solana_logger::setup_with_default("solana=info");

    if config.verbose {
        println!("JSON RPC URL: {}", config.json_rpc_url);
    }
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        config.json_rpc_url.clone(),
        config.commitment_config,
    ));

    match (command, matches) {
        ("create-list", arg_matches) => {
            let mode = arg_matches.get_one::<String>("mode").unwrap();
            let mode = match mode.as_str() {
                "allow" => token_acl_gate_client::types::Mode::Allow,
                "block" => token_acl_gate_client::types::Mode::Block,
                _ => unreachable!(),
            };
            let response = process_create_list(&rpc_client, &config.payer, mode)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: create-list: {}", err);
                    exit(1);
                });
            println!("{}", response);
        }
        ("delete-list", arg_matches) => {
            let list_address =
                SignerSource::try_get_pubkey(arg_matches, "list_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response = process_delete_list(&rpc_client, &config.payer, &list_address)
                .await
                .unwrap_or_else(|err| {
                    eprintln!("error: delete-list: {}", err);
                    exit(1);
                });
            println!("{}", response);
        }
        ("add-wallet", arg_matches) => {
            let wallet_address =
                SignerSource::try_get_pubkey(arg_matches, "wallet_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let list_address =
                SignerSource::try_get_pubkey(arg_matches, "list_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response =
                process_add_wallet(&rpc_client, &config.payer, &wallet_address, &list_address)
                    .await
                    .unwrap_or_else(|err| {
                        eprintln!("error: add-wallet: {}", err);
                        exit(1);
                    });
            println!("{}", response);
        }
        ("remove-wallet", arg_matches) => {
            let wallet_address =
                SignerSource::try_get_pubkey(arg_matches, "wallet_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let list_address =
                SignerSource::try_get_pubkey(arg_matches, "list_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let response =
                process_remove_wallet(&rpc_client, &config.payer, &wallet_address, &list_address)
                    .await
                    .unwrap_or_else(|err| {
                        eprintln!("error: remove-wallet: {}", err);
                        exit(1);
                    });
            println!("{}", response);
        }
        ("apply-lists-to-mint", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            println!("mint_address: {:?}", mint_address);

            let lists = SignerSource::try_get_pubkeys(arg_matches, "lists", &mut wallet_manager)
                .unwrap()
                .unwrap();
            println!("lists: {:?}", lists);
            let response =
                process_setup_extra_metas(&rpc_client, &config.payer, &mint_address, &lists)
                    .await
                    .unwrap_or_else(|err| {
                        eprintln!("error: apply-lists-to-mint: {}", err);
                        exit(1);
                    });
            println!("{}", response);
        }
        ("apply-lists-to-mint-freeze", arg_matches) => {
            let mint_address =
                SignerSource::try_get_pubkey(arg_matches, "mint_address", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            println!("mint_address: {:?}", mint_address);

            let lists = SignerSource::try_get_pubkeys(arg_matches, "lists", &mut wallet_manager)
                .unwrap()
                .unwrap();
            println!("lists: {:?}", lists);
            let response =
                process_setup_freeze_extra_metas(&rpc_client, &config.payer, &mint_address, &lists)
                    .await
                    .unwrap_or_else(|err| {
                        eprintln!("error: apply-lists-to-mint-freeze: {}", err);
                        exit(1);
                    });
            println!("{}", response);
        }
        _ => unreachable!(),
    };

    Ok(())
}
