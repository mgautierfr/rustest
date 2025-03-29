mod fixture;
mod test;
mod utils;

use core::convert::From;
use fixture::{FixtureAttr, fixture_impl};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{ItemFn, parse_macro_input};
use test::{TEST_COLLECTORS, TestAttr, test_impl};

#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let args = parse_macro_input!(args as TestAttr);
    match test_impl(args, input) {
        Ok(output) => output,
        Err(output) => output,
    }
    .into()
}

#[proc_macro_attribute]
pub fn fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let args = parse_macro_input!(args as FixtureAttr);
    match fixture_impl(args, input) {
        Ok(output) => output,
        Err(output) => output,
    }
    .into()
}

#[proc_macro_attribute]
pub fn main(_args: TokenStream, _input: TokenStream) -> TokenStream {
    let test_ctors: Vec<_> =
        std::mem::take::<Vec<String>>(TEST_COLLECTORS.lock().unwrap().as_mut())
            .into_iter()
            .map(|s| Ident::new(&s, Span::call_site()))
            .collect();

    (quote! {
        const TEST_CTORS: &[::rustest::TestCtorFn] = &[
            #(#test_ctors),*
        ];

        fn main() -> std::process::ExitCode {
            ::rustest::run_tests(TEST_CTORS)
        }
    })
    .into()
}
