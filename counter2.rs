use anchor::prelude::*;
#[program]
pub mod VoteProgram {
    declare_id!("HC2oqz2p6DEWfrahenqdq2moUcga9c9biqRBcdK3XKU1");
    pub fn initialize(ctx: Context<InitializeContext>, hash: Vec<u8>) -> Result<()> {}
    pub fn upvote(ctx: Context<UpvoteContext>, hash: Vec<u8>) -> Result<()> {}
    pub fn downvote(ctx: Context<DownvoteContext>, hash: Vec<u8>) -> Result<()> {}
}
#[derive(Accounts)]
pub struct InitializeContext {
    #[account(init)]
    pub state: Account<VoteState>,
}
#[derive(Accounts)]
pub struct UpvoteContext {
    #[account()]
    pub state: Account<VoteState>,
}
#[derive(Accounts)]
pub struct DownvoteContext {
    #[account()]
    pub state: Account<VoteState>,
}
#[account]
pub struct VoteState {
    pub vote: i64,
    pub bump: u8,
}
