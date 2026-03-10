use lotic::{
    declare_program, instruction,
    pinocchio::{AccountView, Address, ProgramResult},
    Context, InstructionAccounts,
};

declare_program!("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");

#[instruction]
fn initialize(ctx: Context<Initialize>) -> ProgramResult {
    ctx.accounts
        .authority
        .set_lamports(ctx.accounts.authority.lamports().checked_sub(5).unwrap());
    ctx.accounts
        .data_account
        .set_lamports(ctx.accounts.data_account.lamports().checked_add(5).unwrap());
    Ok(())
}

#[instruction]
fn aupdate(_ctx: Context<Initialize>) -> ProgramResult {
    let _vote_program_address =
        Address::from_str_const("Vote111111111111111111111111111111111111111");

    Ok(())
}

#[instruction]
fn update(_ctx: Context<Initialize>) -> ProgramResult {
    Ok(())
}

#[derive(InstructionAccounts)]
pub struct Initialize {
    #[lotic(mut, signer)]
    pub authority: AccountView,
    #[lotic(mut)]
    pub data_account: AccountView,
    #[lotic(program = token)]
    pub token_program: AccountView,
    #[lotic(program = system)]
    pub system_account: AccountView,
}
