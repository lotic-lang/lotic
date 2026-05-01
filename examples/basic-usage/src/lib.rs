use lotic::{
    account_state, declare_program, instruction,
    pinocchio::{AccountView, ProgramResult},
    Account, Context, InstructionAccounts,
};

declare_program!("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");

#[instruction]
fn initialize(_ctx: Context<Initialize>) -> ProgramResult {
    Ok(())
}

#[derive(InstructionAccounts)]
pub struct Initialize<'b> {
    #[lotic(signer)]
    pub account0: &'b mut AccountView,
    #[lotic(init, payer = account0, space = 8)]
    pub account: Account<'b, NewAccount>,
    // #[lotic(init, payer = account2, space = 9)]
    // pub account2: Account<'b, NewAccount>,
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
