// use lotic::pinocchio::sysvars::Sysvar;
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
pub struct Initialize<'a> {
    #[lotic(signer, writable)]
    pub account: &'a mut AccountView,
    #[lotic(init, payer = account, space = 10)]
    pub new_account: Account<'a, NewAccount>,
    #[lotic(system)]
    pub system_program: &'a mut AccountView,
}

#[account_state]
pub struct NewAccount {}
