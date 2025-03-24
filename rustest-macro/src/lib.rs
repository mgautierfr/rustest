use core::{convert::From, todo, unreachable};
use std::sync::Mutex;

use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, Attribute, FnArg, GenericParam, ItemFn, LitStr, PathArguments,
    ReturnType, TypeParam, parse_macro_input, spanned::Spanned,
};

static TEST_COLLECTORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn build_fixture_create(pat: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        let #pat = ::rustest::get_fixture(ctx)?;
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

        fn #ctor_ident(global_reg: &mut ::rustest::FixtureRegistry) -> ::std::result::Result<::rustest::Test, ::rustest::FixtureCreationError> {
            use ::rustest::IntoError;
            let mut test_registry = ::rustest::FixtureRegistry::new();
            let mut ctx = ::rustest::TestContext::new(global_reg, &mut test_registry);
            let ctx = &mut ctx;
            #(#fixtures)*
            let runner = || {#ident(#(#test_args),*).into_error()};
            Ok(::rustest::Test::new(#test_name_str, #is_xfail, runner))
        }
    })
    .into()
}

#[derive(FromMeta)]
struct FixtureAttr {
    #[darling(default)]
    scope: Option<Ident>,

    #[darling(default)]
    fallible: Option<bool>,

    #[darling(default)]
    name: Option<Ident>,

    #[darling(default)]
    teardown: Option<syn::Expr>,
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
    let fixture_generics = &sig.generics;
    let (impl_generics, ty_generics, where_clause) = fixture_generics.split_for_impl();
    let where_preticate = where_clause.as_ref().map(|wc| &wc.predicates);
    let (fallible, fixture_type) = get_fixture_type(&sig)?;
    let fallible = args.fallible.unwrap_or(fallible);
    let (shared, scope) = args
        .scope
        .map(|s| match s.to_string().as_str() {
            "unique" => (false, quote! {::rustest::FixtureScope::Unique}),
            "test" => (true, quote! {::rustest::FixtureScope::Test}),
            "global" => (true, quote! {::rustest::FixtureScope::Global}),
            _ => todo!(),
        })
        .unwrap_or((false, quote! { ::rustest::FixtureScope::Unique}));

    let mut sub_fixtures = vec![];
    let mut sub_fixtures_args = vec![];

    sig.inputs.iter().for_each(|fnarg| {
        if let FnArg::Typed(fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = fnarg.pat.as_ref() {
                &patident.ident
            } else {
                unreachable!()
            };
            sub_fixtures.push(build_fixture_create(pat));
            sub_fixtures_args.push(quote! {#pat});
        }
    });

    let convert_result = if fallible {
        quote! {
            result.map(|v| v.into()).map_err(|e| ::rustest::FixtureCreationError::new(stringify!(#fixture_name), e))
        }
    } else {
        quote! {
            Ok(result.into())
        }
    };

    let inner_wrapper = if shared {
        quote! { ::rustest::SharedFixtureValue }
    } else {
        quote! { ::rustest::UniqueFixtureValue }
    };
    let sig_inputs = &sig.inputs;
    let builder_output = &sig.output;

    let teardown = args
        .teardown
        .map(|expr| quote! { Some(Box::new(#expr)) })
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
            inner: #inner_wrapper<#fixture_type>,
            #(#phantom_markers),*
        }

        impl #impl_generics #fixture_name #ty_generics #where_clause {
            fn new(inner: #inner_wrapper<#fixture_type>) -> Self {
                Self{
                    inner,
                    #(#phantom_builders),*
                }
            }
        }

        impl #impl_generics Clone for #fixture_name #ty_generics where for<'a> #inner_wrapper<#fixture_type>: Clone,
          #where_preticate

        {
            fn clone(&self) -> Self {
                Self::new(self.inner.clone())
            }
        }

        impl #impl_generics ::rustest::Fixture for #fixture_name #ty_generics  #where_clause {
            type InnerType = #inner_wrapper<#fixture_type>;
            type Type = #fixture_type;
            fn setup(ctx: &mut ::rustest::TestContext) -> ::std::result::Result<Self, ::rustest::FixtureCreationError> {
                let builder = |ctx: &mut ::rustest::TestContext| {
                    let user_provided_setup = |#sig_inputs| #builder_output {
                        #block
                    };

                    #(#sub_fixtures)*
                    let result = user_provided_setup(#(#sub_fixtures_args),*);
                    #convert_result
                };

                Ok(Self::new(Self::InnerType::build::<Self, _>(ctx, builder, #teardown)?))
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
          for<'a> #inner_wrapper<#fixture_type> : ::std::ops::DerefMut,
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
        const TEST_CTORS: &[fn (&mut ::rustest::FixtureRegistry) -> ::std::result::Result<::rustest::Test, ::rustest::FixtureCreationError>] = &[
            #(#test_ctors),*
        ];

        fn main() -> std::process::ExitCode {
            use ::rustest::libtest_mimic::{Arguments, Trial, run};
            let args = Arguments::from_args();

            let mut global_registry = ::rustest::FixtureRegistry::new();

            let tests: ::std::result::Result<_, ::rustest::FixtureCreationError> = TEST_CTORS
                .iter()
                .map(|test_ctor| {
                    Ok(test_ctor(&mut global_registry)?.into())
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
