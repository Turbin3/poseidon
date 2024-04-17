use anchor::prelude::*;
#[program]
pub mod VoteProgram {
    declare_id!("HC2oqz2p6DEWfrahenqdq2moUcga9c9biqRBcdK3XKU1");
    pub fn initialize(ctx: Context<InitializeContext>, hash: Vec<u8>) -> Result<()> {}
    pub fn upvote(ctx: Context<UpvoteContext>, hash: Vec<u8>) -> Result<()> {}
    pub fn downvote(ctx: Context<DownvoteContext>, hash: Vec<u8>) -> Result<()> {}
}
#[derive(Accounts)]
pub struct InitializeContext<'info> {
    #[account()]
    pub user: Signer<'info>,
    # [account (init , payer = user , bump seeds = [b "vote" , hash])]
    pub state: Account<'info, VoteState>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct UpvoteContext<'info> {
    # [account (seeds = [b "vote" , hash])]
    pub state: Account<'info, VoteState>,
}
#[derive(Accounts)]
pub struct DownvoteContext<'info> {
    # [account (seeds = [b "vote" , hash])]
    pub state: Account<'info, VoteState>,
}
#[account]
pub struct VoteState {
    pub vote: i64,
}
