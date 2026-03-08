pub use lotic_macros::{declare_program, instruction, InstructionAccounts};
pub use pinocchio;
pub use solana_address;

pub struct Context<'a, T> {
    pub program_id: &'a pinocchio::Address,
    pub accounts: &'a mut T,
}
