use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::{
    AngleBracketedGenericArguments, GenericParam, ItemFn, PathArguments, ReturnType, TypeParam,
    spanned::Spanned,
};

use crate::utils::{gen_fixture_call, gen_param_fixture};

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
            _ => Err(syn::Error::new_spanned(
                &ident,
                format!(
                    "expected one of 'unique', 'global', or 'test'. Got {}.",
                    ident
                ),
            )),
        }
    }
}

impl From<FixtureScope> for TokenStream {
    fn from(value: FixtureScope) -> Self {
        match value {
            FixtureScope::Unique => quote! {::rustest::FixtureScope::Unique},
            FixtureScope::Test => quote! {::rustest::FixtureScope::Test},
            FixtureScope::Global => quote! {::rustest::FixtureScope::Global},
        }
    }
}

pub(crate) struct FixtureAttr {
    scope: Option<FixtureScope>,
    fallible: Option<bool>,
    name: Option<Ident>,
    teardown: Option<syn::Expr>,
    params: Option<(syn::Type, syn::Expr)>,
}

impl Parse for FixtureAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut scope = None;
        let mut fallible = None;
        let mut name = None;
        let mut teardown = None;
        let mut params = None;

        while !input.is_empty() {
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
                    let _: syn::Token![:] = input.parse()?;
                    let ty = input.parse()?;
                    let _: syn::Token![=] = input.parse()?;
                    let expr = input.parse()?;
                    params = Some((ty, expr));
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &ident,
                        format!("Unknown token {}.", ident),
                    ));
                }
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

fn get_fixture_type(signature: &syn::Signature) -> Result<(bool, TokenStream), TokenStream> {
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

pub(crate) fn fixture_impl(args: FixtureAttr, input: ItemFn) -> Result<TokenStream, TokenStream> {
    let ItemFn {
        sig, block, vis, ..
    } = input;

    let fixture_name = args.name.as_ref().unwrap_or(&sig.ident);
    let fixture_generics = &sig.generics;
    let (impl_generics, ty_generics, where_clause) = fixture_generics.split_for_impl();
    let where_predicate = where_clause.as_ref().map(|wc| &wc.predicates);
    let (fallible, fixture_type) = get_fixture_type(&sig)?;
    let fallible = args.fallible.unwrap_or(fallible);
    let scope = args
        .scope
        .or(Some(FixtureScope::Unique))
        .map(TokenStream::from);

    let (sub_fixtures_build, call_args) = gen_fixture_call(&sig)?;
    let param_fixture_def = gen_param_fixture(&args.params);

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
            _ => {
                return Err(
                    syn::Error::new_spanned(param, "expected a type parameter").to_compile_error()
                );
            }
        };
        phantom_markers.push(quote! {
            #phantom_ident: std::marker::PhantomData<#phantom_type>
        });
        phantom_builders.push(quote! { #phantom_ident: Default::default() });
    }

    Ok(quote! {
        #[derive(Clone)]
        #vis struct #fixture_name #fixture_generics #where_clause {
            inner: #inner_type,
            #(#phantom_markers),*
        }

        impl #impl_generics #fixture_name #ty_generics #where_clause {
            fn new(inner: #inner_type) -> Self {
                Self {
                    inner,
                    #(#phantom_builders),*
                }
            }
        }

        impl #impl_generics ::rustest::FixtureDisplay for #fixture_name #ty_generics
        where
            for<'a> #inner_type: ::rustest::FixtureDisplay,
            #where_predicate
        {
            fn display(&self) -> String {
                format!("{}:{}", stringify!(#fixture_name), self.inner.display())
            }
        }

        impl #impl_generics ::rustest::Fixture for #fixture_name #ty_generics #where_clause {
            type InnerType = #inner_type;
            type Type = #fixture_type;
            fn setup(ctx: &mut ::rustest::TestContext) -> ::std::result::Result<Vec<Self>, ::rustest::FixtureCreationError> {
                #param_fixture_def
                // From InnerType::build, builders must be a function which, when call with a TestContext, returns a Vec of #fixture_type.
                let builders = |ctx: &mut ::rustest::TestContext| {

                    // This is a lambda which call the initial impl of the fixture and transform the (#fixture_type) into a
                    // `Resutl<#fixture_type, >` if this is not already a `Result`.
                    let user_provided_setup_as_result = |#(#call_args),*| {
                        let user_provided_setup = |#sig_inputs| #builder_output #block;
                        let result = user_provided_setup(#(#call_args),*);
                        #convert_result
                    };

                    // We have to call this function for each combination of its fixtures.
                    // Lets build a fixture_matrix.
                    let fixtures_matrix = ::rustest::FixtureMatrix::new()#(.feed(#sub_fixtures_build))*;
                    let combinations = fixtures_matrix.flatten();

                    combinations.into_iter().map(|c|
                        // call do not call the lambda but return a new callable which will call the input builder with
                        // the right fixture combination.
                        c.call(move | _, #(#call_args),* | user_provided_setup_as_result(#(#call_args),*))
                    )
                    .collect::<std::result::Result<Vec<_>, _>>()

                };
                let inners = Self::InnerType::build::<Self, _>(ctx, builders, #teardown)?;
                Ok(inners.into_iter().map(|i| Self::new(i)).collect())
            }

            fn build(&self) -> Self {
                self.clone()
            }

            fn scope() -> ::rustest::FixtureScope { #scope }
        }

        impl #impl_generics ::std::ops::Deref for #fixture_name #ty_generics #where_clause {
            type Target = <Self as ::rustest::Fixture>::Type;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }
    })
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
            params:my_type = my_params_expr
        };

        let fixture_attr = parse2::<FixtureAttr>(input).unwrap();

        assert_eq!(fixture_attr.scope, Some(FixtureScope::Global));
        assert!(fixture_attr.fallible.unwrap());
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
        assert!(!fixture_attr.fallible.unwrap());
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
