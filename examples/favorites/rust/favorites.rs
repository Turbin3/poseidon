use anchor_lang::prelude::*;
declare_id!("11111111111111111111111111111111");
#[program]
pub mod favorites_program {
    use super::*;
    pub fn set_favorites(
        ctx: Context<SetFavoritesContext>,
        number: u64,
        color: String,
        hobbies: Vec<String>,
    ) -> Result<()> {
        ctx.accounts.favorites.number = number;
        ctx.accounts.favorites.color = color;
        ctx.accounts.favorites.hobbies = hobbies;
        Ok(())
    }
}
#[derive(Accounts)]
pub struct SetFavoritesContext<'info> {
    #[account(
        init_if_needed,
        payer = owner,
        space = 344,
        seeds = [b"favorites",
        owner.key().as_ref()],
        bump,
    )]
    pub favorites: Account<'info, Favorites>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[account]
pub struct Favorites {
    pub number: u64,
    pub color: String,
    pub hobbies: Vec<String>,
}
