use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, FnArg, Signature};

// Generate the fixture call from the function signature.
// For each argument in the signature, we must :
// - Build a fixture
// - Generate the call argument
pub(crate) fn gen_fixture_call(
    params: Option<&Expr>,
    sig: &Signature,
) -> Result<(Vec<TokenStream>, Vec<TokenStream>), TokenStream> {
    let mut fixtures_build = vec![];
    let mut call_args = vec![];

    for fnarg in sig.inputs.iter() {
        if let FnArg::Typed(typed_fnarg) = fnarg {
            let pat = if let syn::Pat::Ident(patident) = typed_fnarg.pat.as_ref() {
                &patident.ident
            } else {
                return Err(
                    syn::Error::new_spanned(fnarg, "expected an identifier").to_compile_error()
                );
            };
            if pat == "param" && params.is_some() {
                fixtures_build.push(
                    quote! { #params.into_iter().map(|i| ::rustest::FixtureParam(i)).collect::<Vec<_>>() },
                );
            } else {
                fixtures_build.push(quote! {::rustest::get_fixture(ctx)?});
            }
            call_args.push(quote! {#pat});
        }
    }
    Ok((fixtures_build, call_args))
}
