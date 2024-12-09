use anchor_lang::prelude::*;
declare_id!("11111111111111111111111111111111");
#[program]
pub mod chat_program {
    use super::*;
    pub fn initialize(ctx: Context<InitializeContext>) -> Result<()> {
        ctx.accounts.board_state.authority = ctx.accounts.authority.key();
        ctx.accounts.board_state.message_count = 0;
        ctx.accounts.board_state.bump = ctx.bumps.board_state;
        Ok(())
    }
    pub fn post_message(
        ctx: Context<PostMessageContext>,
        title: String,
        content: String,
    ) -> Result<()> {
        ctx.accounts.message.author = ctx.accounts.author.key();
        ctx.accounts.message.title = title;
        ctx.accounts.message.content = content;
        ctx.accounts.message.message_index = ctx.accounts.board_state.key(); //type mismatch error, won't build
        ctx.accounts.message.bump = ctx.bumps.message;
        ctx
            .accounts
            .board_state
            .message_count = ctx.accounts.board_state.message_count + 1;
        Ok(())
    }
    pub fn edit_message(
        ctx: Context<EditMessageContext>,
        new_title: String,
        new_content: String,
    ) -> Result<()> {
        ctx.accounts.message.title = new_title;
        ctx.accounts.message.content = new_content;
        Ok(())
    }
    pub fn delete_message(ctx: Context<DeleteMessageContext>) -> Result<()> {
        Ok(())
    }
}
#[derive(Accounts)]
pub struct InitializeContext<'info> {
    #[account(init, payer = authority, space = 49, seeds = [b"board"], bump)]
    pub board_state: Account<'info, BoardState>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct PostMessageContext<'info> {
    #[account(
        init,
        payer = author,
        space = 1145,
        seeds = [b"message",
        board_state.message_count.to_le_bytes().as_ref(),
        author.key().as_ref()],
        bump,
    )]
    pub message: Account<'info, Message>,
    #[account(mut, seeds = [b"board"], bump)]
    pub board_state: Account<'info, BoardState>,
    #[account(mut)]
    pub author: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct EditMessageContext<'info> {
    #[account(
        mut,
        seeds = [b"message",
        message.message_index.to_le_bytes().as_ref(),
        author.key().as_ref()],
        bump,
    )]
    pub message: Account<'info, Message>,
    #[account(seeds = [b"board"], bump)]
    pub board_state: Account<'info, BoardState>,
    #[account(mut)]
    pub author: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct DeleteMessageContext<'info> {
    #[account(mut)]
    pub author: Signer<'info>,
    #[account(seeds = [b"board"], bump)]
    pub board_state: Account<'info, BoardState>,
    #[account(
        mut,
        seeds = [b"message",
        message.message_index.to_le_bytes().as_ref(),
        author.key().as_ref()],
        bump,
        close = author,
    )]
    pub message: Account<'info, Message>,
    pub system_program: Program<'info, System>,
}
#[account]
pub struct Message {
    pub author: Pubkey,
    pub title: String,
    pub content: String,
    pub message_index: i64,
    pub bump: u8,
}
#[account]
pub struct BoardState {
    pub authority: Pubkey,
    pub message_count: i64,
    pub bump: u8,
}
