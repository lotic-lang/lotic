use proc_macro::TokenStream;

mod account_state;
mod declare_program;
mod instruction;
mod instruction_accounts;
mod metadata_reader;

#[proc_macro]
pub fn declare_program(input: TokenStream) -> TokenStream {
    declare_program::declare_program(input)
}

#[proc_macro_attribute]
pub fn instruction(attr: TokenStream, item: TokenStream) -> TokenStream {
    instruction::instruction(attr, item)
}

#[proc_macro_derive(InstructionAccounts, attributes(lotic))]
pub fn instruction_accounts(input: TokenStream) -> TokenStream {
    instruction_accounts::instruction_accounts(input)
}

#[proc_macro_attribute]
pub fn account_state(attr: TokenStream, item: TokenStream) -> TokenStream {
    account_state::account_state(attr, item)
}
