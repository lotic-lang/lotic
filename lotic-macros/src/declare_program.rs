use {
    camino::Utf8PathBuf,
    cargo_metadata::MetadataCommand,
    pinocchio::Address,
    proc_macro::TokenStream,
    quote::quote,
    serde::Deserialize,
    std::{fs, path::Path},
    syn::{parse_macro_input, Ident, LitStr},
};

#[derive(Deserialize)]
pub struct InstructionFn {
    pub ix_name: String,
    pub ix_args: Vec<String>,
}

pub fn read_instructions() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir).join("Cargo.toml");
    let metadata = MetadataCommand::new()
        .manifest_path(&manifest_path)
        .exec()
        .expect("Failed to read cargo metadata");

    let target_dir: Utf8PathBuf = metadata.target_directory;
    let json_path = target_dir.join("instructions.json");

    fs::read_to_string(&json_path)
        .unwrap_or_else(|_| panic!("Failed to read instructions.json at {}", json_path))
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

    let instructions: Vec<InstructionFn> =
        serde_json::from_str(&read_instructions()).expect("Invalid instructions.json");

    let mut arms = Vec::new();

    for (index, inst) in instructions.iter().enumerate() {
        let discriminator = index as u8; // sequential discriminator starting from 0
        let ix_handler = Ident::new(&inst.ix_name, proc_macro2::Span::call_site());

        let ctx_type_str = &inst.ix_args[0];
        let ctx_type = Ident::new(ctx_type_str, proc_macro2::Span::call_site());

        arms.push(quote! {
            #discriminator => {
                let ctx = ::lotic::Context {
                    program_id,
                    accounts: &mut #ctx_type::try_from(accounts)?,
                };
                #ix_handler(ctx)
            }
        });
    }

    let expanded = quote! {
        pub const __PROGRAM_ID__: ::lotic::pinocchio::Address =
            ::lotic::pinocchio::Address::new_from_array([
                #( #program_id_bytes ),*
            ]);
        entrypoint!(__process_instruction__);
        #[inline(always)]
        pub fn __process_instruction__(
            program_id: &::lotic::pinocchio::Address,
            accounts: &[::lotic::pinocchio::AccountView],
            instruction_data: &[u8],
        ) -> ProgramResult {
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
    // TokenStream::new()
}
