#![allow(clippy::cmp_owned)]

mod attr_parse;

#[macro_use]
extern crate proc_macro_error;
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{
    parse_macro_input, spanned::Spanned, token::Async, AttributeArgs, Ident, ItemFn, Signature,
};

#[proc_macro_attribute]
#[proc_macro_error]
pub fn tracing(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let ItemFn {
        attrs,
        vis,
        block,
        sig,
    } = input;

    let Signature {
        output: return_type,
        inputs: params,
        unsafety,
        asyncness,
        constness,
        abi,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        ..
    } = sig;

    if asyncness.is_some() {
        abort!(
            asyncness,
            "Unexpected async\nIf want to trace async function, consider `minitrace::trace_async`"
        );
    };

    let trace_name = ident.to_string();

    quote::quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            let _guard = common_logger::LocalSpan::enter(#trace_name);
            #block
        }
    )
    .into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn tracing_async(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);

    let ItemFn {
        attrs,
        vis,
        block,
        sig,
    } = input;

    let Signature {
        output: return_type,
        inputs: params,
        unsafety,
        asyncness,
        constness,
        abi,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        ..
    } = sig;

    let trace_name = ident.to_string();

    let body = if asyncness.is_some() {
        let async_kwd = Async { span: block.span() };
        let await_kwd = Ident::new("await", block.span());
        quote::quote_spanned! {block.span() =>
            #async_kwd move {
                #block
            }
            .in_local_span(#trace_name)
            .#await_kwd
        }
    } else {
        quote::quote_spanned! {
            block.span() => std::boxed::Box::pin({
                #block.in_local_span(#trace_name)
            })
        }
    };

    quote::quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            use common_logger::FutureExt;
            #body
        }
    )
    .into()
}
