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
    let sig = &input.sig;
    let inputs = &sig.inputs;

    // Allowed primitive types
    let primitives = [
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
        "f32", "f64", "bool", "char", "str",
    ];

    for (i, arg_param) in inputs.iter().enumerate() {
        if let FnArg::Typed(pat_type) = arg_param {
            let ty = &pat_type.ty;

            if let Type::Reference(ref_type) = &**ty {
                let inner_type = &*ref_type.elem;

                if i == 0 {
                    if let Type::Path(type_path) = inner_type {
                        let segment = type_path.path.segments.last().unwrap();
                        let has_generics =
                            matches!(segment.arguments, PathArguments::AngleBracketed(_));

                        if segment.ident != "Context" || !has_generics {
                            return syn::Error::new_spanned(ty, "First arg must be &Context<T>")
                                .to_compile_error()
                                .into();
                        }
                    } else {
                        return syn::Error::new_spanned(ty, "First arg must be &Context<T>")
                            .to_compile_error()
                            .into();
                    }
                } else {
                    if let Type::Path(type_path) = inner_type {
                        let type_name = quote!(#type_path).to_string();
                        if !primitives.contains(&type_name.as_str()) {
                            return syn::Error::new_spanned(
                                ty,
                                format!(
                                    "Argument '{}' is not a supported primitive reference.",
                                    type_name
                                ),
                            )
                            .to_compile_error()
                            .into();
                        }
                    } else {
                        return syn::Error::new_spanned(
                            ty,
                            "Argument must be a primitive reference.",
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
        }
    }

    quote! {
        #[allow(dead_code)]
        #input
    }
    .into()
}
