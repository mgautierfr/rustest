use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, FnArg, Generics, Ident, PatType, PathArguments, Signature, Type, TypePath};

// Generate the fixture call from the function signature.
// For each argument in the signature, we must :
// - Build a fixture
// - Generate the call argument
pub(crate) fn gen_fixture_call(
    sig: &Signature,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>, TokenStream), TokenStream> {
    let mut fixtures_build = vec![];
    let mut call_args = vec![];
    for (idx, fnarg) in sig.inputs.iter().enumerate() {
        let pat = &syn::Ident::new(&format!("__fixt_{}", idx), Span::call_site());
        if let FnArg::Typed(PatType { ty, .. }) = fnarg {
            if let Type::Path(TypePath { path, .. }) = ty.as_ref() {
                let mut new_path = path.clone();
                let last_segment = new_path.segments.last_mut().unwrap();
                if let PathArguments::AngleBracketed(g) = &last_segment.arguments {
                    let gene: Generics = syn::parse_quote! { #g };
                    let gene = gene.split_for_impl();
                    let turbo_fish = gene.1.as_turbofish();
                    last_segment.arguments = PathArguments::None;
                    fixtures_build.push(
                        quote! { <#new_path #turbo_fish as ::rustest::Fixture>::Builder::setup(ctx)? },
                    );
                } else {
                    fixtures_build
                        .push(quote! { <#new_path as ::rustest::Fixture>::Builder::setup(ctx)? });
                }
            } else {
                return Err(syn::Error::new_spanned(ty, "Invalid arg type").to_compile_error());
            }
        } else {
            return Err(syn::Error::new_spanned(fnarg, "Invalid arg type").to_compile_error());
        };
        call_args.push(quote! {#pat});
    }
    let call_args_input = if call_args.is_empty() {
        quote! { ::rustest::CallArgs(()) }
    } else if call_args.len() == 1 {
        quote! { ::rustest::CallArgs((#(#call_args),*,)) }
    } else {
        quote! { ::rustest::CallArgs((#(#call_args),*)) }
    };
    Ok((fixtures_build, call_args, call_args_input))
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
            pub struct Param(#param_type);
            #[derive(Clone)]
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
                type InnerType = #param_type;
                type Type = #param_type;
                type Fixt = Param;

                fn setup(ctx: &mut ::rustest::TestContext) -> std::result::Result<Vec<Self>, ::rustest::FixtureCreationError> {
                    Ok(#expr.into_iter().map(|i| Self::new(i)).collect())
                }

                fn build(&self) -> Self::Fixt {
                    Param(self.0.clone())
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
