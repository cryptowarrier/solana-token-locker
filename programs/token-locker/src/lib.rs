use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, Create, AssociatedToken},
    token::{self, Mint, TokenAccount, Token, Transfer}
};

declare_id!("AEH6XDoe6hK6XNVJpuVos19xUaE1yXNdjcVK8RRifpUB");

#[program]
pub mod token_locker {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        fee_rate: u64,
        fee_amount: u64,
    ) -> Result<()> {
        let base_account = &mut ctx.accounts.base_account;
        if !base_account.is_initialized {
            base_account.owner_account = *ctx.accounts.owner.to_account_info().key;
            base_account.fee_rate = fee_rate;
            base_account.fee_amount = fee_amount;
            base_account.fee_account = *ctx.accounts.fee_account.to_account_info().key;
            base_account.is_initialized = true;
        }
        
        Ok(())
    }

    pub fn create_user_stats(ctx: Context<CreateUserStats>) -> Result<()> {
        let user_stats = &mut ctx.accounts.user_stats;
        user_stats.total_vesting = 0;
        user_stats.bump = *ctx.bumps.get("user_stats").unwrap();
        Ok(())
    }

    pub fn create_vesting(
        ctx: Context<CreateVesting>,
        total_amount: u64,
        start_ts: u64,
        end_ts: u64,
        with_sol: bool,
        vault_bump: u8
    ) -> Result<()> {
        let mut real_amount = total_amount;
        let base_account = &mut ctx.accounts.base_account;
        if with_sol {
            let fee_amount = base_account.fee_amount;
            if ctx.accounts.user.to_account_info().lamports() < fee_amount {
                return err!(ErrCode::InvalidFund)
            }
            // send sol
            let ix = anchor_lang::solana_program::system_instruction::transfer(
                &ctx.accounts.user.key(), 
                &ctx.accounts.fee_account.key(), 
                fee_amount
            );
            anchor_lang::solana_program::program::invoke(
                &ix,
                &[
                    ctx.accounts.user.to_account_info(),
                    ctx.accounts.fee_account.to_account_info()
                ]
            )?;
        } else {
            let fee_amount = base_account.fee_rate * total_amount / 100;
            real_amount = total_amount - fee_amount;
            let fee_token_key = associated_token::get_associated_token_address(
                ctx.accounts.fee_account.key,
                ctx.accounts.mint.to_account_info().key,
            );
            if &fee_token_key != ctx.accounts.fee_token.key {
                return err!(ErrCode::InvalidAssociatedTokenAddress);
            }
            if ctx.accounts.fee_token.data_is_empty() {
                let cpi_accounts = Create {
                    payer: ctx.accounts.user.to_account_info(),
                    associated_token: ctx.accounts.fee_token.clone(),
                    authority: ctx.accounts.fee_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                };
                let cpi_program = ctx.accounts.associated_token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                associated_token::create(cpi_ctx)?;
            }
            // send fee
            let cpi_accounts = Transfer {
                from: ctx.accounts.user_token.to_account_info(),
                to: ctx.accounts.fee_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, fee_amount)?;
        }
        let now = ctx.accounts.clock.unix_timestamp as u64;
        // // check duration sanity
        if now > start_ts || end_ts <= start_ts {
            return err!(ErrCode::InvalidSchedule);
        }
        // // check recipient token address
        let recipient_token_key = associated_token::get_associated_token_address(
            ctx.accounts.recipient.key,
            ctx.accounts.mint.to_account_info().key,
        );
        
        if &recipient_token_key != ctx.accounts.recipient_token.key {
            return err!(ErrCode::InvalidAssociatedTokenAddress);
        }
        // format recipient token account if empty
        if ctx.accounts.recipient_token.data_is_empty() {
            let cpi_accounts = Create {
                payer: ctx.accounts.user.to_account_info(),
                associated_token: ctx.accounts.recipient_token.clone(),
                authority: ctx.accounts.recipient.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            };
            let cpi_program = ctx.accounts.associated_token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            associated_token::create(cpi_ctx)?;
        }

        // // register lock info
        let vesting = Vesting {
            withdrawn_amount: 0,
            remaining_amount: real_amount,
            granter: *ctx.accounts.user.to_account_info().key,
            granter_token: *ctx.accounts.user_token.to_account_info().key,
            recipient: *ctx.accounts.recipient.to_account_info().key,
            recipient_token: *ctx.accounts.recipient_token.to_account_info().key,
            mint: *ctx.accounts.mint.to_account_info().key,
            vault: *ctx.accounts.vault.to_account_info().key,
            start_ts: start_ts,
            end_ts: end_ts,
            total_amount: real_amount,
        };

        let user_stats = &mut ctx.accounts.user_stats;
        user_stats.vest_list.push(vesting);
        user_stats.total_vesting += 1;
        // lock token

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, real_amount)?;
        base_account.bump = vault_bump;

        Ok(())
    }

    pub fn unlock(ctx: Context<Unlock>, id: u8, amount: u64) -> Result<()> {
        let now = ctx.accounts.clock.unix_timestamp as u64;
        let user_stats = &mut ctx.accounts.user_stats;
        let end_ts = user_stats.vest_list[usize::from(id)].end_ts;

        if end_ts > now {
            return err!(ErrCode::InvalidUnlockTime);
        }

        let remaining_amount = user_stats.vest_list[usize::from(id)].remaining_amount;

        if amount > remaining_amount {
            return err!(ErrCode::InvalidUnlockAmount);
        }

        // funds out
        let base_account = &mut ctx.accounts.base_account;
        let seeds = &[
            base_account.to_account_info().key.as_ref(),
            &[base_account.bump],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.recipient_token.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts).with_signer(signer);
        token::transfer(cpi_ctx, amount)?;

        user_stats.vest_list[usize::from(id)].remaining_amount = user_stats.vest_list[usize::from(id)].remaining_amount - amount;
        user_stats.vest_list[usize::from(id)].withdrawn_amount = user_stats.vest_list[usize::from(id)].withdrawn_amount + amount;

        if user_stats.vest_list[usize::from(id)].remaining_amount == 0 {
            user_stats.vest_list.remove(usize::from(id));
        }

        Ok(())
    }
}

#[account]
pub struct UserStats {
    total_vesting: u64,
    vest_list: Vec<Vesting>,
    bump: u8,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Vesting {
    pub withdrawn_amount: u64,
    pub remaining_amount: u64,
    pub total_amount: u64,
    pub granter: Pubkey,
    pub granter_token: Pubkey,
    pub recipient: Pubkey,
    pub recipient_token: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,

    pub start_ts: u64,
    pub end_ts: u64,
}

#[account]
#[derive(Default)]
pub struct BaseAccount {
    pub fee_account: Pubkey,
    pub owner_account: Pubkey,
    pub fee_amount: u64,
    pub fee_rate: u64,
    pub is_initialized: bool,
    pub bump: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = owner, space = 100)]
    pub base_account: Box<Account<'info, BaseAccount>>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
    ///CHECK we don't read and write this
    pub fee_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CreateVesting<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token: Account<'info, TokenAccount>,
    ///CHECK: don't read write this contract
    pub recipient: AccountInfo<'info>,

    ///CHECK: don't read write this contract
    #[account(mut)]
    pub recipient_token: AccountInfo<'info>,
    ///CHECK: don't read write this contract
    #[account(mut)]
    pub fee_token: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,

    #[account(
        init, payer = user, 
        seeds = [base_account.key().as_ref()], bump,
        owner = token_program.key(),
        rent_exempt = enforce,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub clock: Sysvar<'info, Clock>,
    // pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    #[account(
        mut,
        constraint = base_account.fee_account == fee_account.key()
    )]
    pub base_account: Account<'info, BaseAccount>,
    ///CHECK: don't read write this contract
    #[account(mut)]
    pub fee_account: AccountInfo<'info>,
    #[account(mut, seeds = [b"user-stats", user.key().as_ref()], bump = user_stats.bump)]
    pub user_stats: Account<'info, UserStats>,
    pub system_program: Program<'info, System>,
}

// validation struct
#[derive(Accounts)]

pub struct CreateUserStats<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        space = 9000, seeds = [b"user-stats", user.key().as_ref()], bump
    )]
    pub user_stats: Account<'info, UserStats>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(id: u8)]
pub struct Unlock<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        constraint = recipient_token.mint == mint.key(),
    )]
    pub recipient_token: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault.mint == mint.key(),
        seeds = [base_account.key().as_ref()], bump = base_account.bump,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(address = user_stats.vest_list[usize::from(id)].mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"user-stats", user.key().as_ref()], bump = user_stats.bump)]
    pub user_stats: Account<'info, UserStats>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
    #[account(mut)]
    pub base_account: Account<'info, BaseAccount>,
}


#[error_code]
pub enum ErrCode {
    #[msg("Invalid vesting schedule given.")]
    InvalidSchedule,
    #[msg("Invalid associated token address. Did you provide the correct address?")]
    InvalidAssociatedTokenAddress,
    #[msg("Insufficient fund")]
    InvalidFund,
    #[msg("Invalid unlock time")]
    InvalidUnlockTime,
    #[msg("Invalid unlock amount")]
    InvalidUnlockAmount,
}