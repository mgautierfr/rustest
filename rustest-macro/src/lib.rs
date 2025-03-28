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

/// This macro automatically adds tests function marked with #[test] to the test collection.
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

/// This macro automatically adds tests marked with #[test] to the test collection.
/// Tests then can be run with libtest_mimic_collect::TestCollection::run().
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

#[proc_macro]
pub fn main(_item: TokenStream) -> TokenStream {
    let test_ctors: Vec<_> =
        std::mem::take::<Vec<String>>(TEST_COLLECTORS.lock().unwrap().as_mut())
            .into_iter()
            .map(|s| Ident::new(&s, Span::call_site()))
            .collect();

    (quote! {
        const TEST_CTORS: &[fn (&mut ::rustest::FixtureRegistry) -> ::std::result::Result<Vec<::rustest::Test>, ::rustest::FixtureCreationError>] = &[
            #(#test_ctors),*
        ];

        fn main() -> std::process::ExitCode {
            use ::rustest::libtest_mimic::{Arguments, Trial, run};
            let args = Arguments::from_args();

            let mut global_registry = ::rustest::FixtureRegistry::new();

            let tests: ::std::result::Result<Vec<_>, ::rustest::FixtureCreationError> = TEST_CTORS
                .iter()
                .map(|test_ctor| Ok(test_ctor(&mut global_registry)?))
                .collect();

            let tests = match tests {
                Ok(tests) => tests.into_iter().flatten().map(|t| t.into()).collect(),
                Err(e) => {
                    eprintln!("Failed to create fixture {}: {}", e.fixture_name, e.error);
                    return std::process::ExitCode::FAILURE;
                }
            };
            let conclusion = run(&args, tests);
            conclusion.exit_code()
        }
    })
    .into()
}
