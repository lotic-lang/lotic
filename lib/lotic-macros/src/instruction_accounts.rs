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
    let mut constructors = Vec::new();

    let Data::Struct(data) = input.data else {
        return TokenStream::new();
    };

    let Fields::Named(fields) = data.fields else {
        return TokenStream::new();
    };

    for field in fields.named {
        let field_ident = field.ident.unwrap(); // Because We are expecting named structs.
        let field_type = &field.ty;
        field_idents.push(field_ident.clone());

        let constructor = if let syn::Type::Path(type_path) = field_type {
            let segment = type_path.path.segments.last().unwrap();
            let base_ident = &segment.ident;

            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                let types_only: Vec<_> = args
                    .args
                    .iter()
                    .filter(|arg| !matches!(arg, syn::GenericArgument::Lifetime(_)))
                    .collect();
                quote! { #base_ident::< 'a, #(#types_only),* >::new(#field_ident) }
            } else {
                quote! { #field_ident }
            }
        } else {
            quote! { #field_ident }
        };

        constructors.push(constructor);

        for attr in &field.attrs {
            if attr.path().is_ident("lotic") {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("signer") {
                        validations.push(quote! {
                            if !self.#field_ident.is_signer() {
                                return Err(::lotic::pinocchio::error::ProgramError::MissingRequiredSignature);
                            }
                        });
                    } else if meta.path.is_ident("mut") {
                        validations.push(quote! {
                            if !self.#field_ident.is_writable() {
                                return Err(::lotic::pinocchio::error::ProgramError::Immutable);
                            }
                        });
                    } else if meta.path.is_ident("executable") {
                        validations.push(quote! {
                            if !self.#field_ident.executable() {
                                return Err(::lotic::pinocchio::error::ProgramError::Custom(0));
                            }
                        });
                    } else if meta.path.is_ident("program") {
                        meta.value()?; // Consume =
                        let account_type: syn::Path = meta.input.parse()?;

                        if account_type.is_ident("system") {
                            validations.push(quote! {
                                let system_program_address = ::lotic::pinocchio::Address::from_str_const("11111111111111111111111111111111");
                                if self.#field_ident.address()!= &system_program_address {
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("vote") {
                            validations.push(quote! {
                                let vote_program_address = ::lotic::pinocchio::Address::from_str_const("Vote111111111111111111111111111111111111111");
                                if self.#field_ident.address()!= &vote_program_address {
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("stake") {
                            validations.push(quote! {
                                let stake_program_address = ::lotic::pinocchio::Address::from_str_const("Stake11111111111111111111111111111111111111");
                                if self.#field_ident.address()!= &stake_program_address {
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("config") {
                            validations.push(quote! {
                                let config_program_address = ::lotic::pinocchio::Address::from_str_const("Config1111111111111111111111111111111111111");
                                if self.#field_ident.address()!= &config_program_address {
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.is_ident("compute_budget") {
                            validations.push(quote! {
                                let compute_budget_program_address = ::lotic::pinocchio::Address::from_str_const("ComputeBudget111111111111111111111111111111");
                                if self.#field_ident.address()!= &compute_budget_program_address {
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.get_ident().to_token_stream().to_string().to_lowercase() == "token" {
                            validations.push(quote! {
                                let tokenkeg = ::lotic::pinocchio::Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                                let tokenz = ::lotic::pinocchio::Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
                                if self.#field_ident.address()!= &tokenkeg && self.#field_ident.address()!= &tokenz{
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.get_ident().to_token_stream().to_string().to_lowercase() == "tokenkeg" {
                            validations.push(quote! {
                                let tokenkeg = ::lotic::pinocchio::Address::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                                if self.#field_ident.address()!= &tokenkeg{
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        } else if account_type.get_ident().to_token_stream().to_string().to_lowercase() == "tokenz" {
                            validations.push(quote! {
                                let tokenz = ::lotic::pinocchio::Address::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
                                if self.#field_ident.address()!= &tokenz{
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        }
                    } else if meta.path.is_ident("sysvar") {
                        let sysvar_type: syn::Path = meta.value()?.parse()?;
                        if sysvar_type.is_ident("clock") {
                            validations.push(quote! {
                                let clock_sysvar_address = ::lotic::pinocchio::Address::from_str_const("SysvarC1ock11111111111111111111111111111111");
                                if self.#field_ident.address()!= &clock_sysvar_address {
                                    return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
                                }
                            });
                        }
                    } else if meta.path.is_ident("address") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        let address = value.value();
                        let is_valid = bs58::decode(&address).into_vec().map(|v| v.len() == 32).unwrap_or(false);
                        if !is_valid {
                            panic!("Invalid Solana address: {}", address);
                        }
                        validations.push(quote! {
                            if self.#field_ident.address()!= &::lotic::pinocchio::Address::from_str_const(#address){
                                return Err(::lotic::pinocchio::error::ProgramError::InvalidAccountData);
                            }
                        });
                    } else if meta.path.is_ident("owner") {
                        let value: syn::LitStr = meta.value()?.parse()?;
                        let address = value.value();
                        let is_valid = bs58::decode(&address).into_vec().map(|v| v.len() == 32).unwrap_or(false);
                        if !is_valid {
                            panic!("Invalid Solana address: {}", address);
                        }
                        validations.push(quote! {
                            if !self.#field_ident.owned_by(&::lotic::pinocchio::Address::from_str_const(#address)){
                                return Err(::lotic::pinocchio::error::ProgramError::InvalidAccountOwner);
                            }
                        });
                    }
                    Ok(())
                });
            }
        }
    }

    let expanded = quote! {
        impl<'a> core::convert::TryFrom<&'a mut [::lotic::pinocchio::AccountView]> for #struct_ident<'a> {
            type Error = ::lotic::pinocchio::error::ProgramError;

            fn try_from(accounts: &'a mut [::lotic::pinocchio::AccountView]) -> Result<Self, Self::Error> {
                let [#(#field_idents,)* ..] = accounts else {
                    return Err(::lotic::pinocchio::error::ProgramError::NotEnoughAccountKeys);
                };

                let accounts_struct = Self {
                    #(
                        #field_idents: #constructors,
                    )*
                };

                accounts_struct.check_constraints()?;
                Ok(accounts_struct)
            }
        }

        impl<'a> #struct_ident<'a> {
            #[inline(always)]
            fn check_constraints(&self) -> Result<(), ::lotic::pinocchio::error::ProgramError> {
                #(#validations)*
                Ok(())
            }
        }
    };

    expanded.into()
}
