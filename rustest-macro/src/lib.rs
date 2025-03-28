use core::{convert::From, todo, unreachable};
use std::sync::Mutex;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, Attribute, FnArg, GenericParam, ItemFn, LitStr, PathArguments,
    ReturnType, TypeParam, parse_macro_input, spanned::Spanned,
};

use syn::parse::{Parse, ParseStream};

static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn is_xfail(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("xfail"))
}

impl Parse for TestAttr {
    fn parse(args: ParseStream) -> syn::Result<Self> {
        let mut xfail = false;
        if args.peek(syn::Ident) {
            let ident = args.parse::<syn::Ident>()?;
            if ident == "xfail" {
                xfail = true;
            }
        }
        Ok(TestAttr { xfail })
    }
}
struct TestAttr {
    xfail: bool,
}

/// This macro automatically adds tests function marked with #[test] to the test collection.
#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    let ItemFn {
        sig, block, attrs, ..
    } = parse_macro_input!(input as ItemFn);
    let TestAttr { xfail } = parse_macro_input!(args as TestAttr);

    let ident = &sig.ident;
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let ctor_name = format!("__{}_add_test", test_name);
    let ctor_ident = Ident::new(&ctor_name, Span::call_site());

    let is_xfail = xfail || is_xfail(&attrs);

    let mut fixtures = vec![];
    let mut test_args = vec![];

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = fnarg.pat.as_ref() {
                &patident.ident
            } else {
                unreachable!()
            };
            fixtures.push(quote! {
                ::rustest::get_fixture(ctx)?
            });
            test_args.push(quote! {#pat});
        }
    });

    TEST_COLLECTORS.lock().unwrap().push(ctor_ident.to_string());

    (quote! {
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
    .into()
}

#[derive(Debug, PartialEq)]
enum FixtureScope {
    Unique,
    Test,
    Global,
}

impl Parse for FixtureScope {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "unique" => Ok(FixtureScope::Unique),
            "global" => Ok(FixtureScope::Global),
            "test" => Ok(FixtureScope::Test),
            _ => {
                // Return an error if the identifier does not match any variant
                Err(syn::Error::new_spanned(
                    &ident,
                    format!(
                        "expected one of 'unique', 'global', or 'test'. Got {}.",
                        ident
                    ),
                ))
            }
        }
    }
}

impl From<FixtureScope> for TokenStream2 {
    fn from(value: FixtureScope) -> Self {
        match value {
            FixtureScope::Unique => quote! {::rustest::FixtureScope::Unique},
            FixtureScope::Test => quote! {::rustest::FixtureScope::Test},
            FixtureScope::Global => quote! {::rustest::FixtureScope::Global},
        }
    }
}

struct FixtureAttr {
    scope: Option<FixtureScope>,
    fallible: Option<bool>,
    name: Option<Ident>,
    teardown: Option<syn::Expr>,
    params: Option<syn::Expr>,
}

impl Parse for FixtureAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut scope = None;
        let mut fallible = None;
        let mut name = None;
        let mut teardown = None;
        let mut params = None;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident) {
                let ident: Ident = input.parse()?;
                match ident.to_string().as_str() {
                    "scope" => {
                        let _: syn::Token![=] = input.parse()?;
                        scope = Some(input.parse()?);
                    }
                    "fallible" => {
                        if input.peek(syn::Token![=]) {
                            let _: syn::Token![=] = input.parse()?;
                            let v: syn::LitBool = input.parse()?;
                            fallible = Some(v.value);
                        } else {
                            fallible = Some(true);
                        }
                    }
                    "name" => {
                        let _: syn::Token![=] = input.parse()?;
                        name = Some(input.parse()?);
                    }
                    "teardown" => {
                        let _: syn::Token![=] = input.parse()?;
                        teardown = Some(input.parse()?);
                    }
                    "params" => {
                        let _: syn::Token![=] = input.parse()?;
                        params = Some(input.parse()?);
                    }
                    _ => {
                        return Err(lookahead.error());
                    }
                }
            } else {
                break;
            }

            if input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        Ok(FixtureAttr {
            scope,
            fallible,
            name,
            teardown,
            params,
        })
    }
}

fn get_fixture_type(
    signature: &syn::Signature,
) -> Result<(bool, proc_macro2::TokenStream), proc_macro2::TokenStream> {
    if let ReturnType::Type(_, output_type) = &signature.output {
        match output_type.as_ref() {
            syn::Type::Path(type_path) => {
                let last = type_path.path.segments.last().unwrap();
                if last.ident == "Result" {
                    match &last.arguments {
                        PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            args,
                            ..
                        }) => {
                            let ty = args.first().unwrap();
                            Ok((true, quote! { #ty }))
                        }
                        _ => Err(quote_spanned! {
                            output_type.span()=>
                            compile_error!("Cannot detect fixture type.");
                        }),
                    }
                } else {
                    Ok((false, quote! { #output_type }))
                }
            }
            _ => Ok((false, quote! {#output_type})),
        }
    } else {
        Err(quote_spanned! {
            signature.span()=>
            compile_error!("Cannot detect fixture type.");
        })
    }
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

fn fixture_impl(
    args: FixtureAttr,
    input: ItemFn,
) -> Result<proc_macro2::TokenStream, proc_macro2::TokenStream> {
    let ItemFn {
        sig, block, vis, ..
    } = input;

    let fixture_name = args.name.as_ref().unwrap_or(&sig.ident);
    let fixture_generics = &sig.generics;
    let (impl_generics, ty_generics, where_clause) = fixture_generics.split_for_impl();
    let where_preticate = where_clause.as_ref().map(|wc| &wc.predicates);
    let (fallible, fixture_type) = get_fixture_type(&sig)?;
    let fallible = args.fallible.unwrap_or(fallible);
    let scope = args
        .scope
        .or(Some(FixtureScope::Unique))
        .map(TokenStream2::from);

    let mut sub_fixtures = vec![];
    let mut sub_fixtures_args = vec![];

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = fnarg.pat.as_ref() {
                &patident.ident
            } else {
                unreachable!()
            };
            if pat == "param" && args.params.is_some() {
                let params = &args.params;
                sub_fixtures.push(
                    quote! { #params.into_iter().map(|i| ::rustest::FixtureParam(i)).collect::<Vec<_>>() },
                );
            } else {
                sub_fixtures.push(quote! {::rustest::get_fixture(ctx)?});
            }
            sub_fixtures_args.push(quote! {#pat});
        }
    });

    let convert_result = if fallible {
        quote! {
            result.map_err(|e| ::rustest::FixtureCreationError::new(stringify!(#fixture_name), e))
        }
    } else {
        quote! {
            Ok(result)
        }
    };

    let inner_type = quote! { ::rustest::SharedFixtureValue<#fixture_type> };
    let sig_inputs = &sig.inputs;
    let builder_output = &sig.output;

    let teardown = args
        .teardown
        .map(|expr| quote! { Some(std::sync::Arc::new(#expr)) })
        .unwrap_or_else(|| quote! { None });

    let mut phantom_markers = vec![];
    let mut phantom_builders = vec![];
    for (i, param) in sig.generics.params.iter().enumerate() {
        let phantom_name = format!("__phantom_{}", i);
        let phantom_ident = Ident::new(&phantom_name, Span::call_site());
        let phantom_type = match param {
            GenericParam::Type(TypeParam { ident, .. }) => ident,
            _ => todo!(),
        };
        phantom_markers.push(quote! {
          #phantom_ident: std::marker::PhantomData<#phantom_type>
        });
        phantom_builders.push(quote! { #phantom_ident: Default::default() });
    }

    Ok(quote! {
        #vis struct #fixture_name #fixture_generics #where_clause {
            inner: #inner_type,
            #(#phantom_markers),*
        }

        impl #impl_generics #fixture_name #ty_generics #where_clause {
            fn new(inner: #inner_type) -> Self {
                Self{
                    inner,
                    #(#phantom_builders),*
                }
            }
        }

        impl #impl_generics Clone for #fixture_name #ty_generics
        where
            for<'a> #inner_type: Clone,
            #where_preticate
        {
            fn clone(&self) -> Self {
                Self::new(self.inner.clone())
            }
        }

        impl #impl_generics ::rustest::FixtureName for #fixture_name #ty_generics
        where
            for<'a> #inner_type: ::rustest::FixtureName,
            #where_preticate
        {
            fn name(&self) -> String {
                format!("{}:{}", stringify!(#fixture_name), self.inner.name())
            }
        }

        impl #impl_generics ::rustest::Fixture for #fixture_name #ty_generics  #where_clause {
            type InnerType = #inner_type;
            type Type = #fixture_type;
            fn setup(ctx: &mut ::rustest::TestContext) -> ::std::result::Result<Vec<Self>, ::rustest::FixtureCreationError> {
                let builders = |ctx: &mut ::rustest::TestContext| {
                    let user_provided_setup = |#sig_inputs| #builder_output {
                        #block
                    };

                    let user_provided_setup_as_result = move |#sig_inputs| {
                        let result = user_provided_setup(#(#sub_fixtures_args),*);
                        #convert_result
                    };

                    let fixtures_matrix = ::rustest::FixtureMatrix::new();
                    #(let fixtures_matrix = fixtures_matrix.feed(#sub_fixtures);)*
                    let matrix_caller: ::rustest::MatrixCaller<_> = fixtures_matrix.into();

                    matrix_caller.call(user_provided_setup_as_result).into_iter().map(|(_, p)| p()).collect::<std::result::Result<Vec<_>, _>>()
                };
                let inners = Self::InnerType::build::<Self, _>(ctx, builders, #teardown)?;
                Ok(inners.into_iter().map(|i| Self::new(i)).collect())
            }

            fn scope() -> ::rustest::FixtureScope { #scope }
        }

        impl #impl_generics ::std::ops::Deref for #fixture_name #ty_generics  #where_clause{
            type Target = <Self as Fixture>::Type;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl #impl_generics ::std::ops::DerefMut for #fixture_name #ty_generics
          where
          for<'a> #inner_type: ::std::ops::DerefMut,
          #where_preticate
        {
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.inner.deref_mut()
            }
        }
    })
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

#[cfg(test)]
mod tests {
    use super::{FixtureAttr, FixtureScope};
    use quote::quote;
    use syn::parse2;

    #[test]
    fn test_parse_fixture_attr_all_fields() {
        let input = quote! {
            scope = global,
            fallible = true,
            name = my_name,
            teardown = my_teardown_expr,
            params = my_params_expr
        };

        let fixture_attr = parse2::<FixtureAttr>(input).unwrap();

        assert_eq!(fixture_attr.scope, Some(FixtureScope::Global));
        assert_eq!(fixture_attr.fallible.unwrap(), true);
        assert_eq!(fixture_attr.name.unwrap().to_string(), "my_name");
        assert!(fixture_attr.teardown.is_some());
        assert!(fixture_attr.params.is_some());
    }

    #[test]
    fn test_parse_fixture_attr_some_fields() {
        let input = quote! {
            scope = unique,
            fallible = false
        };

        let fixture_attr = parse2::<FixtureAttr>(input).unwrap();

        assert_eq!(fixture_attr.scope, Some(FixtureScope::Unique));
        assert_eq!(fixture_attr.fallible.unwrap(), false);
        assert!(fixture_attr.name.is_none());
        assert!(fixture_attr.teardown.is_none());
        assert!(fixture_attr.params.is_none());
    }

    #[test]
    fn test_parse_fixture_attr_no_fields() {
        let input = quote! {};

        let fixture_attr = parse2::<FixtureAttr>(input).unwrap();

        assert!(fixture_attr.scope.is_none());
        assert!(fixture_attr.fallible.is_none());
        assert!(fixture_attr.name.is_none());
        assert!(fixture_attr.teardown.is_none());
        assert!(fixture_attr.params.is_none());
    }

    #[test]
    fn test_parse_fixture_attr_invalid_field() {
        let input = quote! {
            invalid_field = some_value
        };

        let result = parse2::<FixtureAttr>(input);
        assert!(result.is_err());
    }
    #[test]
    fn test_parse_fixture_attr_invalid_scope() {
        let input = quote! {
            scope = invalid_scope
        };
        let result = parse2::<FixtureAttr>(input);
        assert!(result.is_err());
        // Get the error
        let error = result.err().unwrap();

        // Check that the error message is as expected
        assert_eq!(
            error.to_string(),
            "expected one of 'unique', 'global', or 'test'. Got invalid_scope."
        );
    }
}
