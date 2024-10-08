use anchor_lang::prelude::*;

declare_id!("F2GMv5BTFvvJofgkx8iMrNGT8K6BDm7UDYCqPZARM6Rq");

// #[program]
// pub mod orca_manage {
//     use super::*;

//     pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
//         msg!("Greetings from: {:?}", ctx.program_id);
//         Ok(())
//     }
// }

// #[derive(Accounts)]
// pub struct Initialize {}

// use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{
        self,
        Mint,
        Token,
        TokenAccount,
        // Transfer,
    },
};

// use solana_program::program_pack::Pack;
// use std::collections::BTreeMap;
// use whirlpool_cpi::cpi::{decrease_liquidity_v2, increase_liquidity_v2, open_position};
// use whirlpool_cpi::program::Whirlpool;
use whirlpool_cpi::{self, program::Whirlpool as WhirlpoolProgram, state::*};

pub mod instructions;
use instructions::proxy_open_position::*;

#[program]
pub mod liquidity_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        // vault.bump = *ctx.bumps.get("vault").unwrap();
        vault.bump = ctx.bumps.vault;
        vault.lp_token_account = ctx.accounts.lp_token_account.key();
        vault.total_lp_tokens = 0;
        Ok(())
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        tick_lower_index: i32,
        tick_upper_index: i32,
    ) -> Result<()> {
        let _vault_info = ctx.accounts.vault.to_account_info();
        let vault_lp_token_account_info = ctx.accounts.vault_lp_token_account.to_account_info();
        let vault = &mut ctx.accounts.vault;

        // token::transfer(ctx.accounts.into_transfer_to_vault_context(), amount)?;

        // let program_id = ctx.accounts.system_program.to_account_info();
        let program_id = ctx.accounts.token_program.to_account_info();

        let cpi_context = CpiContext::new(
            program_id,
            Transfer {
                from: ctx.accounts.user_lp_token_account.to_account_info(),
                // to: ctx.accounts.vault.to_account_info(),
                to: vault_lp_token_account_info,
                // authority: ctx.accounts.user.to_account_info(), // Add this line
            },
        );
        // from user's lp token account to vault's lp token account (specific to a single token pair)
        transfer(cpi_context, amount)?;

        vault.total_lp_tokens += amount;

        // not needed if user already has lp tokens (open position)
        // open_position_handler(ctx, tick_lower_index, tick_upper_index)?;

        Ok(())
    }

    pub fn rebalance(ctx: Context<Rebalance>, min_price: u64, max_price: u64) -> Result<()> {
        // let vault = &mut ctx.accounts.vault;
        // // Load the existing whirlpool
        // let whirlpool = get_whirlpool(
        //     ctx.accounts.whirlpool_program.clone(),
        //     &vault.lp_token_account,
        // )?;
        // // Remove the liquidity first
        // let remove_result =
        //     remove_liquidity(&whirlpool, &ctx.accounts.user, vault.total_lp_tokens)?;
        // // Update the balance
        // vault.total_lp_tokens = remove_result.remaining_lp_tokens;
        // // Add new liquidity to the new price range
        // let _add_result = add_liquidity(
        //     &whirlpool,
        //     &ctx.accounts.user,
        //     min_price,
        //     max_price,
        //     remove_result.remaining_lp_tokens,
        // )?;

        rebalance_handler(ctx)?;
        Ok(())
    }
}

pub fn rebalance_handler(ctx: Context<Rebalance>) -> Result<()> {
    // close position
    let cpi_program = ctx.accounts.whirlpool_program.to_account_info();

    let cpi_accounts_close_position = whirlpool_cpi::cpi::accounts::ClosePosition {
        position_authority: ctx.accounts.position_authority.to_account_info(),
        receiver: ctx.accounts.receiver.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        position_mint: ctx.accounts.position_mint.to_account_info(),
        position_token_account: ctx.accounts.position_token_account.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };

    let cpi_ctx_close_position = CpiContext::new(cpi_program.clone(), cpi_accounts_close_position);

    // execute CPI
    msg!("CPI: whirlpool close_position instruction");
    whirlpool_cpi::cpi::close_position(cpi_ctx_close_position)?;

    // collect reward
    let reward_index = 0;
    let cpi_accounts_collect_reward = whirlpool_cpi::cpi::accounts::CollectReward {
        whirlpool: ctx.accounts.whirlpool.to_account_info(),
        position_authority: ctx.accounts.position_authority.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        position_token_account: ctx.accounts.position_token_account.to_account_info(),
        reward_owner_account: ctx.accounts.reward_owner_account.to_account_info(),
        reward_vault: ctx.accounts.reward_vault.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };

    let cpi_ctx_collect_reward = CpiContext::new(cpi_program.clone(), cpi_accounts_collect_reward);

    // execute CPI
    msg!("CPI: whirlpool collect_reward instruction");
    whirlpool_cpi::cpi::collect_reward(cpi_ctx_collect_reward, reward_index)?;

    // collect fees
    let cpi_accounts_collect_fees = whirlpool_cpi::cpi::accounts::CollectFees {
        whirlpool: ctx.accounts.whirlpool.to_account_info(),
        position_authority: ctx.accounts.position_authority.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        position_token_account: ctx.accounts.position_token_account.to_account_info(),
        token_owner_account_a: ctx.accounts.token_owner_account_a.to_account_info(),
        token_vault_a: ctx.accounts.token_vault_a.to_account_info(),
        token_owner_account_b: ctx.accounts.token_owner_account_b.to_account_info(),
        token_vault_b: ctx.accounts.token_vault_b.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };

    let cpi_ctx_collect_fees = CpiContext::new(cpi_program.clone(), cpi_accounts_collect_fees);

    // execute CPI
    msg!("CPI: whirlpool collect_fees instruction");
    whirlpool_cpi::cpi::collect_fees(cpi_ctx_collect_fees)?;

    // open position
    let tick_lower_index = 0;
    let tick_upper_index = 10;
    let cpi_accounts_open_position = whirlpool_cpi::cpi::accounts::OpenPosition {
        funder: ctx.accounts.funder.to_account_info(),
        owner: ctx.accounts.owner.to_account_info(),
        position: ctx.accounts.position.to_account_info(),
        position_mint: ctx.accounts.position_mint.to_account_info(),
        position_token_account: ctx.accounts.position_token_account.to_account_info(),
        whirlpool: ctx.accounts.whirlpool.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        rent: ctx.accounts.rent.to_account_info(),
        associated_token_program: ctx.accounts.associated_token_program.to_account_info(),
    };

    let cpi_ctx_open_position = CpiContext::new(cpi_program.clone(), cpi_accounts_open_position);

    // execute CPI
    msg!("CPI: whirlpool open_position instruction");
    whirlpool_cpi::cpi::open_position(
        cpi_ctx_open_position,
        whirlpool_cpi::state::OpenPositionBumps { position_bump: 0 }, // passed bump is no longer used
        tick_lower_index,
        tick_upper_index,
    )?;
    Ok(())
}

#[derive(Accounts)]
// #[instruction(instruction_data: String)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        seeds = [
            b"vault",
            user.key().as_ref(),
            // instruction_data.as_ref()
        ],
        bump,
        payer = user,
        space = 8 + 64
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub lp_token_account: Account<'info, TokenAccount>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_lp_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_lp_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    pub fn into_transfer_to_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_lp_token_account.to_account_info().clone(),
            to: self.vault_lp_token_account.to_account_info().clone(),
            // authority: self.user.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(reward_index: u8)]
pub struct Rebalance<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub lp_token_account: Account<'info, TokenAccount>,
    pub whirlpool_program: Program<'info, WhirlpoolProgram>,

    /// additional
    #[account(mut)]
    pub user_lp_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub vault_lp_token_account: Account<'info, TokenAccount>,
    // pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    /// close position
    // pub whirlpool_program: Program<'info, WhirlpoolProgram>,
    pub position_authority: Signer<'info>,

    /// CHECK: safe (the account to receive the remaining balance of the closed account)
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,

    // #[account(mut)]
    // pub position: Account<'info, Position>,
    #[account(mut, address = position.position_mint)]
    pub position_mint: Account<'info, Mint>,

    #[account(mut,
      constraint = position_token_account.amount == 1,
      constraint = position_token_account.mint == position.position_mint)]
    pub position_token_account: Box<Account<'info, TokenAccount>>,

    #[account(address = token::ID)]
    pub token_program: Program<'info, Token>,

    /// collect fees
    pub whirlpool: Box<Account<'info, Whirlpool>>,

    // pub position_authority: Signer<'info>,
    #[account(mut, has_one = whirlpool)]
    pub position: Box<Account<'info, Position>>,
    //     #[account(
    //       constraint = position_token_account.mint == position.position_mint,
    //       constraint = position_token_account.amount == 1
    //   )]
    //     pub position_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut, constraint = token_owner_account_a.mint == whirlpool.token_mint_a)]
    pub token_owner_account_a: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = whirlpool.token_vault_a)]
    pub token_vault_a: Box<Account<'info, TokenAccount>>,

    #[account(mut, constraint = token_owner_account_b.mint == whirlpool.token_mint_b)]
    pub token_owner_account_b: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = whirlpool.token_vault_b)]
    pub token_vault_b: Box<Account<'info, TokenAccount>>,
    // #[account(address = token::ID)]
    // pub token_program: Program<'info, Token>,
    /// collect reward
    #[account(mut,
        constraint = reward_owner_account.mint == whirlpool.reward_infos[reward_index as usize].mint
    )]
    pub reward_owner_account: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = whirlpool.reward_infos[reward_index as usize].vault)]
    pub reward_vault: Box<Account<'info, TokenAccount>>,

    /// open position
    #[account(mut)]
    pub funder: Signer<'info>,

    /// CHECK: safe (the owner of position_token_account)
    pub owner: UncheckedAccount<'info>,

    // /// CHECK: init by whirlpool
    // #[account(mut)]
    // pub position: UncheckedAccount<'info>,

    // /// CHECK: init by whirlpool
    // #[account(mut)]
    // pub position_mint: Signer<'info>,

    // /// CHECK: init by whirlpool
    // #[account(mut)]
    // pub position_token_account: UncheckedAccount<'info>,

    // pub whirlpool: Box<Account<'info, Whirlpool>>,

    // #[account(address = token::ID)]
    // pub token_program: Program<'info, Token>,
    // pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    // #[account(mut)]
    // pub vault: Account<'info, Vault>,
    // #[account(mut)]
    // pub user: Signer<'info>,
    // #[account(mut)]
    // pub user_lp_token_account: Account<'info, TokenAccount>,
    // #[account(mut)]
    // pub vault_lp_token_account: Account<'info, TokenAccount>,
}

#[account]
pub struct Vault {
    pub bump: u8,
    pub lp_token_account: Pubkey,
    pub total_lp_tokens: u64,
}
