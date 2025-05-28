use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, FnArg, Ident, PatType, PathArguments, Signature, Type, TypePath, Visibility};

pub fn to_tuple(input: &[TokenStream]) -> TokenStream {
    if input.is_empty() {
        quote! { () }
    } else if input.len() == 1 {
        quote! { (#(#input),*,) }
    } else {
        quote! { (#(#input),*) }
    }
}

pub fn to_call_args(input: &[TokenStream]) -> TokenStream {
    let tuple = to_tuple(input);
    quote! { ::rustest::CallArgs(#tuple) }
}

pub struct FixtureInfo {
    pub sub_fixtures_proxies: Vec<TokenStream>,
    pub sub_fixtures: Vec<TokenStream>,
    pub sub_fixtures_inputs: Vec<TokenStream>,
}

// Generate the fixture call from the function signature.
// For each argument in the signature, we must :
// - Build a fixture
// - Generate the call argument
pub(crate) fn gen_fixture_call(
    sig: &Signature,
    mod_name: Option<&Ident>,
) -> Result<FixtureInfo, TokenStream> {
    let mut sub_fixtures_proxies = vec![];
    let mut sub_fixtures = vec![];
    let mut sub_fixtures_inputs = vec![];
    for (idx, fnarg) in sig.inputs.iter().enumerate() {
        let pat = &syn::Ident::new(&format!("__fixt_{}", idx), Span::call_site());
        if let FnArg::Typed(PatType { ty, .. }) = fnarg {
            if let Type::Path(TypePath { path, .. }) = ty.as_ref() {
                let mut new_path = if path.is_ident("Param") && mod_name.is_some() {
                    sub_fixtures.push(quote! { #mod_name::#ty });
                    syn::parse_quote!( #mod_name::#ty)
                } else {
                    sub_fixtures.push(quote! { #ty });
                    path.clone()
                };
                let last_segment = new_path.segments.last_mut().unwrap();
                if let PathArguments::AngleBracketed(_) = last_segment.arguments {
                    let g = std::mem::take(&mut last_segment.arguments);
                    sub_fixtures_proxies
                        .push(quote! { <#new_path :: #g as ::rustest::Fixture>::Proxy });
                } else {
                    sub_fixtures_proxies.push(quote! { <#new_path as ::rustest::Fixture>::Proxy });
                }
            } else {
                return Err(syn::Error::new_spanned(ty, "Invalid arg type").to_compile_error());
            }
        } else {
            return Err(syn::Error::new_spanned(fnarg, "Invalid arg type").to_compile_error());
        };
        sub_fixtures_inputs.push(quote! {#pat});
    }
    Ok(FixtureInfo {
        sub_fixtures_proxies,
        sub_fixtures,
        sub_fixtures_inputs,
    })
}

pub(crate) fn gen_param_fixture(
    params: &Option<(Visibility, Type, Expr)>,
    fixture_name: Option<&Ident>,
) -> TokenStream {
    let test_name_format = if let Some(i) = fixture_name {
        format!("{}:{{}}", i)
    } else {
        "{}".to_owned()
    };
    if let Some((visibility, param_type, expr)) = params {
        let visibility = if let Visibility::Inherited = visibility {
            quote! { pub }
        } else {
            quote! { #visibility }
        };
        quote! {
            #visibility struct Param(pub #param_type);
            #visibility struct ParamProxy {
                v: #param_type,
                name: String
            }
            impl ParamProxy
            {
                fn new<T>(inner: T) -> Self
                where
                    T: ::rustest::ToParamName<#param_type>
                {
                    let (v, name) = inner.into();
                    let name = format!(#test_name_format, name);
                    Self{v, name}
                }
            }

            impl ::rustest::Duplicate for ParamProxy {
                fn duplicate(&self) -> Self {
                    Self{
                        v: self.v.clone(),
                        name: self.name.clone()
                    }
                }
            }

            impl ::rustest::TestName for ParamProxy
            {
                fn name(&self) -> Option<String> {
                    Some(self.name.clone())
                }
            }

            impl ::rustest::FixtureProxy for ParamProxy
             {
                type Fixt = Param;
                const SCOPE : ::rustest::FixtureScope = ::rustest::FixtureScope::Test;

                fn setup(ctx: &mut ::rustest::TestContext) -> Vec<Self> {
                    #expr.into_iter().map(Self::new).collect()
                }

                fn build(self) -> ::rustest::FixtureCreationResult<Self::Fixt> {
                    Ok(Param(self.v))
                }
            }

            impl ::rustest::Fixture for Param {
                type Type = #param_type;
                type Proxy = ParamProxy;
            }

            impl std::ops::Deref for Param
            {
                type Target = #param_type;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        }
    } else {
        quote! {}
    }
}
