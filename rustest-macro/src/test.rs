use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::sync::Mutex;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, FnArg, ItemFn, LitStr};

pub(crate) static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn is_xfail(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("xfail"))
}

pub(crate) struct TestAttr {
    xfail: bool,
}

impl Parse for TestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut xfail = false;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "xfail" {
                xfail = true;
            } else {
                return Err(input.error("unexpected attribute"));
            }
            if input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(TestAttr { xfail })
    }
}

pub(crate) fn test_impl(args: TestAttr, input: ItemFn) -> Result<TokenStream, TokenStream> {
    let ItemFn {
        sig, block, attrs, ..
    } = input;
    let TestAttr { xfail } = args;

    let ident = &sig.ident;
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let ctor_name = format!("__{}_add_test", test_name);
    let ctor_ident = Ident::new(&ctor_name, Span::call_site());

    let is_xfail = xfail || is_xfail(&attrs);

    let mut fixtures = vec![];
    let mut test_args = vec![];

    for fnarg in sig.inputs.iter() {
        if let FnArg::Typed(typed_fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = typed_fnarg.pat.as_ref() {
                &patident.ident
            } else {
                return Err(
                    syn::Error::new_spanned(fnarg, "expected an identifier").to_compile_error()
                );
            };
            fixtures.push(quote! {
                ::rustest::get_fixture(ctx)?
            });
            test_args.push(quote! {#pat});
        }
    }

    TEST_COLLECTORS.lock().unwrap().push(ctor_ident.to_string());

    Ok(quote! {
        #sig #block

        fn #ctor_ident(global_reg: &mut ::rustest::FixtureRegistry)
            -> ::std::result::Result<Vec<::rustest::Test>, ::rustest::FixtureCreationError> {
            use ::rustest::IntoError;
            let mut test_registry = ::rustest::FixtureRegistry::new();
            let mut ctx = ::rustest::TestContext::new(global_reg, &mut test_registry);
            let ctx = &mut ctx;
            let fixtures_matrix = ::rustest::FixtureMatrix::new();
            #(let fixtures_matrix = fixtures_matrix.feed(#fixtures);)*
            let matrix_caller: ::rustest::MatrixCaller<_> = fixtures_matrix.into();
            let runner = |#(#test_args),*| {#ident(#(#test_args),*).into_error()};
            let test_runners = matrix_caller.call(runner);
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

        let args = TestAttr { xfail: false };

        let result = test_impl(args, input);
        assert!(result.is_ok());
    }
}
