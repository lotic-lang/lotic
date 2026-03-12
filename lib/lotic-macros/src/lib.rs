use proc_macro::TokenStream;

mod declare_program;
mod instruction;
mod instruction_accounts;

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
