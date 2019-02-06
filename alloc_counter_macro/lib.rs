#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::*;
use syn::{
    parse_macro_input, parse_quote, ArgCaptured, ArgSelf, ArgSelfRef, AttributeArgs, FnArg, ItemFn,
    NestedMeta,
};

#[proc_macro_attribute]
/// Macro for marking functions as unable to allocate.
///
/// By default, will panic on allocations not running in an `allow_alloc` closure.
/// To panic on all allocations, use `#[no_alloc(forbid)]`. To conditionally enable
/// panicking, the attribute can be wrapped:
///
/// ```rust,ignore
/// // Panic only when running debug builds
/// #[cfg_attr(debug_assertions, no_alloc)]
/// fn my_function() {
///
/// }
/// ```
pub fn no_alloc(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let mut item = parse_macro_input!(item as ItemFn);

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
    let force_move = item.decl.inputs.iter().filter_map(|a| match a {
        FnArg::SelfRef(ArgSelfRef { .. }) => None,
        // we cannot force the move of self without rewriting all instances of self in the body,
        // which may generate incorrect code. so instead we check that Self: Copy, because Copy and
        // Drop are mutually exclusive and we cannot yet check Self: !Drop.
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

    let block = item.block;
    let output = &item.decl.output;

    item.block =
        if item.asyncness.is_none() {
            parse_quote!({
                alloc_counter::#mode(move || #output {
                    #( #force_move )*
                    #self_hack
                    #block
                })
            })
        } else {
            parse_quote!({
                use core::{ops::{Generator, GeneratorState}, pin::Pin};

                let mut gen = move || #output {
                    // make sure this becomes a generator
                    if false { yield }
                    #( #force_move )*
                    #self_hack
                    #block
                };

                loop {
                    match alloc_counter::#mode(|| Pin::new(&mut gen).resume()) {
                        GeneratorState::Yielded(y) => yield y,
                        GeneratorState::Complete(r) => return r,
                    }
                }
            })
        };

    item.into_token_stream().into()
}

#[proc_macro_attribute]
/// Macro for counting allocations inside a functions.
///
/// After calling the function the message:
///
/// ```text
/// "Function #fn allocated {}, reallocated {}, deallocated {}"
/// ```
///
/// is printed to stderr.
///
/// ```rust,ignore
/// // Panic only when running debug builds
/// #[cfg_attr(debug_assertions, no_alloc)]
/// fn my_function() {
///
/// }
/// ```
pub fn count_alloc(_args: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as ItemFn);

    let mut self_hack = None;
    let force_move = item.decl.inputs.iter().filter_map(|a| match a {
        FnArg::SelfRef(ArgSelfRef { .. }) => None,
        // we cannot force the move of self without rewriting all instances of self in the body,
        // which may generate incorrect code. so instead we check that Self: Copy, because Copy and
        // Drop are mutually exclusive and we cannot yet check Self: !Drop.
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

    let ident = &item.ident;
    let block = item.block;
    let output = &item.decl.output;

    item.block =
        if item.asyncness.is_none() {
            parse_quote!({
                let (c, r) = count_alloc(move || #output {
                    #( #force_move )*
                    #self_hack
                    #block
                });
                eprintln!(
                    "Function {} allocated {}, reallocated {}, and deallocated {}",
                    stringify!(#ident),
                    c.0,
                    c.1,
                    c.2
                );
                r
            })
        } else {
            parse_quote!({
                use core::{ops::{Generator, GeneratorState}, pin::Pin};

                let mut gen = move || #output {
                    // make sure this becomes a generator
                    if false { yield }
                    #( #force_move )*
                    #self_hack
                    #block
                };
                let mut count = (0, 0, 0);

                loop {
                    match count_alloc(|| Pin::new(&mut gen).resume()) {
                        (c, GeneratorState::Yielded(y)) => {
                            count = (count.0 + c.0, count.1 + c.1, count.2 + c.2);
                            yield y
                        }
                        (c, GeneratorState::Complete(r)) => {
                            count = (count.0 + c.0, count.1 + c.1, count.2 + c.2);
                            eprintln!(
                                "Function {} allocated {}, reallocated {}, and deallocated {}",
                                #ident,
                                count.0,
                                count.1,
                                count.2
                            );
                            return t
                        }
                    }
                }
            })
        };

    item.into_token_stream().into()
}
