use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Signature, Type};

// Generate the fixture call from the function signature.
// For each argument in the signature, we must :
// - Build a fixture
// - Generate the call argument
pub(crate) fn gen_fixture_call(
    sig: &Signature,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>), TokenStream> {
    let mut fixtures_build = vec![];
    let mut call_args = vec![];

    for idx in 0..sig.inputs.len() {
        let pat = &syn::Ident::new(&format!("__fixt_{}", idx), Span::call_site());
        fixtures_build.push(quote! { ctx.get_fixture()? });
        call_args.push(quote! {#pat});
    }
    Ok((fixtures_build, call_args))
}

pub(crate) fn gen_param_fixture(params: &Option<(Type, Expr)>) -> TokenStream {
    if let Some((param_type, expr)) = params {
        quote! {
            #[derive(Clone)]
            pub struct Param(#param_type);
            impl Param
            {
                fn new(inner: #param_type) -> Self {
                    Self(inner)
                }
            }

            impl ::rustest::FixtureDisplay for Param
            {
                fn display(&self) -> String {
                    self.0.display()
                }
            }

            impl ::rustest::Fixture for Param
             {
                type InnerType = #param_type;
                type Type = #param_type;

                fn setup(ctx: &mut ::rustest::TestContext) -> std::result::Result<Vec<Self>, ::rustest::FixtureCreationError> {
                    Ok(#expr.into_iter().map(|i| Self::new(i)).collect())
                }

                fn build(&self) -> Self {
                    self.clone()
                }

                fn scope() -> ::rustest::FixtureScope { ::rustest::FixtureScope::Test }
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
