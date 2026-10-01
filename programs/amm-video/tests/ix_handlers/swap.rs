use {
    anchor_lang::{
        solana_program::instruction::Instruction, system_program::ID as SYSTEM_PROGRAM_ID,
        InstructionData, ToAccountMetas,
    },
    anchor_spl::associated_token::{self, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
    litesvm::LiteSVM,
    litesvm_token::spl_token::ID as TOKEN_PROGRAM_ID,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
};

pub fn create_swap_ix(
    mut _svm: &mut LiteSVM,
    payer: &Keypair,
    mint_x: Pubkey,
    mint_y: Pubkey,
    mint_lp: Pubkey,
    config: Pubkey,
    treasury: Pubkey,
    vault_x: Pubkey,
    vault_y: Pubkey,
    amount_in: u64,
    min_amount_out: u64,
    is_x: bool,
) -> Instruction {
    let user = payer.pubkey();
    let user_x = associated_token::get_associated_token_address(&user, &mint_x);
    let user_y = associated_token::get_associated_token_address(&user, &mint_y);

    Instruction::new_with_bytes(
        amm_video::id(),
        &amm_video::instruction::Swap {
            is_x,
            amount_in,
            min_amount_out,
        }
        .data(),
        amm_video::accounts::Swap {
            user,
            mint_x,
            mint_y,
            config,
            treasury,
            mint_lp,
            vault_x,
            vault_y,
            treasury_x: associated_token::get_associated_token_address(&treasury, &mint_x),
            treasury_y: associated_token::get_associated_token_address(&treasury, &mint_y),
            user_x,
            user_y,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
        }
        .to_account_metas(None),
    )
}
