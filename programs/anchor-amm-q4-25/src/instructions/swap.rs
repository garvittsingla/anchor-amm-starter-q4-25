use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{errors::AmmError, state::Config};

#[derive(Accounts)]
pub struct Swap<'info> {
// TODO: Write the accounts struct
   // we need to swap x tokens for y
pub user : Signer<'info>, //user that wants to swap tokens
pub mint_x: Account<'info, Mint>, //mint account of token x
pub mint_y: Account<'info, Mint>, //mint account of token y
#[account(
    has_one = mint_x, 
    has_one = mint_y,
    seeds = [b"config", config.seed.to_le_bytes().as_ref()],
    bump = config.config_bump,
)]
pub config: Account<'info, Config>, //config account for the pool that the user wants
#[account(
    mut,
    seeds = [b"lp", config.key().as_ref()],
    bump = config.lp_bump,
)]
pub mint_lp: Account<'info, Mint>, //mint account for the lp token
#[account(
    mut,
    associated_token::mint = mint_x,
    associated_token::authority = config,
)]
pub vault_x: Account<'info, TokenAccount>, //vault owned by the program to hold
#[account(
    mut,
    associated_token::mint = mint_y,
    associated_token::authority = config,
)]
pub vault_y: Account<'info, TokenAccount>, //vault owned by the program to hold
#[account(
    mut,
    associated_token::mint = mint_x,
    associated_token::authority = user,
)]
pub user_x: Account<'info, TokenAccount>, //ata token account owned by the user to send token x from
#[account(
    mut,
    associated_token::mint = mint_y,
    associated_token::authority = user,     
)]
pub user_y: Account<'info, TokenAccount>, //ata token account owned by the user
#[account(
    mut,
    associated_token::mint = mint_lp,
    associated_token::authority = user,
)]
pub user_lp: Account<'info, TokenAccount>, //ata token account owned by the user
pub token_program: Program<'info, Token>,
pub associated_token_program: Program<'info, AssociatedToken>,
pub system_program: Program<'info, System>,
    
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount: u64, min: u64) -> Result<()> {
    require!(amount > 0, AmmError::InvalidAmount);
    require!(self.config.locked == false, AmmError::PoolLocked);
    
    self.deposit_tokens(is_x, amount)?;

    let curve = ConstantProduct::init(
        self.vault_x.amount,
        self.vault_y.amount,
        self.mint_lp.supply,
        self.config.fee,
        None,
    );
    let mut result = curve.unwrap();

   let result2 = match is_x {
        // Swapping X for Y
        true => result.swap(LiquidityPair::X, amount, min),
        // Swapping Y for X
        false => result.swap(LiquidityPair::Y, amount, min),
    };

    let amount_out = result2.unwrap();
    self.withdraw_tokens(!is_x, amount_out.deposit)?;

    Ok(())
}
    //transfers token from user to vault
    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from,to) = match is_x{
            true => (&self.user_x, &self.vault_x),
            false => (&self.user_y, &self.vault_y),
        };

        let cpi_accounts = Transfer {
            from: from.to_account_info(),
            to: to.to_account_info(),
            authority: self.user.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, amount)?;
        Ok(())
}
    //transfers token fro vault to user
    pub fn withdraw_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
        let (from,to) = match is_x{
            true => (&self.vault_x, &self.user_x),
            false => (&self.vault_y, &self.user_y),
        };
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];
        let cpi_accounts = Transfer {
            from: from.to_account_info(),
            to: to.to_account_info(),
            authority: self.config.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)?;

        Ok(())
    }
// TODO
 
}
