use core::convert::From;
use std::sync::Mutex;

use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, FnArg, ItemFn, LitStr, PathArguments, ReturnType,
    parse_macro_input, spanned::Spanned,
};

static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static FIXTURE_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn build_fixture_create(pat: &syn::Pat, ty: &syn::Type) -> proc_macro2::TokenStream {
    quote! {
        let #pat = ctx.get_fixture()?;
    }
}

/// This macro automatically adds tests function marked with #[test] to the test collection.
#[proc_macro_attribute]
pub fn test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ItemFn { sig, block, .. } = parse_macro_input!(input as ItemFn);

    let ident = &sig.ident;
    let test_name = ident.to_string();
    let test_name_str = LitStr::new(&test_name, Span::call_site());
    let ctor_name = format!("__{}_add_test", test_name);
    let ctor_ident = Ident::new(&ctor_name, Span::call_site());

    let mut fixtures = vec![];
    let mut test_args = vec![];

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = &fnarg.pat;
            fixtures.push(build_fixture_create(pat, &fnarg.ty));
            test_args.push(quote! {#pat});
        }
    });

    TEST_COLLECTORS.lock().unwrap().push(ctor_ident.to_string());

    (quote! {
        #sig #block

        fn #ctor_ident(ctx: &mut ::rustest::Context) -> ::std::result::Result<::rustest::libtest_mimic::Trial, ::rustest::FixtureCreationError> {
            use ::rustest::CollectError;
            #(#fixtures)*
            Ok(::rustest::libtest_mimic::Trial::test(
                #test_name_str,
                    move || {
                        #ident(#(#test_args),*).into()
                }
            ))
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
    let ItemFn { sig, block, .. } = input;

    let fixture_name = args.name.as_ref().unwrap_or(&sig.ident);
    let builder_output = &sig.output;
    let (fallible, fixture_type) = get_fixture_type(&sig)?;

    let mut fixtures = vec![];
    let mut test_args = vec![];
    let sig_inputs = &sig.inputs;

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = &fnarg.pat;
            fixtures.push(build_fixture_create(pat, &fnarg.ty));
            test_args.push(quote! {#pat});
        }
    });

    let (fixture_inner, fixture_impl) = if args.global {
        FIXTURE_COLLECTORS
            .lock()
            .unwrap()
            .push(fixture_name.to_string());

        let fixture_inner = quote! { std::sync::Arc<#fixture_type> };
        let fixture_impl = quote! {};
        (fixture_inner, fixture_impl)
    } else {
        let fixture_inner = quote! { #fixture_type };
        let fixture_impl = quote! {
            fn into_inner(self) -> #fixture_type {
                self.0
            }
        };
        (fixture_inner, fixture_impl)
    };

    let convert_result = if args.fallible.unwrap_or(fallible) {
        quote! {
            result.map(|v| v.into()).map_err(|e| ::rustest::FixtureCreationError::new(stringify!(#fixture_name), e))
        }
    } else {
        quote! {
            Ok(result.into())
        }
    };

    Ok((quote! {
        #[derive(Clone)]
        pub struct #fixture_name(#fixture_inner);

        impl #fixture_name {
            #fixture_impl
        }

        impl From<#fixture_type> for #fixture_name {
            fn from(v: #fixture_type) -> Self {
                Self(v.into())
            }
        }

        impl ::rustest::Fixture for #fixture_name {
            fn setup(ctx: &mut ::rustest::Context) -> ::std::result::Result<Self, ::rustest::FixtureCreationError> {
                use ::rustest::ToResult;
                let inner_build = |#sig_inputs| #builder_output {
                    #block
                };

                #(#fixtures)*
                let result = inner_build(#(#test_args),*);
                #convert_result
            }
        }

        impl std::ops::Deref for #fixture_name {
            type Target = #fixture_type;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    })
    .into())
}

#[proc_macro]
pub fn main(_item: TokenStream) -> TokenStream {
    let global_fixture_types: Vec<_> =
        std::mem::take::<Vec<String>>(FIXTURE_COLLECTORS.lock().unwrap().as_mut())
            .into_iter()
            .map(|s| Ident::new(&s, Span::call_site()))
            .collect();
    let test_ctors: Vec<_> =
        std::mem::take::<Vec<String>>(TEST_COLLECTORS.lock().unwrap().as_mut())
            .into_iter()
            .map(|s| Ident::new(&s, Span::call_site()))
            .collect();

    (quote! {
        fn main() -> std::process::ExitCode {
            use ::rustest::libtest_mimic::{Arguments, Trial, run};
            let args = Arguments::from_args();

            let mut context = ::rustest::Context::new();
            #(context.register_fixture(std::any::TypeId::of::<#global_fixture_types>());)*

            let mut tests = vec![];
            #(tests.push(
                match #test_ctors(&mut context) {
                    Ok(test) => test,
                    Err(e) => {
                        eprintln!("Failed to create fixture {}: {}", e.fixture_name, e.error);
                        return std::process::ExitCode::FAILURE;
                    }
                }

            );)*
            let conclusion = run(&args, tests);
            conclusion.exit_code()
        }
    })
    .into()
}
