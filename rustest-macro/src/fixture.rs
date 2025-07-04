use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, GenericParam, ItemFn, PathArguments, ReturnType, TypeParam,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

use crate::utils::{FixtureInfo, gen_fixture_call, gen_param_fixture, to_call_args, to_tuple};

#[derive(Debug, PartialEq, Copy, Clone)]
enum FixtureScope {
    Once,
    MatrixUnique,
    Test,
    Global,
}

impl Parse for FixtureScope {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "once" => Ok(FixtureScope::Once),
            "matrix" => Ok(FixtureScope::MatrixUnique),
            "global" => Ok(FixtureScope::Global),
            "test" => Ok(FixtureScope::Test),
            _ => Err(syn::Error::new_spanned(
                &ident,
                format!(
                    "expected one of 'once', 'matrix', 'global', or 'test'. Got {}.",
                    ident
                ),
            )),
        }
    }
}

impl From<FixtureScope> for TokenStream {
    fn from(value: FixtureScope) -> Self {
        match value {
            FixtureScope::Once => quote! {::rustest::FixtureScope::Once},
            FixtureScope::MatrixUnique => quote! {::rustest::FixtureScope::MatrixUnique},
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
    params: Option<(syn::Visibility, syn::Type, syn::Expr)>,
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
                    let visibility: syn::Visibility = input.parse()?;
                    let ty = input.parse()?;
                    let _: syn::Token![=] = input.parse()?;
                    let expr = input.parse()?;
                    params = Some((visibility, ty, expr));
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
    let def_name = Ident::new(&format!("__{}Def", fixture_name), Span::call_site());
    let mod_name = Ident::new(&format!("__{}_mod", fixture_name), Span::call_site());
    let fixture_generics = &sig.generics;
    let (impl_generics, ty_generics, where_clause) = fixture_generics.split_for_impl();
    let (fallible, fixture_type) = get_fixture_type(&sig)?;
    let fallible = args.fallible.unwrap_or(fallible);
    let scope = args.scope.unwrap_or(FixtureScope::Once);

    let scope_token = TokenStream::from(scope);

    let FixtureInfo {
        sub_fixtures_proxies,
        sub_fixtures,
        sub_fixtures_inputs,
    } = gen_fixture_call(&sig, Some(&mod_name))?;
    let sub_proxy_types_tuple = to_tuple(&sub_fixtures_proxies);
    let sub_fixtures_tuple = to_tuple(&sub_fixtures);
    let sub_fixtures_call_args = to_call_args(&sub_fixtures_inputs);
    let param_fixture_def = gen_param_fixture(&args.params, Some(fixture_name));
    let use_param = if args.params.is_some() {
        quote! { use #mod_name::Param; }
    } else {
        quote! {}
    };

    let convert_result = if fallible {
        quote! {
            result.map_err(|e| ::rustest::FixtureCreationError::new(stringify!(#fixture_name), e))
        }
    } else {
        quote! {
            Ok(result)
        }
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

    let def_tokens = quote! {
        #vis struct #def_name #fixture_generics #where_clause {
                #(#phantom_markers),*
            }

        impl #impl_generics ::rustest::FixtureDef for #def_name #ty_generics #where_clause {
            type Fixt = #fixture_name #ty_generics;
            type SubFixtures = #sub_fixtures_tuple;
            type SubProxies =  #sub_proxy_types_tuple;
            const SCOPE: ::rustest::FixtureScope = #scope_token;

            fn build_fixt(
                #sub_fixtures_call_args : ::rustest::CallArgs<Self::SubFixtures>,
            ) -> ::rustest::FixtureCreationResult<<Self::Fixt as ::rustest::Fixture>::Type> {
                use ::rustest::FixtureProxy;
                #use_param

                fn user_provided_setup #fixture_generics (#sig_inputs) #builder_output #where_clause
                #block

                let result = user_provided_setup(#(#sub_fixtures_inputs),*);
                // Transform the (#fixture_type) into a `Result<#fixture_type, _>` if it is not
                // already a `Result`.
                #convert_result
            }

            fn teardown() -> Option<::rustest::TeardownFn<<Self::Fixt as ::rustest::Fixture>::Type>> {
                #teardown
            }
        }
    };

    let (inner_type, proxy_type) = if let FixtureScope::Once = scope {
        (
            quote! { ::rustest::FixtureTeardown<#fixture_type> },
            quote! { ::rustest::OnceProxy },
        )
    } else {
        (
            quote! { ::rustest::SharedFixtureValue<#fixture_type> },
            quote! { ::rustest::SharedProxy },
        )
    };

    Ok(quote! {
        mod #mod_name {
            use super::*;
            #param_fixture_def
        } // end of inner mod
        #def_tokens

        #vis struct #fixture_name #fixture_generics #where_clause {
            inner: #inner_type,
            #(#phantom_markers),*
        }

        impl #impl_generics ::rustest::Fixture for #fixture_name #ty_generics #where_clause {
            type Type = #fixture_type;
            type Proxy = #proxy_type <#def_name #ty_generics>;
        }

        impl #impl_generics From<#inner_type> for #fixture_name #ty_generics #where_clause {
            fn from(inner : #inner_type) -> Self {
                Self{
                    inner,
                    #(#phantom_builders),*
                }
            }
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
            scope = once,
            fallible = false
        };

        let fixture_attr = parse2::<FixtureAttr>(input).unwrap();

        assert_eq!(fixture_attr.scope, Some(FixtureScope::Once));
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
            "expected one of 'once', 'matrix', 'global', or 'test'. Got invalid_scope."
        );
    }
}
