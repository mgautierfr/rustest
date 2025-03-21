use std::sync::Mutex;

use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{FnArg, ItemFn, LitStr, ReturnType, parse_macro_input};

static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static FIXTURE_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

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
            fixtures.push(quote! {let #pat = ctx.get_fixture();});
            test_args.push(quote! {#pat});
        }
    });

    TEST_COLLECTORS.lock().unwrap().push(ctor_ident.to_string());

    (quote! {
        #sig #block

        fn #ctor_ident(ctx: &mut ::rustest::Context) -> ::rustest::libtest_mimic::Trial {
            use ::rustest::CollectError;
            #(#fixtures)*
            ::rustest::libtest_mimic::Trial::test(
                    #test_name_str,
                        move || {
                            #ident(#(#test_args),*).into()
                    }
                )
        }
    })
    .into()
}

#[derive(FromMeta)]
struct FixtureAttr {
    #[darling(default)]
    global: bool,

    name: Option<Ident>,
}

/// This macro automatically adds tests marked with #[test] to the test collection.
/// Tests then can be run with libtest_mimic_collect::TestCollection::run().
#[proc_macro_attribute]
pub fn fixture(args: TokenStream, input: TokenStream) -> TokenStream {
    let ItemFn { sig, block, .. } = parse_macro_input!(input as ItemFn);

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

    let fixture_name = args.name.as_ref().unwrap_or(&sig.ident);
    let fixture_type = if let ReturnType::Type(_, t) = &sig.output {
        t.clone()
    } else {
        todo!()
    };

    let mut fixtures = vec![];
    let mut test_args = vec![];
    let sig_inputs = &sig.inputs;

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = &fnarg.pat;
            fixtures.push(quote! {let #pat = ctx.get_fixture();});
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
    (quote! {
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
            fn setup(ctx: &mut ::rustest::Context) -> Self {
                let inner_build = |#sig_inputs| {
                    #block
                };

                #(#fixtures)*
                inner_build(#(#test_args),*).into()
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
            #(tests.push(#test_ctors(&mut context));)*
            let conclusion = run(&args, tests);
            println!("End of run");
            conclusion.exit_code()
        }
    })
    .into()
}
