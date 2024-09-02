use anchor_lang::prelude::*;
declare_id!("HC2oqz2p6DEWfrahenqdq2moUcga9c9biqRBcdK3XKU1");
#[program]
pub mod VoteProgram {
    pub fn initialize(ctx: Context<InitializeContext>, hash: Vec<u8>) -> Result<()> {
        ctx.state.vote = 0;
        Ok(())
    }
    pub fn upvote(ctx: Context<UpvoteContext>, hash: Vec<u8>) -> Result<()> {
        ctx.state.vote = ctx.accounts.state.vote + 1;
        Ok(())
    }
    pub fn downvote(ctx: Context<DownvoteContext>, hash: Vec<u8>) -> Result<()> {
        ctx.accounts.state.vote = ctx.accounts.state.vote - 1;
        Ok(())
    }
}
#[derive(Accounts)]
pub struct InitializeContext<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(init, payer = user, space = 9, seeds = [b"vote", hash], bump)]
    pub state: Account<'info, VoteState>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct UpvoteContext<'info> {
    #[account(mut, seeds = [b"vote", hash], bump)]
    pub state: Account<'info, VoteState>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct DownvoteContext<'info> {
    #[account(mut, seeds = [b"vote", hash], bump)]
    pub state: Account<'info, VoteState>,
    pub system_program: Program<'info, System>,
}
#[account]
pub struct VoteState {
    pub vote: i64,
    pub bump: u8,
}
