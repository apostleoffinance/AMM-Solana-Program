use {
    anchor_lang::AccountDeserialize,
    anchor_spl::{associated_token, token::TokenAccount},
    litesvm::LiteSVM,
    litesvm_token::CreateMint,
    solana_keypair::Keypair,
    solana_message::{Instruction, Message, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

mod ix_handlers;
use ix_handlers::*;

fn send(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    svm.send_transaction(tx)
}

// Setup function to initialize LiteSVM and create a payer keypair
fn setup() -> (
    LiteSVM,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
) {
    let program_id = amm_video::id();
    let payer = Keypair::new();
    let treasury = Keypair::new().pubkey();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/amm_video.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&treasury, 1_000_000_000).unwrap();

    // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
    // This done using litesvm-token's CreateMint utility which creates the mint in the LiteSVM environment
    let mint_x = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .authority(&payer.pubkey())
        .send()
        .unwrap();

    let mint_y = CreateMint::new(&mut svm, &payer)
        .decimals(6)
        .authority(&payer.pubkey())
        .send()
        .unwrap();

    let config =
        Pubkey::find_program_address(&[b"config", &123u64.to_le_bytes()], &amm_video::id()).0;
    let mint_lp = Pubkey::find_program_address(&[b"lp", config.as_ref()], &amm_video::id()).0;

    // Derive the PDA for the vault associated token account using the config PDA and Mint A
    let vault_x = associated_token::get_associated_token_address(&config, &mint_x);
    let vault_y = associated_token::get_associated_token_address(&config, &mint_y);

    (
        svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury,
    )
}

fn token_amount(svm: &LiteSVM, address: Pubkey) -> u64 {
    let account = svm.get_account(&address).unwrap();
    let mut data = account.data.as_slice();
    TokenAccount::try_deserialize(&mut data).unwrap().amount
}

fn initialize_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    config: Pubkey,
    mint_lp: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    treasury: Pubkey,
    fee: u16,
) -> Instruction {
    create_initialise_ix(
        svm,
        payer,
        mint_x,
        mint_y,
        treasury,
        config,
        mint_lp,
        vault_x,
        vault_y,
        associated_token::get_associated_token_address(&treasury, &mint_x),
        associated_token::get_associated_token_address(&treasury, &mint_y),
        fee,
    )
}

fn deposit_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    amount: u64,
    max_x: u64,
    max_y: u64,
) -> Instruction {
    create_deposit_ix(
        svm, payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y, amount, max_x, max_y,
    )
}

fn withdraw_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    amount: u64,
    min_x: u64,
    min_y: u64,
) -> Instruction {
    create_withdraw_ix(
        svm, payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y, amount, min_x, min_y,
    )
}

fn swap_ix(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    treasury: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    amount: u64,
    min: u64,
) -> Instruction {
    create_swap_ix(
        svm, payer, mint_x, mint_y, mint_lp, config, treasury, vault_x, vault_y, amount, min, true,
    )
}

#[test]
fn test_initialize() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let instruction = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let res = send(&mut svm, &[instruction], &payer, &[&payer]);
    assert!(res.is_ok());
    assert_eq!(
        token_amount(
            &svm,
            associated_token::get_associated_token_address(&treasury, &mint_x)
        ),
        0
    );
    assert_eq!(
        token_amount(
            &svm,
            associated_token::get_associated_token_address(&treasury, &mint_y)
        ),
        0
    );
}

#[test]
fn test_initialize_rejects_fee_at_or_above_one_hundred_percent() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let instruction = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 10_000,
    );
    assert!(send(&mut svm, &[instruction], &payer, &[&payer]).is_err());
}

#[test]
fn test_deposit_mints_lp_and_funds_both_vaults() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit_ix = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    let res = send(&mut svm, &[init_ix, deposit_ix], &payer, &[&payer]);
    assert!(res.is_ok());
    assert_eq!(token_amount(&svm, vault_x), 200_000_000);
    assert_eq!(token_amount(&svm, vault_y), 200_000_000);
    assert_eq!(
        token_amount(
            &svm,
            associated_token::get_associated_token_address(&payer.pubkey(), &mint_lp)
        ),
        200_000_000
    );
}

#[test]
fn test_deposit_rejects_zero_lp_amount() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit_ix = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        0,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit_ix], &payer, &[&payer]).is_err());
    assert!(svm.get_account(&vault_x).is_none());
}

#[test]
fn test_initial_deposit_rejects_mismatched_lp_issuance() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit_ix = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        100_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit_ix], &payer, &[&payer]).is_err());
    assert!(svm.get_account(&vault_x).is_none());
}

#[test]
fn test_deposit_enforces_maximum_amounts_after_initial_liquidity() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let first = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, first], &payer, &[&payer]).is_ok());
    let too_small = deposit_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y, 10_000_000, 1, 1,
    );
    assert!(send(&mut svm, &[too_small], &payer, &[&payer]).is_err());
    assert_eq!(token_amount(&svm, vault_x), 200_000_000);
}

#[test]
fn test_deposit_rejects_amounts_that_round_to_zero() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let first = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        100_000_000,
    );
    assert!(send(&mut svm, &[init_ix, first], &payer, &[&payer]).is_ok());
    let dust = deposit_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y, 1, 1, 1,
    );
    assert!(send(&mut svm, &[dust], &payer, &[&payer]).is_err());
    assert_eq!(token_amount(&svm, vault_x), 200_000_000);
}

#[test]
fn test_withdraw_burns_lp_and_returns_both_assets() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit_ix = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    let withdraw_ix = withdraw_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y, 10_000_000,
        10_000_000, 10_000_000,
    );
    let res = send(
        &mut svm,
        &[init_ix, deposit_ix, withdraw_ix],
        &payer,
        &[&payer],
    );
    assert!(res.is_ok());
    assert_eq!(token_amount(&svm, vault_x), 190_000_000);
    assert_eq!(token_amount(&svm, vault_y), 190_000_000);
    assert_eq!(
        token_amount(
            &svm,
            associated_token::get_associated_token_address(&payer.pubkey(), &mint_lp)
        ),
        190_000_000
    );
}

#[test]
fn test_withdraw_enforces_minimum_amounts() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit_ix = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit_ix], &payer, &[&payer]).is_ok());
    let withdraw_ix = withdraw_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, vault_x, vault_y, 10_000_000,
        10_000_001, 10_000_001,
    );
    assert!(send(&mut svm, &[withdraw_ix], &payer, &[&payer]).is_err());
    assert_eq!(token_amount(&svm, vault_x), 200_000_000);
}

#[test]
fn test_withdraw_rejects_amount_above_total_lp_supply() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit], &payer, &[&payer]).is_ok());
    let too_many_lp = withdraw_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_001,
        0,
        0,
    );
    assert!(send(&mut svm, &[too_many_lp], &payer, &[&payer]).is_err());
    assert_eq!(token_amount(&svm, vault_x), 200_000_000);
}

#[test]
fn test_swap_routes_x_fee_to_treasury_and_prices_from_net_input() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let treasury_x = associated_token::get_associated_token_address(&treasury, &mint_x);
    let treasury_y = associated_token::get_associated_token_address(&treasury, &mint_y);
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit], &payer, &[&payer]).is_ok());
    let swap = swap_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, treasury, vault_x, vault_y, 10_000_000,
        1,
    );
    assert!(send(&mut svm, &[swap], &payer, &[&payer]).is_ok());

    let fee = 30_000;
    let net_input = 10_000_000 - fee;
    let output =
        (200_000_000u128 * net_input as u128 / (200_000_000u128 + net_input as u128)) as u64;
    assert_eq!(token_amount(&svm, treasury_x), fee);
    assert_eq!(token_amount(&svm, treasury_y), 0);
    assert_eq!(token_amount(&svm, vault_x), 200_000_000 + net_input);
    assert_eq!(token_amount(&svm, vault_y), 200_000_000 - output);
}

#[test]
fn test_swap_reverse_direction_routes_y_fee_to_treasury() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let treasury_y = associated_token::get_associated_token_address(&treasury, &mint_y);
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit], &payer, &[&payer]).is_ok());
    let reverse = create_swap_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, treasury, vault_x, vault_y, 10_000_000,
        1, false,
    );
    assert!(send(&mut svm, &[reverse], &payer, &[&payer]).is_ok());
    assert_eq!(token_amount(&svm, treasury_y), 30_000);
    assert_eq!(token_amount(&svm, vault_y), 209_970_000);
}

#[test]
fn test_swap_rejects_slippage_and_zero_input() {
    let (mut svm, payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury) = setup();
    let init_ix = initialize_ix(
        &mut svm, &payer, mint_x, mint_y, config, mint_lp, vault_x, vault_y, treasury, 30,
    );
    let deposit = deposit_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        vault_x,
        vault_y,
        200_000_000,
        200_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[init_ix, deposit], &payer, &[&payer]).is_ok());
    let impossible_min = swap_ix(
        &mut svm,
        &payer,
        mint_x,
        mint_y,
        mint_lp,
        config,
        treasury,
        vault_x,
        vault_y,
        10_000_000,
        200_000_000,
    );
    assert!(send(&mut svm, &[impossible_min], &payer, &[&payer]).is_err());
    let zero_input = swap_ix(
        &mut svm, &payer, mint_x, mint_y, mint_lp, config, treasury, vault_x, vault_y, 0, 0,
    );
    assert!(send(&mut svm, &[zero_input], &payer, &[&payer]).is_err());
    assert_eq!(token_amount(&svm, vault_x), 200_000_000);
}
