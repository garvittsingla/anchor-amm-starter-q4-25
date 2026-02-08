use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{burn, transfer, Burn, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{errors::AmmError, state::Config};



//user gives back the lp tokens, we burn the give token x and token y in return to the user
#[derive(Accounts)]
pub struct Withdraw<'info> {
    //TODO
    pub user: Signer<'info>, //user that wants to withdraw liquidity,
    pub mint_x: Account<'info, Mint>, //mint account of token x
    pub mint_y: Account<'info, Mint>, //mint account of token y

    #[account(
        has_one = mint_x, 
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    //ata of token x and token y , vault and user
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Account<'info, TokenAccount>, //ata token account owned by the user to receive token x, initialized if it doesn't exist
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Account<'info, TokenAccount>, 
     #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Account<'info, TokenAccount>, 
     #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Account<'info, TokenAccount>, 

    //lp account
     #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Account<'info, Mint>,
     #[account(
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
    )]
    pub user_lp: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,

}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        amount: u64, // Amount of LP tokens that the user wants to "burn"
        min_x: u64,  // Minimum amount of token X that the user wants to receive
        min_y: u64,  // Minimum amount of token Y that the user wants to receive
    ) -> Result<()> {

       let amountsoftokens = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            self.mint_lp.supply,
            amount,
            6,
        );
        let result = amountsoftokens.unwrap();
        
        // require!(result.x >= min_x, AmmError::SlippageExceeded);
        // require!(result.y >= min_y, AmmError::SlippageExceeded);

        // burn lp tokens from the user
        self.burn_lp_tokens(amount)?;
        // transfer token x from vault to user
        self.withdraw_tokens(true, result.x)?;
        // transfer token y from vault to user
        self.withdraw_tokens(false, result.y)?;
        

        Ok(())
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from,to) = match is_x {
            true => {
                //transfer token x from vault to user
                (self.vault_x.to_account_info(), self.user_x.to_account_info())
            },
            false => {
                //transfer token y from vault to user
                (self.vault_y.to_account_info(), self.user_y.to_account_info())
            },
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];


       let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            Transfer {
                from,
                to,
                authority: self.config.to_account_info(),
            },
            signer_seeds,
        );

        transfer(ctx, amount)?;
        Ok(())
    }

    pub fn burn_lp_tokens(&self, amount: u64) -> Result<()> {
        //TODO
        let ctx = CpiContext::new(
            self.token_program.to_account_info(),
            Burn {
                mint: self.mint_lp.to_account_info(),
                from: self.user_lp.to_account_info(),
                authority: self.user.to_account_info(),
            },
        );

        burn(ctx, amount)?;
        Ok(())
    }
}
