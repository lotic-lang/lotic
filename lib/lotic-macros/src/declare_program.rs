use {
    cargo_metadata::MetadataCommand,
    pinocchio::Address,
    proc_macro::TokenStream,
    quote::quote,
    serde::Deserialize,
    std::fs,
    syn::{parse_macro_input, Ident, LitStr},
};

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(dead_code)]
enum ArgDetail {
    Simple(String),
    Complex {
        name: String,
        fields: Vec<FieldDetail>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct EnumVariant {
    name: String,
    fields: Option<Vec<FieldDetail>>,
    is_named: bool,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct FieldDetail {
    name: String,
    r#type: ArgDetail,
    is_option: bool,
    is_result: bool,
    is_vec: bool,
    is_set: bool,
    is_map: bool,
    key_type: Option<Box<ArgDetail>>,
    array_length: Option<usize>,
    error_type: Option<Box<ArgDetail>>,
}

#[derive(Deserialize)]
struct InstructionFn {
    ix_name: String,
    ix_args: Vec<FieldDetail>,
}

impl ArgDetail {
    pub fn name(&self) -> &str {
        match self {
            ArgDetail::Simple(n) => n,
            ArgDetail::Complex { name, .. } => name,
            ArgDetail::Enum { name, .. } => name,
        }
    }
}

fn read_instructions() -> Option<String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let local_manifest_path = std::path::Path::new(&manifest_dir).join("Cargo.toml");

    let metadata = MetadataCommand::new()
        .manifest_path(&local_manifest_path)
        .no_deps()
        .exec()
        .expect("Failed to fetch cargo metadata for the current crate");

    let package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.manifest_path == local_manifest_path)
        .unwrap_or_else(|| {
            panic!(
                "Could not find package info for manifest at {:?}",
                local_manifest_path
            )
        });

    let package_name = &package.name;

    let json_path = metadata
        .target_directory
        .join(format!("{package_name}-instructions.json"));

    fs::read_to_string(json_path).ok()
}

fn generate_deserializer(field_name: &Ident, field: &FieldDetail) -> proc_macro2::TokenStream {
    let inner_logic = match &field.r#type {
        ArgDetail::Complex { name, fields } if is_virtual_wrapper(name) => {
            let next_layer = fields
                .first()
                .expect("Virtual container missing inner field");
            let inner_val_name = quote::format_ident!("_inner_val");
            let inner_read = generate_deserializer(&inner_val_name, next_layer);
            quote! { { #inner_read #inner_val_name } }
        }
        _ => {
            let read = generate_arg_read(&field.r#type);
            quote! { { #read } }
        }
    };

    if field.is_option {
        return quote! {
            let #field_name = if _args[offset] == 0 {
                offset += 1;
                None
            } else {
                offset += 1;
                Some(#inner_logic)
            };
        };
    }

    if field.is_vec || field.is_set {
        let constructor = if field.is_set {
            quote! { ::std::collections::HashSet::with_capacity(len) }
        } else {
            quote! { Vec::with_capacity(len) }
        };
        let inserter = if field.is_set {
            quote! { insert }
        } else {
            quote! { push }
        };

        return quote! {
            let #field_name = {
                let len = u32::from_le_bytes(_args[offset..offset+4].try_into().unwrap()) as usize;
                offset += 4;
                let mut container = #constructor;
                for _ in 0..len {
                    container.#inserter(#inner_logic);
                }
                container
            };
        };
    }

    if field.is_map {
        let key_type = field.key_type.as_ref().expect("Map missing key_type");
        let key_read = generate_arg_read(key_type);
        return quote! {
            let #field_name = {
                let len = u32::from_le_bytes(_args[offset..offset+4].try_into().unwrap()) as usize;
                offset += 4;
                let mut map = ::std::collections::HashMap::with_capacity(len);
                for _ in 0..len {
                    let k = { #key_read };
                    let v = #inner_logic;
                    map.insert(k, v);
                }
                map
            };
        };
    }

    if let Some(len) = field.array_length {
        return quote! {
            let #field_name: [_; #len] = {
                let mut temp_vec = Vec::with_capacity(#len);
                for _ in 0..#len {
                    temp_vec.push(#inner_logic);
                }
                temp_vec.try_into().unwrap_or_else(|_| panic!("Array length mismatch"))
            };
        };
    }

    if field.is_result {
        let err_type = field
            .error_type
            .as_ref()
            .expect("Result missing error_type");
        let err_read = generate_arg_read(err_type);
        return quote! {
            let #field_name = if _args[offset] == 1 {
                offset += 1;
                Ok(#inner_logic)
            } else {
                offset += 1;
                Err({ #err_read })
            };
        };
    }

    quote! {
        let #field_name = #inner_logic;
    }
}

fn is_virtual_wrapper(name: &str) -> bool {
    matches!(name, "Array" | "Vec" | "Option" | "Result" | "Set" | "Map")
}

fn generate_arg_read(arg: &ArgDetail) -> proc_macro2::TokenStream {
    match arg {
        ArgDetail::Simple(name) => {
            let type_ident = Ident::new(name.as_str(), proc_macro2::Span::call_site());
            match name.as_str() {
                "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "u128" | "i128" => {
                    let width: usize = match name.as_str() {
                        "u8" | "i8" => 1,
                        "u16" | "i16" => 2,
                        "u32" | "i32" => 4,
                        "u64" | "i64" => 8,
                        "u128" | "i128" => 16,
                        _ => unreachable!(),
                    };
                    quote! {
                        {
                            let val = #type_ident::from_le_bytes(_args[offset..offset+#width].try_into().unwrap());
                            offset += #width;
                            val
                        }
                    }
                }
                "bool" => quote! { { let v = _args[offset] != 0; offset += 1; v } },
                "String" => quote! {
                    {
                        let len = u32::from_le_bytes(_args[offset..offset+4].try_into().unwrap()) as usize;
                        offset += 4;
                        let s = core::str::from_utf8(&_args[offset..offset+len]).unwrap().to_owned();
                        offset += len;
                        s
                    }
                },
                "Address" => quote! {
                    {
                        let addr = ::lotic::pinocchio::Address::new_from_array(_args[offset..offset+32].try_into().unwrap());
                        offset += 32;
                        addr
                    }
                },
                _other => {
                    // Handle custom types that might not be in our registry but implement Borsh
                    quote! { #type_ident::deserialize(&mut &_args[offset..]).unwrap() }
                }
            }
        }
        ArgDetail::Complex { name, fields } => {
            let struct_name = Ident::new(name, proc_macro2::Span::call_site());
            let field_reads: Vec<_> = fields
                .iter()
                .map(|f| {
                    let f_name = Ident::new(&f.name, proc_macro2::Span::call_site());
                    generate_deserializer(&f_name, f)
                })
                .collect();

            let field_names: Vec<_> = fields
                .iter()
                .map(|f| Ident::new(&f.name, proc_macro2::Span::call_site()))
                .collect();

            quote! {
                {
                    #( #field_reads )*
                    #struct_name { #( #field_names ),* }
                }
            }
        }
        ArgDetail::Enum { name, variants } => {
            let enum_name = Ident::new(name, proc_macro2::Span::call_site());
            let variant_arms: Vec<_> = variants.iter().enumerate().map(|(i, v)| {
                let tag = i as u8;
                let v_name = Ident::new(&v.name, proc_macro2::Span::call_site());

                if let Some(v_fields) = &v.fields {
                    let field_reads: Vec<_> = v_fields.iter().enumerate().map(|(fi, fa)| {
                        let f_local_name = quote::format_ident!("f{}", fi);
                        generate_deserializer(&f_local_name, fa)
                    }).collect();

                    if v.is_named {
                        let assignments: Vec<_> = v_fields.iter().enumerate().map(|(fi, fa)| {
                            let f_ident = Ident::new(&fa.name, proc_macro2::Span::call_site());
                            let f_local = quote::format_ident!("f{}", fi);
                            quote! { #f_ident: #f_local }
                        }).collect();
                        quote! { #tag => { #( #field_reads )* #enum_name::#v_name { #( #assignments ),* } } }
                    } else {
                        let locals: Vec<_> = (0..v_fields.len()).map(|fi| quote::format_ident!("f{}", fi)).collect();
                        quote! { #tag => { #( #field_reads )* #enum_name::#v_name ( #( #locals ),* ) } }
                    }
                } else {
                    quote! { #tag => #enum_name::#v_name }
                }
            }).collect();

            quote! {
                {
                    let tag = _args[offset];
                    offset += 1;
                    match tag {
                        #( #variant_arms, )*
                        _ => return Err(::lotic::pinocchio::error::ProgramError::InvalidInstructionData.into()),
                    }
                }
            }
        }
    }
}

pub fn declare_program(input: TokenStream) -> TokenStream {
    let program_id_lit = parse_macro_input!(input as LitStr);

    let decoded = match bs58::decode(&program_id_lit.value()).into_vec() {
        Ok(v) => v,
        Err(_) => {
            return syn::Error::new_spanned(program_id_lit, "invalid base58 Solana program id")
                .to_compile_error()
                .into();
        }
    };

    if decoded.len() != 32 {
        return syn::Error::new_spanned(
            program_id_lit,
            "program id must decode to exactly 32 bytes",
        )
        .to_compile_error()
        .into();
    }

    let pubkey = Address::from_str_const(&program_id_lit.value());
    if !Address::is_on_curve(&pubkey) {
        return syn::Error::new_spanned(
            program_id_lit,
            "program id must be a non-PDA (on-curve) Solana address",
        )
        .to_compile_error()
        .into();
    }

    let program_id_bytes = decoded.iter();

    let instructions: Vec<InstructionFn> = read_instructions()
        .and_then(|json| {
            let res = serde_json::from_str::<Vec<InstructionFn>>(&json);
            if let Err(ref e) = res {
                eprintln!("Failed to deserialize instructions: {}", e);
            }
            res.ok()
        })
        .unwrap_or_default();

    let mut arms = Vec::new();

    for (index, inst) in instructions.iter().enumerate() {
        let discriminator = index as u8;
        let ix_handler = Ident::new(&inst.ix_name, proc_macro2::Span::call_site());
        let ctx_field = inst.ix_args.first().expect("Missing Context");
        let ctx_type = Ident::new(ctx_field.r#type.name(), proc_macro2::Span::call_site());

        let mut arg_names = Vec::new();
        let mut deserializers = Vec::new();

        // Skip context, process every other data argument
        for arg in inst.ix_args.iter().skip(1) {
            let name = Ident::new(&arg.name, proc_macro2::Span::call_site());
            deserializers.push(generate_deserializer(&name, arg));
            arg_names.push(name);
        }

        arms.push(quote! {
            #discriminator => {
                let mut accounts_struct = #ctx_type::try_from(accounts)?;
                let ctx = ::lotic::Context { program_id, accounts: &mut accounts_struct };
                let mut offset: usize = 0;
                #( #deserializers )*
                #ix_handler(ctx, #( #arg_names ),*)
            }
        });
    }

    let expanded = quote! {
        pub const __PROGRAM_ID__: ::lotic::pinocchio::Address =
            ::lotic::pinocchio::Address::new_from_array([
                #( #program_id_bytes ),*
            ]);
        ::lotic::pinocchio::entrypoint!(__process_instruction__);
        #[inline(always)]
        pub fn __process_instruction__(
            program_id: &::lotic::pinocchio::Address,
            accounts: &[::lotic::pinocchio::AccountView],
            instruction_data: &[u8],
        ) -> ::lotic::pinocchio::ProgramResult {
            if program_id != &__PROGRAM_ID__ {
                return Err(::lotic::pinocchio::error::ProgramError::IncorrectProgramId);
            }

            let (discriminator, _args) = instruction_data
                .split_first()
                .ok_or(::lotic::pinocchio::error::ProgramError::InvalidInstructionData)?;

            match *discriminator {
                #( #arms, )*
                _ => Err(::lotic::pinocchio::error::ProgramError::InvalidInstructionData),
            }
        }
    };

    expanded.into()
}
