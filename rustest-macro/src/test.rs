use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::sync::atomic::{AtomicUsize, Ordering};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, ItemFn, LitStr, Meta, MetaNameValue, parse_quote};

use crate::utils::{FixtureInfo, gen_fixture_call, gen_param_fixture, to_call_args};

pub(crate) static TEST_COUNT: AtomicUsize = AtomicUsize::new(0);

fn is_xfail(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("xfail"))
}

fn is_ignored(attrs: &[Attribute]) -> Result<Option<syn::Expr>, TokenStream> {
    attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("ignore") {
                match &attr.meta {
                    Meta::Path(_) => Some(Ok(parse_quote! { || true })),
                    Meta::NameValue(MetaNameValue { value, .. }) => Some(Ok(value.clone())),
                    _ => Some(Err(syn::Error::new_spanned(
                        attr,
                        "Invalid format for ignore attr.",
                    )
                    .to_compile_error())),
                }
            } else {
                None
            }
        })
        .transpose()
}

#[derive(Debug, PartialEq)]
pub(crate) struct TestAttr {
    xfail: bool,
    ignore: Option<syn::Expr>,
    params: Option<(syn::Visibility, syn::Type, syn::Expr)>,
}

impl Parse for TestAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut xfail = false;
        let mut ignore = None;
        let mut params = None;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "xfail" => {
                    xfail = true;
                }
                "ignore" => {
                    if input.parse::<syn::Token![=]>().is_ok() {
                        let expr = input.parse()?;
                        ignore = Some(expr);
                    } else {
                        ignore = Some(parse_quote! { || true });
                    }
                }
                "params" => {
                    let _: syn::Token![:] = input.parse()?;
                    let visibility: syn::Visibility = input.parse()?;
                    let ty = input.parse()?;
                    let _: syn::Token![=] = input.parse()?;
                    let expr = input.parse()?;
                    params = Some((visibility, ty, expr));
                }

                _ => {
                    return Err(input.error("unexpected attribute"));
                }
            }
            if input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }
        Ok(TestAttr {
            xfail,
            ignore,
            params,
        })
    }
}

pub(crate) fn test_impl(args: TestAttr, input: ItemFn) -> Result<TokenStream, TokenStream> {
    let ItemFn {
        mut sig,
        block,
        attrs,
        ..
    } = input;
    let TestAttr {
        xfail,
        ignore,
        params,
    } = args;

    let ident = sig.ident.clone();
    sig.ident = Ident::new("test", Span::call_site());
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let test_generator_ident = Ident::new(&format!("__{}_register", test_name), Span::call_site());
    let test_register_ident = Ident::new(&format!("__{}_ctor", test_name), Span::call_site());

    let is_xfail = xfail || is_xfail(&attrs);
    let ignored_fn = match ignore {
        Some(func) => func,
        None => is_ignored(&attrs)?.unwrap_or(parse_quote! {|| false}),
    };

    let FixtureInfo {
        sub_fixtures_proxies,
        sub_fixtures_inputs,
        ..
    } = gen_fixture_call(&sig, None)?;
    let sub_fixtures_call_args = to_call_args(&sub_fixtures_inputs);

    let param_fixture_def = gen_param_fixture(&params, None);

    let test_idx = TEST_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(quote! {

            mod #ident {
                use super::*;
                #param_fixture_def
                pub(super) #sig #block

                pub fn #test_generator_ident(ctx: &mut ::rustest::TestContext) -> Vec<::rustest::Test> {
                    use ::rustest::{FixtureProxy, IntoError, ProxyCall};

                    // We have to call build a Test per combination of fixtures.
                    // Lets build a proxy_matrix.
                    let proxies_matrix = ::rustest::ProxyMatrix::new()#(.feed(#sub_fixtures_proxies::setup(ctx)))*;
                    let combinations = proxies_matrix.flatten();

                    // Append a fixture identifier to test name if we have multiple fixtures instances
                    let test_name = if combinations.len() > 1 {
                        |name: Option<_>| format!("{}[{}]", #test_name_str, name.unwrap())
                    } else {
                        |name| #test_name_str.to_owned()
                    };

                    let is_ignored = #ignored_fn;

                    // Lets loop on all the fixture combinations and build a Test for each of them.
                    let tests = combinations.into_iter().map(|c| {
                        use ::rustest::TestName;
                        let name = c.name();
                        let runner_gen = Box::new(move || {
                            c.call(move |#sub_fixtures_call_args| -> ::rustest::FixtureCreationResult<Box<::rustest::TestRunner>> {
                                Ok(
                                    Box::new(|| #ident::test(#(#sub_fixtures_inputs),*).into_error()),
                                )
                            })
                        });
                        ::rustest::Test::new(test_name(name), #is_xfail, is_ignored(), runner_gen)
                    })
                    .collect::<Vec<_>>();
                    tests
                }
            }

            ::rustest::ctor! {
                #[ctor]
                fn #test_register_ident() {
                    // SAFETY: ctor are run outside of main, one after the others, so it is safe
                    // to modify it.
                    unsafe {
                        crate::TEST_GENERATORS[#test_idx] = Some(#ident::#test_generator_ident);
                    };
                }
            }

    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{ItemFn, Visibility, parse_quote, parse2};

    #[test]
    fn test_parse_test_attr_no_attr() {
        let attr: TestAttr = parse_quote! {};

        assert_eq!(
            attr,
            TestAttr {
                xfail: false,
                ignore: None,
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
                ignore: None,
                params: None
            }
        );
    }

    #[test]
    fn test_parse_test_ignore() {
        let attr: TestAttr = parse_quote! {
            ignore
        };

        assert_eq!(
            attr,
            TestAttr {
                xfail: false,
                ignore: Some(parse_quote! {|| true}),
                params: None
            }
        );
    }

    #[test]
    fn test_parse_test_ignore_fn() {
        let attr: TestAttr = parse_quote! {
            ignore = || true
        };

        assert_eq!(
            attr,
            TestAttr {
                xfail: false,
                ignore: Some(parse_quote! {|| true}),
                params: None
            }
        );
    }

    #[test]
    fn test_parse_test_ignore_fn_wrong_syntax() {
        let parse_result = parse2::<TestAttr>(quote! {
            ignore(|| true)
        });

        assert!(parse_result.is_err());
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
                ignore: None,
                params: Some((
                    Visibility::Inherited,
                    parse_quote! { (u32,u8) },
                    parse_quote! { [(10,5),(42,58)] }
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
                ignore: None,
                params: Some((
                    Visibility::Inherited,
                    parse_quote! { (u32,u8) },
                    parse_quote! { [(10,5),(42,58)] }
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
                ignore: None,
                params: Some((
                    Visibility::Inherited,
                    parse_quote! { (u32,u8) },
                    parse_quote! { [(10,5),(42,58)] }
                ))
            }
        );
    }

    #[test]
    fn test_isxfail_empty() {
        let attr: Vec<Attribute> = vec![];

        assert!(!is_xfail(&attr));
    }

    #[test]
    fn test_isxfail_xfail() {
        let attr: Vec<Attribute> = parse_quote! {#[xfail]};

        assert!(is_xfail(&attr));
    }

    #[test]
    fn test_isxfail_xfail_other() {
        let attr: Vec<Attribute> = parse_quote! {
            #[xfail]
            #[other]
        };

        assert!(is_xfail(&attr));
    }

    #[test]
    fn test_isxfail_other() {
        let attr: Vec<Attribute> = parse_quote! {
            #[other]
        };

        assert!(!is_xfail(&attr));
    }

    #[test]
    fn test_isignore_empty() {
        let attr: Vec<Attribute> = vec![];

        assert!(is_ignored(&attr).unwrap().is_none());
    }

    #[test]
    fn test_isignored_ignore() {
        let attr: Vec<Attribute> = parse_quote! {#[ignore]};

        assert!(is_ignored(&attr).unwrap().is_some());
        assert_eq!(
            is_ignored(&attr).unwrap().unwrap(),
            parse_quote! { || true }
        );
    }

    #[test]
    fn test_isignored_ignore_fn() {
        let attr: Vec<Attribute> = parse_quote! {#[ignore=ignore_func]};

        assert!(is_ignored(&attr).unwrap().is_some());
        assert_eq!(
            is_ignored(&attr).unwrap().unwrap(),
            parse_quote! { ignore_func }
        );
    }

    #[test]
    fn test_isignore_ignore_other() {
        let attr: Vec<Attribute> = parse_quote! {
            #[ignore]
            #[other]
        };

        assert!(is_ignored(&attr).unwrap().is_some());
    }

    #[test]
    fn test_isignore_other() {
        let attr: Vec<Attribute> = parse_quote! {
            #[other]
        };

        assert!(is_ignored(&attr).unwrap().is_none());
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
            ignore: None,
            params: None,
        };

        let result = test_impl(args, input);
        assert!(result.is_ok());
    }
}
