use lotic::{
    Account, Context, InstructionAccounts, account_state, declare_program, instruction, pinocchio::{AccountView, Address, ProgramResult, account}
};

declare_program!("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");

#[instruction]
fn initialize(ctx: Context<Initialize>) -> ProgramResult {
    ctx.accounts.account.state.age = NewAccount2 { name: 1, age: 1 };
    Ok(())
}

#[derive(InstructionAccounts)]
pub struct Initialize<'b> {
    #[lotic(signer)]
    pub account: Account<'b, NewAccount>,
}

#[account_state]
pub struct NewAccount {
    name: u32,
    age: NewAccount2,
}

#[account_state]
pub struct NewAccount2 {
    name: u32,
    age: u32,
}
