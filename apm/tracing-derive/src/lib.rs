#![allow(clippy::cmp_owned)]

mod attr_parse;

#[macro_use]
extern crate proc_macro_error;
extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Signature, token::Async, spanned::Spanned, Ident};

#[proc_macro_attribute]
#[proc_macro_error]
pub fn tracing(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let tracing_attrs = attr_parse::parse_attrs(parse_macro_input!(args as AttributeArgs));

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

    let span_tag_stmts = tracing_attrs
        .get_tag_map()
        .into_iter()
        .map(|(key, val)| attr_parse::span_tag(key, val))
        .collect::<Vec<_>>();

    quote::quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            let mut span_tags: Vec<(&'static str, String)> = Vec::new();
            #(#span_tag_stmts)*

            let _guard = common_logger::LocalSpan::enter(#ident).with_property(span_tags.iter());
            #block
        }
    )
    .into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn tracing_async(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemFn);
    let tracing_attrs = attr_parse::parse_attrs(parse_macro_input!(args as AttributeArgs));

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

    let span_tag_stmts = tracing_attrs
        .get_tag_map()
        .into_iter()
        .map(|(key, val)| attr_parse::span_tag(key, val))
        .collect::<Vec<_>>();

    let body = if asyncness.is_some() {
        let async_kwd = Async { span: block.span() };
        let await_kwd = Ident::new("await", block.span());
        quote::quote_spanned! {block.span() =>
            #async_kwd move { 
                let mut span_tags: Vec<(&'static str, String)> = Vec::new();
                #(#span_tag_stmts)*
                #block 
            }
            .in_local_span(#ident)
            .with_property(span_tags.iter())
            .#await_kwd
        }
    } else {
        quote::quote_spanned! {
            block.span() => std::boxed::Box::pin({
                let mut span_tags: Vec<(&'static str, String)> = Vec::new();
                #(#span_tag_stmts)*
                #block.in_local_span(#ident).with_property(span_tags.iter())
            })
        }
    };

    quote::quote!(
        #(#attrs) *
        #vis #constness #unsafety #asyncness #abi fn #ident<#gen_params>(#params) #return_type
        #where_clause
        {
            #body
        }
    )
    .into()
}
