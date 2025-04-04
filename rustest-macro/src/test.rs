use core::sync::atomic::AtomicUsize;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, ItemFn, LitStr};

use crate::utils::{gen_fixture_call, gen_param_fixture};

pub(crate) static TEST_COUNT: AtomicUsize = AtomicUsize::new(0);

fn is_xfail(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("xfail"))
}

#[derive(Debug, PartialEq)]
pub(crate) struct TestAttr {
    xfail: bool,
    params: Option<(syn::Type, syn::Expr)>,
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
                    let _: syn::Token![:] = input.parse()?;
                    let ty = input.parse()?;
                    let _: syn::Token![=] = input.parse()?;
                    let expr = input.parse()?;
                    params = Some((ty, expr));
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
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let test_generator_ident = Ident::new(&format!("__{}_register", test_name), Span::call_site());
    let test_register_ident = Ident::new(&format!("__{}_ctor", test_name), Span::call_site());

    let is_xfail = xfail || is_xfail(&attrs);

    let (fixtures_build, call_args) = gen_fixture_call(&sig)?;

    let param_fixture_def = gen_param_fixture(&params);

    let test_idx = TEST_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    Ok(quote! {

            mod #ident {
                use super::*;
                #param_fixture_def
                pub(super) #sig #block
            }

            pub fn #test_generator_ident(ctx: &mut ::rustest::TestContext)
                -> ::std::result::Result<Vec<::rustest::Test>, ::rustest::FixtureCreationError> {
                use ::rustest::IntoError;

                // This is run our test with a set of input and convert it into err as needed.
                let runner = |#(#call_args),*| {#ident::#ident(#(#call_args),*).into_error()};


                // We have to call build a Test per combination of fixtures.
                // Lets build a fixture_matrix.
                let fixtures_matrix = ::rustest::FixtureMatrix::new()#(.feed(#fixtures_build))*;

                // Lets build a set of test_runners. They are taking no input and call the captured
                // fixture combination as needed.
                let test_name = if fixtures_matrix.is_multiple() {
                    |name| format!("{}[{}]", #test_name_str, name)
                } else {
                    |name| #test_name_str.to_owned()
                };

                // Lets loop on all those runners and build a actual Test for each of them.
                let tests = fixtures_matrix.call(
                    move |name, #(#call_args),* | ::rustest::Test::new(
                        test_name(name), #is_xfail, move || runner(#(#call_args),*)
                    )
                )
                    .collect::<Vec<_>>();
                Ok(tests)
            }

            ::rustest::ctor! {
                #[ctor]
                fn #test_register_ident() {
                    // SAFETY: ctor are run outside of main, one after the others, so it is safe
                    // to modify it.
                    unsafe {
                        crate::TEST_GENERATORS[#test_idx] = Some(#test_generator_ident);
                    };
                }
            }

    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ItemFn, parse_quote, parse2};

    #[test]
    fn test_parse_test_attr_no_attr() {
        let attr: TestAttr = parse_quote! {};

        assert_eq!(
            attr,
            TestAttr {
                xfail: false,
                params: None
            }
        );
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
    fn test_parse_test_xfail() {
        let attr: TestAttr = parse_quote! {
            xfail
        };

        assert_eq!(
            attr,
            TestAttr {
                xfail: true,
                params: None
            }
        );
    }

    #[test]
    fn test_parse_test_params() {
        let attr: TestAttr = parse_quote! {
            params:(u32, u8)=[(10,5), (42, 58)]
        };

        assert_eq!(
            attr,
            TestAttr {
                xfail: false,
                params: Some((
                    parse2::<syn::Type>(quote! { (u32,u8) }).unwrap(),
                    parse2::<syn::Expr>(quote! { [(10,5),(42,58)] }).unwrap()
                ))
            }
        );
    }

    #[test]
    fn test_parse_test_xfail_params() {
        let attr: TestAttr = parse_quote! {
            xfail,
            params:(u32, u8)=[(10,5), (42, 58)]
        };

        assert_eq!(
            attr,
            TestAttr {
                xfail: true,
                params: Some((
                    parse2::<syn::Type>(quote! { (u32,u8) }).unwrap(),
                    parse2::<syn::Expr>(quote! { [(10,5),(42,58)] }).unwrap()
                ))
            }
        );
    }

    #[test]
    fn test_parse_test_params_xfail() {
        let attr: TestAttr = parse_quote! {
            params:(u32, u8)=[(10,5), (42, 58)],
            xfail
        };

        assert_eq!(
            attr,
            TestAttr {
                xfail: true,
                params: Some((
                    parse2::<syn::Type>(quote! { (u32,u8) }).unwrap(),
                    parse2::<syn::Expr>(quote! { [(10,5),(42,58)] }).unwrap()
                ))
            }
        );
    }

    #[test]
    fn test_isxfail_empty() {
        let attr: Vec<Attribute> = vec![];

        assert_eq!(is_xfail(&attr), false);
    }

    #[test]
    fn test_isxfail_xfail() {
        let attr: Vec<Attribute> = parse_quote! {#[xfail]};

        assert_eq!(is_xfail(&attr), true);
    }

    #[test]
    fn test_isxfail_xfail_other() {
        let attr: Vec<Attribute> = parse_quote! {
            #[xfail]
            #[other]
        };

        assert_eq!(is_xfail(&attr), true);
    }

    #[test]
    fn test_isxfail_other() {
        let attr: Vec<Attribute> = parse_quote! {
            #[other]
        };

        assert_eq!(is_xfail(&attr), false);
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
