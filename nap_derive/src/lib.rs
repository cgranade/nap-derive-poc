mod arguments;

use std::{collections::HashMap};

use arguments::{ArgKind, Arg};
use quote::{quote};
use syn::{parse_macro_input, DeriveInput, Expr, punctuated::Punctuated, Token, LitStr, parse_quote, Data, FieldValue, ExprStruct, Arm};

#[proc_macro_derive(PluginSignatures, attributes(signature, req, opt, flag, usage))]
pub fn derive_plugin_signatures(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_data = if let Data::Enum(enum_data) = input.data {
        enum_data
    } else {
        panic!("Expected to derive signatures for an enum, but got {:?}.", match input.data {
            Data::Struct(_) => "struct",
            Data::Union(_) => "union",
            Data::Enum(_) => "enum"
        });
    };
    
    let name = input.ident;
    let mut signature_data: HashMap<String, (Expr, ExprStruct, Option<Expr>)> =
        enum_data
            .variants
            .into_iter()
            .map(|variant| {
                let mut call_name: Option<String> = None;
                let variant_name = variant.ident.clone();
                let mut call_parsers: Vec<FieldValue> = vec![];
                let mut usage = None;

                // Process attributes on the entire variant.
                // TODO: allow adding examples and help from attributes.
                for attr in &variant.attrs {
                    if attr.path().is_ident("signature") {
                        // FIXME: generate compiler error instead
                        let lit: LitStr = attr.parse_args().unwrap();
                        call_name = Some(lit.value());
                    }

                    if attr.path().is_ident("usage") {
                        // FIXME: generate compiler error instead
                        let lit: LitStr = attr.parse_args().unwrap();
                        usage = Some(lit.value());
                    }
                }

                let call_name = call_name.unwrap();
                let call_name_expr: Expr = parse_quote!(#call_name);

                let mut compiler_errors = Vec::<Expr>::new();
                let mut sig_builder: Expr = parse_quote! {
                    nu_protocol::PluginSignature::build(#call_name_expr)
                };
                
                if let Some(usage) = usage {
                    sig_builder = parse_quote! {
                        #sig_builder
                            .usage(#usage)
                    }
                }

                // Process individual fields.
                let mut req_idx = 0usize;
                let mut seen_opt_yet = false;
                for field in variant.fields {
                    // TODO: look at attributes for help and the like.

                    let arg = Arg::from_field(&field);

                    seen_opt_yet |= !matches!(arg.kind, ArgKind::Required(_));

                    // Add the required argument to both the signature and the call
                    // parser.
                    let field_ident = field.ident.unwrap();
                    let field_usage = arg.usage_quote();
                    let field_name = arg.name;
                    match arg.kind {
                        ArgKind::Required(_) if seen_opt_yet => {
                            compiler_errors.push(parse_quote! {
                                compile_error!(
                                    "Required arguments may not follow optional or flag arguments."
                                )
                            });
                        },
                        ArgKind::Required(ty) => {
                            let shape = ty.syntax_shape_quote();
                            sig_builder = parse_quote! {
                                #sig_builder
                                    .required(#field_name, #shape, #field_usage)
                            };
                            call_parsers.push(parse_quote! {
                                #field_ident: call.req(#req_idx)?
                            });
                            req_idx += 1;
                        },
                        ArgKind::Optional(ty) => {
                            let shape = ty.syntax_shape_quote();
                            sig_builder = parse_quote! {
                                #sig_builder
                                    .optional(#field_name, #shape, #field_usage)
                            };
                            call_parsers.push(parse_quote! {
                                #field_ident: call.opt(#req_idx)?
                            });
                            req_idx += 1;
                        },
                        ArgKind::Flag(None) => {
                            // TODO: Allow short names for flags instead of
                            //       None here.
                            sig_builder = parse_quote! {
                                #sig_builder
                                    .switch(#field_name, #field_usage, None)
                            };
                            call_parsers.push(parse_quote! {
                                #field_ident: call.has_flag(#field_name)
                            });
                        },
                        ArgKind::Flag(Some(ty)) => {
                            // TODO: Allow short names for flags instead of
                            //       None here.
                            let shape = ty.syntax_shape_quote();
                            sig_builder = parse_quote! {
                                #sig_builder
                                    .named(#field_name, #shape, #field_usage, None)
                            };
                            call_parsers.push(parse_quote! {
                                #field_ident: call.get_flag(#field_name)?
                            });
                        }
                        _ => todo!()
                    }
                }

                // TODO: fix spans
                let call_parsers =
                    Punctuated::<FieldValue, Token![,]>::from_iter(call_parsers);
                let compiler_errors = if !compiler_errors.is_empty() {
                    let compiler_errors = Punctuated::<Expr, Token![;]>::from_iter(compiler_errors);
                    Some(parse_quote! { #compiler_errors })
                } else {
                    None
                };

                (
                    call_name,
                    (
                        sig_builder,
                        parse_quote! {
                            #name :: #variant_name {
                                #call_parsers
                            }
                        },
                        compiler_errors
                    )
                )
            })
        .collect();

    let mut signature_builders = vec![];
    let mut signature_parsers = vec![];
    for s in signature_data.drain() {
        signature_builders.push(s.1.0);
        if let Some(errors) = s.1.2 {
            signature_builders.push(errors);
        }
        let name = s.0;
        let arm_expr = s.1.1;
        let arm: Arm = parse_quote! {
            #name => #arm_expr
        };
        signature_parsers.push(arm);
    }
    let signature_builders = Punctuated::<Expr, Token![,]>::from_iter(signature_builders);
    let signature_parsers = Punctuated::<Arm, Token![,]>::from_iter(signature_parsers);

    let parser_impl = quote! {
        fn parse_call(name: &str, call: &nu_plugin::EvaluatedCall) -> Result<Self, nu_plugin::LabeledError> {
            Ok(match name {
                #signature_parsers,
                _ => Err(nu_plugin::LabeledError {
                    label: "Plugin call with wrong name signature".into(),
                    msg: "The signature used to call the plugin does not match any known signature.".into(),
                    span: Some(call.head)
                })?
            })
        }
    };

    let expanded = quote! {
        use nu_protocol;

        impl nap::PluginSignatures for #name {
            fn signature() -> Vec<nu_protocol::PluginSignature> {
                vec![
                    #signature_builders
                ]
            }

            #parser_impl
        }
    };

    proc_macro::TokenStream::from(expanded)
}
