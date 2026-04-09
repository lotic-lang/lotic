use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, ItemStruct},
};

pub fn account_state(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    let expanded = quote! {
        #[repr(C)]
        #[derive(Clone, Copy, ::lotic::bytemuck::Pod, ::lotic::bytemuck::Zeroable, Debug)]
        #[bytemuck(crate = "lotic::bytemuck")]
        #input
    };

    TokenStream::from(expanded)
}
