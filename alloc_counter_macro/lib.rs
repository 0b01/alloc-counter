extern crate proc_macro;

use core::str::FromStr;
use proc_macro::TokenStream;
use quote::*;
use syn::{
    parse_macro_input, parse_quote, AttributeArgs, FnArg, ItemFn, Lit, Meta, MetaNameValue,
    NestedMeta, Pat, PatIdent, PatType, Receiver,
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

    let mut mode = quote!(alloc_counter::AllocMode::Count);
    for arg in &args {
        match arg {
            NestedMeta::Meta(meta) if meta.path().is_ident("forbid") => {
                mode = quote!(alloc_counter::AllocMode::CountAll);
            }
            NestedMeta::Meta(meta) if meta.path().is_ident("allow") => {
                mode = quote!(alloc_counter::AllocMode::Ignore);
            }
            NestedMeta::Meta(meta) => {
                panic!("Invalid meta argument for #[no_alloc]. {}", quote!(#meta));
            }
            NestedMeta::Lit(lit) => {
                panic!("Invalid literal argument for #[no_alloc]. {}", quote!(#lit));
            }
        }
    }

    let mut self_hack = None;
    let force_move = item.sig.inputs.iter().filter_map(|a| match a {
        FnArg::Receiver(Receiver {
            reference: None, self_token, ..
        }) => {
            self_hack = Some(quote!(
                let _ = || {
                    fn assert_is_copy<T: Copy>(_: T) {}
                    assert_is_copy(#self_token);
                };
            ));
            None
        }
        FnArg::Receiver(_) => None,
        FnArg::Typed(PatType { pat, .. }) => match &**pat {
            Pat::Ident(PatIdent {
                mutability, ident, ..
            }) => {
                // FIXME: remove 'mut' from the function's arguments
                Some(quote!( let #mutability #ident = #ident; ))
                }
            _ => panic!("unhandled pattern type"),
        },
    });

    let block = item.block;
    let output = &item.sig.output;

    item.block = if item.sig.asyncness.is_some() {
        parse_quote!({
            alloc_counter::guard_future(
                #mode,
                async {
                    #( #force_move )*
                    #self_hack
                    #block
                }
            ).await
        })
    } else {
        parse_quote!({
            alloc_counter::guard_fn(#mode, move || #output {
                #( #force_move )*
                #self_hack
                #block
            })
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
/// #[cfg_attr(debug_assertions, count_alloc)]
/// fn my_function() {
///
/// }
/// ```
pub fn count_alloc(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as ItemFn);
    let ident = &item.sig.ident;
    let block = item.block;
    let output = &item.sig.output;

    let mut callback = quote!({
        |(allocs, reallocs, deallocs)| {
            eprintln!(
                "Function {} allocated {}, reallocated {}, and deallocated {}",
                stringify!(#ident),
                allocs,
                reallocs,
                deallocs,
            )
        }
    });
    let args = parse_macro_input!(args as AttributeArgs);
    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                path,
                lit: Lit::Str(s),
                ..
            })) if path.is_ident("func") => {
                callback = TokenStream::from_str(&*s.value()).expect("fixme").into();
            }
            NestedMeta::Meta(meta) => {
                panic!("Invalid meta argument for #[no_alloc]. {}", quote!(#meta));
            }
            NestedMeta::Lit(lit) => {
                panic!("Invalid literal argument for #[no_alloc]. {}", quote!(#lit));
            }
        }
    }

    let mut self_hack = None;
    let force_move = item.sig.inputs.iter().filter_map(|a| match a {
        FnArg::Receiver(Receiver {
            reference: None, self_token, ..
        }) => {
            self_hack = Some(quote!(
                let _ = || {
                    fn assert_is_copy<T: Copy>(_: T) {}
                    assert_is_copy(#self_token);
                };
            ));
            None
        }
        FnArg::Receiver(_) => None,
        FnArg::Typed(PatType { pat, .. }) => match &**pat {
            Pat::Ident(PatIdent {
                mutability, ident, ..
            }) => Some(quote!( let #mutability #ident = #ident; )),
            _ => panic!("unhandled pattern type"),
        },
    });

    let res = if item.sig.asyncness.is_some() {
        quote!({
            count_alloc_future(async {
                #( #force_move )*
                #self_hack
                #block
            }).await
        })
    } else {
        quote!({
            count_alloc(move || #output {
                #( #force_move )*
                #self_hack
                #block
            })
        })
    };

    item.block = parse_quote!({
        let (counts, x) = #res;
        (#callback)(counts);
        x
    });

    item.into_token_stream().into()
}
