use core::{todo, unreachable};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
    AngleBracketedGenericArguments, FnArg, GenericParam, ItemFn, PathArguments, ReturnType,
    TypeParam, spanned::Spanned,
};

use syn::parse::{Parse, ParseStream};

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
    let where_preticate = where_clause.as_ref().map(|wc| &wc.predicates);
    let (fallible, fixture_type) = get_fixture_type(&sig)?;
    let fallible = args.fallible.unwrap_or(fallible);
    let scope = args
        .scope
        .or(Some(FixtureScope::Unique))
        .map(TokenStream::from);

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
