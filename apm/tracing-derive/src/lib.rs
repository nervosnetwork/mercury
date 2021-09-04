#![allow(clippy::cmp_owned)]

mod attr_parse;
mod expand;

extern crate proc_macro;

use proc_macro::TokenStream;

use crate::expand::func_expand;

#[proc_macro_attribute]
pub fn tracing_span(attr: TokenStream, func: TokenStream) -> TokenStream {
    func_expand(attr, func)
}
