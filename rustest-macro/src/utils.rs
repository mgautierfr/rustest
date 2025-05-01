use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, FnArg, Generics, Ident, PatType, PathArguments, Signature, Type, TypePath};

pub fn to_tuple(input: &Vec<TokenStream>) -> TokenStream {
    if input.is_empty() {
        quote! { () }
    } else if input.len() == 1 {
        quote! { (#(#input),*,) }
    } else {
        quote! { (#(#input),*) }
    }
}

pub struct FixtureInfo {
    pub sub_fixtures_builders: Vec<TokenStream>,
    pub sub_fixtures: Vec<TokenStream>,
    pub sub_fixtures_inputs: Vec<TokenStream>,
    pub sub_fixtures_call_args: TokenStream,
}

// Generate the fixture call from the function signature.
// For each argument in the signature, we must :
// - Build a fixture
// - Generate the call argument
pub(crate) fn gen_fixture_call(sig: &Signature) -> Result<FixtureInfo, TokenStream> {
    let mut sub_fixtures_builders = vec![];
    let mut sub_fixtures = vec![];
    let mut sub_fixtures_inputs = vec![];
    for (idx, fnarg) in sig.inputs.iter().enumerate() {
        let pat = &syn::Ident::new(&format!("__fixt_{}", idx), Span::call_site());
        if let FnArg::Typed(PatType { ty, .. }) = fnarg {
            sub_fixtures.push(quote! { #ty });
            if let Type::Path(TypePath { path, .. }) = ty.as_ref() {
                let mut new_path = path.clone();
                let last_segment = new_path.segments.last_mut().unwrap();
                if let PathArguments::AngleBracketed(g) = &last_segment.arguments {
                    let gene: Generics = syn::parse_quote! { #g };
                    let gene = gene.split_for_impl();
                    let turbo_fish = gene.1.as_turbofish();
                    last_segment.arguments = PathArguments::None;
                    sub_fixtures_builders
                        .push(quote! { <#new_path #turbo_fish as ::rustest::Fixture>::Builder });
                } else {
                    sub_fixtures_builders
                        .push(quote! { <#new_path as ::rustest::Fixture>::Builder });
                }
            } else {
                return Err(syn::Error::new_spanned(ty, "Invalid arg type").to_compile_error());
            }
        } else {
            return Err(syn::Error::new_spanned(fnarg, "Invalid arg type").to_compile_error());
        };
        sub_fixtures_inputs.push(quote! {#pat});
    }
    let sub_fixtures_inputs_tuple = to_tuple(&sub_fixtures_inputs);
    let sub_fixtures_call_args = quote! { ::rustest::CallArgs(#sub_fixtures_inputs_tuple) };
    Ok(FixtureInfo {
        sub_fixtures_builders,
        sub_fixtures,
        sub_fixtures_inputs,
        sub_fixtures_call_args,
    })
}

pub(crate) fn gen_param_fixture(
    params: &Option<(Type, Expr)>,
    fixture_name: Option<&Ident>,
) -> TokenStream {
    let display_format = if let Some(i) = fixture_name {
        format!("{}:{{}}", i)
    } else {
        "{}".to_owned()
    };
    if let Some((param_type, expr)) = params {
        quote! {
            #[derive(Debug)]
            pub struct Param(#param_type);
            #[derive(Clone, Debug)]
            pub struct ParamBuilder(#param_type);
            impl ParamBuilder
            {
                fn new(inner: #param_type) -> Self {
                    Self(inner)
                }
            }

            impl ::rustest::FixtureDisplay for ParamBuilder
            {
                fn display(&self) -> Option<String> {
                    // Param value should always be display
                    Some(format!(#display_format, self.0.display().unwrap()))
                }
            }

            impl ::rustest::FixtureBuilder for ParamBuilder
             {
                type Type = #param_type;
                type Fixt = Param;

                fn setup(ctx: &mut ::rustest::TestContext) -> std::result::Result<Vec<Self>, ::rustest::FixtureCreationError> {
                    Ok(#expr.into_iter().map(|i| Self::new(i)).collect())
                }

                fn build(&self) -> std::result::Result<Self::Fixt, ::rustest::FixtureCreationError> {
                    Ok(Param(self.0.clone()))
                }

                fn scope() -> ::rustest::FixtureScope { ::rustest::FixtureScope::Test }
            }

            impl ::rustest::Fixture for Param {
                type Type = #param_type;
                type Builder = ParamBuilder;
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
