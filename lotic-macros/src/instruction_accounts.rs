use {
    proc_macro::TokenStream,
    quote::{quote, ToTokens},
    syn::{parse_macro_input, Data, DeriveInput, Fields},
};

pub fn instruction_accounts(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident;

    let mut validations = Vec::new();
    let mut field_idents = Vec::new();

    let Data::Struct(data) = input.data else {
        return TokenStream::new();
    };

    let Fields::Named(fields) = data.fields else {
        return TokenStream::new();
    };

    for field in fields.named {
        let field_ident = field.ident.unwrap(); // Because We are expecting named structs.
        field_idents.push(field_ident.clone());

        for attr in &field.attrs {
            if attr.path().is_ident("lotic") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("signer") {
                        validations.push(quote! {
                            if !self.#field_ident.is_signer() {
                                return Err(::pinocchio::error::ProgramError::MissingRequiredSignature);
                            }
                        });
                    } else if meta.path.is_ident("mut") {
                        validations.push(quote! {
                            if !self.#field_ident.is_writable() {
                                return Err(::pinocchio::error::ProgramError::Immutable);
                            }
                        });
                    } else if meta.path.is_ident("program") {
                        meta.value()?; // Consume =
                        let account_type: syn::Path = meta.input.parse()?;

                        if account_type.is_ident("system") {
                            validations.push(quote! {
                                let system_program_address = ::pinocchio::Address::from_str_const("11111111111111111111111111111111");
                                if self.#field_ident.address()!= &system_program_address {
                                    return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("vote") {
                            validations.push(quote! {
                                let vote_program_address = ::pinocchio::Address::from_str_const("Vote111111111111111111111111111111111111111");
                                if self.#field_ident.address()!= &vote_program_address {
                                    return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("stake") {
                            validations.push(quote! {
                                let stake_program_address = ::pinocchio::Address::from_str_const("Stake11111111111111111111111111111111111111");
                                if self.#field_ident.address()!= &stake_program_address {
                                    return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("config") {
                            validations.push(quote! {
                                let config_program_address = ::pinocchio::Address::from_str_const("Config1111111111111111111111111111111111111");
                                if self.#field_ident.address()!= &config_program_address {
                                    return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("compute_budget") {
                            validations.push(quote! {
                                let compute_budget_program_address = ::pinocchio::Address::from_str_const("ComputeBudget111111111111111111111111111111");
                                if self.#field_ident.address()!= &compute_budget_program_address {
                                    return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.get_ident().to_token_stream().to_string().to_lowercase() == "token" {
                            validations.push(quote! {
                                let tokenkeg = ::pinocchio::Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                                let tokenz = ::pinocchio::Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
                                if self.#field_ident.address()!= &tokenkeg && self.#field_ident.address()!= &tokenz{
                                    return Err(::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.get_ident().to_token_stream().to_string().to_lowercase() == "tokenkeg" {
                            validations.push(quote! {
                                let tokenkeg = ::pinocchio::Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                                if self.#field_ident.address()!= &tokenkeg{
                                    return Err(ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.get_ident().to_token_stream().to_string().to_lowercase() == "tokenz" {
                            validations.push(quote! {
                                let tokenz = ::pinocchio::Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
                                if self.#field_ident.address()!= &tokenz{
                                    return Err(ProgramError::IncorrectProgramId);
                                }
                            });
                        }
                    }
                    Ok(())
                });
            }
        }
    }

    let expanded = quote! {
        impl <'view> core::convert::TryFrom<&'view [AccountView]> for #struct_ident <'view>  {
            type Error = ::pinocchio::error::ProgramError;

            fn try_from(accounts: &'view [AccountView]) -> Result<Self, Self::Error> {
                let [#(#field_idents,)* ..] = accounts else {
                    return Err(::pinocchio::error::ProgramError::NotEnoughAccountKeys);
                };

                let accounts = Self {
                    #(#field_idents,)*
                };

                accounts.check_constraints()?;
                Ok(accounts)
            }
        }

        impl <'view> #struct_ident <'view> {
            fn check_constraints(&self) -> Result<(), ::pinocchio::error::ProgramError> {
                #(#validations)*
                Ok(())
            }
        }
    };

    expanded.into()
}
