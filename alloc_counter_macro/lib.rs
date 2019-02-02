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

    let mut mode = quote!(deny_alloc);
    for arg in &args {
        match arg {
            NestedMeta::Meta(meta) if meta.name() == "forbid" => {
                mode = quote!(forbid_alloc);
            }
            NestedMeta::Meta(meta) => {
                panic!("Invalid meta argument for #[no_alloc]. {}", quote!(#meta));
            }
            NestedMeta::Literal(lit) => {
                panic!("Invalid literal argument for #[no_alloc]. {}", quote!(#lit));
            }
        }
    }

    let mut self_hack = None;
    let force_move = inputs.iter().filter_map(|a| match a {
        FnArg::SelfRef(ArgSelfRef { .. }) => None,
        // we cannot do: let self = self; because rustc is stubborn, so to prevent an uncaught drop
        // we must check that self is Copy. ideally this would only check !Drop but we can't do
        // that yet.
        FnArg::SelfValue(ArgSelf { self_token, .. }) => {
            self_hack = Some(quote!(
                fn _self_hack<T: Copy>(t: T) {}
                _self_hack(#self_token);
            ));
            None
        }
        FnArg::Captured(ArgCaptured { pat, .. }) => Some(quote!( let #pat = #pat; )),
        _ => panic!("FIXME: unhandled function argument in #[no_alloc]."),
    });

    let attrs = &attrs;

    quote!(
        #( #attrs )*
        #vis #constness #unsafety #asyncness
        fn #ident #generics (#inputs) #output {
            alloc_counter::#mode(move || {
                #( #force_move )*
                #self_hack
                #block
            })
        }
    )
    .into()
}
