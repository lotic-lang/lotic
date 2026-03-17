use lotic::{
    declare_program, instruction,
    pinocchio::{AccountView, ProgramResult},
    Context, InstructionAccounts,
};

declare_program!("2JF8AjwkmCz6brkAkJf8NEEKhg89a8KrTuDZiZ5cVdS2");

#[instruction]
fn initialize(_ctx: Context<Initialize>) -> ProgramResult {
    Ok(())
}

#[derive(InstructionAccounts)]
pub struct Initialize {
    pub user: AccountView,
    #[lotic(program = system)]
    pub stake_account: AccountView,
}
