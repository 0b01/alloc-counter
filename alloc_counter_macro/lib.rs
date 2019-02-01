extern crate proc_macro;

use proc_macro::TokenStream;
use proc_quote::quote;
use syn::{
    parse_macro_input, ArgCaptured, ArgSelf, ArgSelfRef, AttributeArgs, FnArg, FnDecl, ItemFn,
    NestedMeta,
};

#[proc_macro_attribute]
pub fn no_alloc(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);

    let ItemFn {
        attrs,
        vis,
        constness,
        unsafety,
        asyncness,
        ident,
        decl,
        block,
        ..
    } = parse_macro_input!(item as ItemFn);

    let FnDecl {
        generics,
        inputs,
        output,
        ..
    } = *decl;

    let call_inputs = inputs.iter().map(|a| match a {
        FnArg::SelfRef(ArgSelfRef { self_token, .. })
        | FnArg::SelfValue(ArgSelf { self_token, .. }) => quote!( #self_token ),
        FnArg::Captured(ArgCaptured { pat, .. }) => quote!( #pat ),
        _ => panic!("FIXME: unhandled function argument in #[no_alloc]."),
    });

    let mut mode = quote!(deny_alloc);
    for arg in &args {
        match arg {
            NestedMeta::Meta(meta) if meta.name() == "forbid" => mode = quote!(forbid_alloc),
            NestedMeta::Meta(meta) => {
                panic!("Invalid meta argument for #[no_alloc]. {}", quote!(#meta))
            }
            NestedMeta::Literal(lit) => {
                panic!("Invalid literal argument for #[no_alloc]. {}", quote!(#lit))
            }
        }
    }

    let attrs = &attrs;

    quote!(
        #( #attrs )*
        #vis #constness #unsafety #asyncness
        fn #ident #generics (#inputs) #output {
            #( #attrs )*
            #vis #constness #unsafety #asyncness
            fn _inner #generics (#inputs) #output #block
            alloc_counter::#mode(move || _inner(#( #call_inputs ),*))
        }
    )
    .into()
}
