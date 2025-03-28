use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::sync::Mutex;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, ItemFn, LitStr};

use crate::utils::gen_fixture_call;

pub(crate) static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn is_xfail(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("xfail"))
}

pub(crate) struct TestAttr {
    xfail: bool,
    params: Option<syn::Expr>,
}

impl Parse for TestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut xfail = false;
        let mut params = None;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "xfail" => {
                    xfail = true;
                }
                "params" => {
                    let _: syn::Token![=] = input.parse()?;
                    params = Some(input.parse()?);
                }

                _ => {
                    return Err(input.error("unexpected attribute"));
                }
            }
            if input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(TestAttr { xfail, params })
    }
}

pub(crate) fn test_impl(args: TestAttr, input: ItemFn) -> Result<TokenStream, TokenStream> {
    let ItemFn {
        sig, block, attrs, ..
    } = input;
    let TestAttr { xfail, params } = args;

    let ident = &sig.ident;
    let sig_inputs = &sig.inputs;
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let ctor_name = format!("__{}_add_test", test_name);
    let ctor_ident = Ident::new(&ctor_name, Span::call_site());

    let is_xfail = xfail || is_xfail(&attrs);

    let (fixtures_build, call_args) = gen_fixture_call(params.as_ref(), &sig)?;

    TEST_COLLECTORS.lock().unwrap().push(ctor_ident.to_string());

    Ok(quote! {
        #sig #block

        fn #ctor_ident(ctx: &mut ::rustest::TestContext)
            -> ::std::result::Result<Vec<::rustest::Test>, ::rustest::FixtureCreationError> {
            use ::rustest::IntoError;
            let fixtures_matrix = ::rustest::FixtureMatrix::new();
            #(let fixtures_matrix = fixtures_matrix.feed(#fixtures_build);)*
            let runner = |#sig_inputs| {#ident(#(#call_args),*).into_error()};
            let test_runners = fixtures_matrix.call(runner);
            let test_name = if test_runners.len() > 1 {
                |name| format!("{}[{}]", #test_name_str, name)
            } else {
                |name| #test_name_str.to_owned()
            };

            let tests = test_runners.into_iter()
                .map(|(name, runner)| ::rustest::Test::new(
                    test_name(name), #is_xfail, runner)
                )
                .collect::<Vec<_>>();
            Ok(tests)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ItemFn, parse_quote, parse2};

    #[test]
    fn test_parse_test_attr() {
        let input = quote! {
            xfail
        };

        let attr = parse2::<TestAttr>(input).unwrap();
        assert!(attr.xfail);
    }

    #[test]
    fn test_parse_test_attr_invalid() {
        let input = quote! {
            invalid_attr
        };

        let result = parse2::<TestAttr>(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_test_impl() {
        let input: ItemFn = parse_quote! {
            fn my_test() {
                assert_eq!(1 + 1, 2);
            }
        };

        let args = TestAttr {
            xfail: false,
            params: None,
        };

        let result = test_impl(args, input);
        assert!(result.is_ok());
    }
}
