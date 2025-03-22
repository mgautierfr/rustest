use core::{convert::From, unreachable};
use std::sync::Mutex;

use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, Attribute, FnArg, ItemFn, LitStr, PathArguments, ReturnType,
    Visibility, parse_macro_input, spanned::Spanned,
};

static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn build_fixture_create(pat: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        let #pat = ctx.get_fixture()?;
    }
}

fn is_xfail(attrs: &Vec<Attribute>) -> bool {
    for attr in attrs.iter() {
        if attr.path().is_ident("xfail") {
            return true;
        }
    }
    false
}

#[derive(FromMeta)]
struct TestAttr {
    #[darling(default)]
    xfail: bool,
}

/// This macro automatically adds tests function marked with #[test] to the test collection.
#[proc_macro_attribute]
pub fn test(args: TokenStream, input: TokenStream) -> TokenStream {
    let ItemFn {
        sig, block, attrs, ..
    } = parse_macro_input!(input as ItemFn);
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(darling::Error::from(e).write_errors());
        }
    };
    let args = match TestAttr::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let ident = &sig.ident;
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let ctor_name = format!("__{}_add_test", test_name);
    let ctor_ident = Ident::new(&ctor_name, Span::call_site());

    let is_xfail = args.xfail || is_xfail(&attrs);

    let mut fixtures = vec![];
    let mut test_args = vec![];

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = fnarg.pat.as_ref() {
                &patident.ident
            } else {
                unreachable!()
            };
            fixtures.push(build_fixture_create(pat));
            test_args.push(quote! {#pat});
        }
    });

    TEST_COLLECTORS.lock().unwrap().push(ctor_ident.to_string());

    (quote! {
        #sig #block

        fn #ctor_ident(ctx: &mut ::rustest::Context) -> ::std::result::Result<::rustest::libtest_mimic::Trial, ::rustest::FixtureCreationError> {
            use ::rustest::IntoError;
            #(#fixtures)*
            let trial = ::rustest::libtest_mimic::Trial::test(
                #test_name_str,
                move || {
                    ::rustest::run_test(|| {#ident(#(#test_args),*).into_error()}, #is_xfail)
                }
            );
            if #is_xfail {
                Ok(trial.with_kind("XFAIL"))
            } else {
                Ok(trial)
            }
        }
    })
    .into()
}

#[derive(FromMeta)]
struct FixtureAttr {
    #[darling(default)]
    global: bool,

    #[darling(default)]
    fallible: Option<bool>,

    name: Option<Ident>,
}

fn get_fixture_type(
    signature: &syn::Signature,
) -> Result<(bool, proc_macro2::TokenStream), proc_macro2::TokenStream> {
    if let ReturnType::Type(_, output_type) = &signature.output {
        match output_type.as_ref() {
            syn::Type::Path(type_path) => {
                let last = type_path.path.segments.last().unwrap();
                if last.ident.to_string() == "Result" {
                    match &last.arguments {
                        PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            args,
                            ..
                        }) => {
                            let ty = args.first().unwrap();
                            Ok((true, quote! { #ty }))
                        }
                        _ => Err(quote_spanned! {
                            output_type.span() =>
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
            signature.span() =>
            compile_error!("Cannot detect fixture type.");
        })
    }
}

/// This macro automatically adds tests marked with #[test] to the test collection.
/// Tests then can be run with libtest_mimic_collect::TestCollection::run().
#[proc_macro_attribute]
pub fn fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(darling::Error::from(e).write_errors());
        }
    };
    let args = match FixtureAttr::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };
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
    let (fallible, fixture_type) = get_fixture_type(&sig)?;

    let mut fixtures = vec![];
    let mut test_args = vec![];

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = fnarg.pat.as_ref() {
                &patident.ident
            } else {
                unreachable!()
            };
            fixtures.push(build_fixture_create(pat));
            test_args.push(quote! {#pat});
        }
    });

    if args.global {
        Ok(global_fixture(
            vis,
            fixture_name,
            fixture_type,
            args.fallible.unwrap_or(fallible),
            fixtures,
            test_args,
            &sig,
            &block,
        )
        .into())
    } else {
        Ok(local_fixture(
            vis,
            fixture_name,
            fixture_type,
            args.fallible.unwrap_or(fallible),
            fixtures,
            test_args,
            &sig,
            &block,
        )
        .into())
    }
}

fn global_fixture(
    vis: Visibility,
    fixture_name: &Ident,
    fixture_type: proc_macro2::TokenStream,
    fallible: bool,
    sub_fixtures: Vec<proc_macro2::TokenStream>,
    sub_fixtures_args: Vec<proc_macro2::TokenStream>,
    sig: &syn::Signature,
    block: &syn::Block,
) -> TokenStream {
    let convert_result = if fallible {
        quote! {
            result.map(|v| v.into()).map_err(|e| ::rustest::FixtureCreationError::new(stringify!(#fixture_name), e))
        }
    } else {
        quote! {
            Ok(result.into())
        }
    };
    let sig_inputs = &sig.inputs;
    let builder_output = &sig.output;

    (quote! {
        #[derive(Clone)]
        #vis struct #fixture_name(std::sync::Arc<#fixture_type>);

        impl From<#fixture_type> for #fixture_name {
            fn from(v: #fixture_type) -> Self {
                Self(v.into())
            }
        }

        impl ::rustest::Fixture for #fixture_name {
            fn setup(ctx: &mut ::rustest::Context) -> ::std::result::Result<Self, ::rustest::FixtureCreationError> {
                if let Some(f) = ctx.fixtures.get(&std::any::TypeId::of::<#fixture_name>()) {
                    let fixture = f.downcast_ref::<#fixture_name>().unwrap();
                    return Ok(fixture.clone());
                }

                let inner_build = |#sig_inputs| #builder_output {
                    #block
                };

                #(#sub_fixtures)*
                let result = inner_build(#(#sub_fixtures_args),*);
                let value: #fixture_name = #convert_result?;

                ctx.fixtures
                    .insert(std::any::TypeId::of::<#fixture_name>(), Box::new(value.clone()));
                Ok(value)

            }
        }

        impl std::ops::Deref for #fixture_name {
            type Target = #fixture_type;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    })
    .into()
}

fn local_fixture(
    vis: Visibility,
    fixture_name: &Ident,
    fixture_type: proc_macro2::TokenStream,
    fallible: bool,
    sub_fixtures: Vec<proc_macro2::TokenStream>,
    sub_fixtures_args: Vec<proc_macro2::TokenStream>,
    sig: &syn::Signature,
    block: &syn::Block,
) -> TokenStream {
    let sig_inputs = &sig.inputs;
    let builder_output = &sig.output;

    let convert_result = if fallible {
        quote! {
            result.map(|v| v.into()).map_err(|e| ::rustest::FixtureCreationError::new(stringify!(#fixture_name), e))
        }
    } else {
        quote! {
            Ok(result.into())
        }
    };

    (quote! {
        #vis struct #fixture_name(#fixture_type);

        impl #fixture_name where for<'a> #fixture_type: Copy {
            pub fn into_inner(self) -> #fixture_type {
                self.0
            }
        }

        impl From<#fixture_type> for #fixture_name {
            fn from(v: #fixture_type) -> Self {
                Self(v.into())
            }
        }

        impl ::rustest::Fixture for #fixture_name {
            fn setup(ctx: &mut ::rustest::Context) -> ::std::result::Result<Self, ::rustest::FixtureCreationError> {
                let inner_build = |#sig_inputs| #builder_output {
                    #block
                };

                #(#sub_fixtures)*
                let result = inner_build(#(#sub_fixtures_args),*);
                #convert_result
            }
        }

        impl std::ops::Deref for #fixture_name {
            type Target = #fixture_type;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for #fixture_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    })
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
        const TEST_CTORS: &[fn (&mut ::rustest::Context) -> ::std::result::Result<::rustest::libtest_mimic::Trial, ::rustest::FixtureCreationError>] = &[
            #(#test_ctors),*
        ];

        fn main() -> std::process::ExitCode {
            use ::rustest::libtest_mimic::{Arguments, Trial, run};
            let args = Arguments::from_args();

            let mut context = ::rustest::Context::new();

            let tests = TEST_CTORS
                .iter()
                .map(|test_ctor| {
                    test_ctor(&mut context)
                })
                .collect();

            let tests = match tests {
                Ok(test) => test,
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
