mod fixture;
mod test;
mod utils;

use fixture::{FixtureAttr, fixture_impl};
use proc_macro::TokenStream;
use quote::quote;
use std::sync::atomic::Ordering;
use syn::{ItemFn, parse_macro_input};
use test::{TEST_COUNT, TestAttr, test_impl};

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
    let test_count = TEST_COUNT.load(Ordering::Relaxed);
    (quote! {
        static mut TEST_GENERATORS: [Option<::rustest::TestGeneratorFn>; #test_count] = [None; #test_count];

        fn main() -> std::process::ExitCode {
            // SAFETY: TEST_CTORS is filled only by functions run from outside of main.
            // So when we are here, no one is modifying (neither read) it.
            let test_registers = unsafe { TEST_GENERATORS.iter().map(|r| r.expect("Slot should be filled")).collect::<Vec<_>>() };
            ::rustest::run_tests(&test_registers)
        }
    })
    .into()
}
