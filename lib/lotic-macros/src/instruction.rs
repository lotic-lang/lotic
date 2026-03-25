use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, FnArg, ItemFn, PathArguments, Type},
};

// This will only check the signature of the instruction functions, it will
// result in error if the first argument isnt &Context<T> or if the rest of the
// argument isnt a reference.
pub fn instruction(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let inputs = &input.sig.inputs;

    for (i, arg_param) in inputs.iter().enumerate() {
        let pat_type = match arg_param {
            FnArg::Typed(pt) => pt,
            _ => continue,
        };

        let inner_type = if let Type::Reference(r) = &*pat_type.ty {
            &*r.elem
        } else {
            &*pat_type.ty
        };

        if i == 0 {
            if let Type::Path(type_path) = inner_type {
                let segment = type_path.path.segments.last().unwrap();
                let has_generics = matches!(segment.arguments, PathArguments::AngleBracketed(_));

                if segment.ident != "Context" || !has_generics {
                    return syn::Error::new_spanned(&pat_type.ty, "First arg must be Context<T>")
                        .to_compile_error()
                        .into();
                }
            } else {
                return syn::Error::new_spanned(&pat_type.ty, "First arg must be Context<T>")
                    .to_compile_error()
                    .into();
            }
        }
    }

    quote! {
        #[allow(dead_code)]
        #input
    }
    .into()
}
