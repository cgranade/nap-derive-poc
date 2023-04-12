//! The types in this module are intended to sit partway between
//! Syn and Nushell's data models, to help make it easier to convert between
//! the two.
//! 
//! As a result, there is some nontrivial duplication with each, but the
//! intent is that these types are easy to fill incrementally as we walk
//! through token streams, and are also easy to convert out to Nushell
//! signatures when we're done walking.
use proc_macro2::TokenStream;
use syn::{Type, PathArguments, GenericArgument, Field, parse_quote};
use quote::quote;

pub enum ArgKind {
    Required(ArgType),
    Optional(ArgType),
    Flag(Option<ArgType>),
    Invalid
}

pub enum ArgType {
    String,
    Bool
}

impl ArgType {
    pub fn from_type(ty: &Type) -> Option<Self> {
        if let Type::Path(ref path) = ty {
            match path.path.simple_path().as_str() {
                "String" => Some(ArgType::String),
                "bool" => Some(ArgType::Bool),
                _ => None
            }
        } else {
            None
        }
    }

    pub fn syntax_shape_quote(&self) -> TokenStream {
        match self {
            ArgType::String => quote! { nu_protocol::SyntaxShape::String },
            ArgType::Bool => quote! { nu_protocol::SyntaxShape::Boolean }
        }
    }
}

impl ArgKind {
    pub fn from_type(ty: &Type) -> Self {
        // TODO: fix unwraps here.
        match ty.option_type() {
            None =>
                ArgKind::Required(ArgType::from_type(ty).unwrap()),
            Some(ref inner_type) =>
                ArgKind::Optional(ArgType::from_type(inner_type).unwrap())
        }
    }

    pub fn from_field(f: &Field) -> Self {
        // Look for the req, opt, and flag attributes, and in that order of
        // priority.
        for attr in &f.attrs {
            if attr.path().is_ident("req") {
                return match ArgKind::from_type(&f.ty) {
                    ArgKind::Required(t) => ArgKind::Required(t),
                    _ => panic!("Field has #[req] attribute, but an Option type.")
                };
            } else if attr.path().is_ident("opt") {
                return match ArgKind::from_type(&f.ty) {
                    ArgKind::Optional(t) => ArgKind::Optional(t),
                    _ => panic!("Field has #[opt] attribute, but did not have an Option type.")
                };
            } else if attr.path().is_ident("flag") {
                return match ArgKind::from_type(&f.ty) {
                    ArgKind::Optional(t) => ArgKind::Flag(Some(t)),
                    ArgKind::Required(ArgType::Bool) => ArgKind::Flag(None),
                    _ => panic!("Field has #[flag] attribute, but did not have a bool or an Option type.")
                };
            }
        }
        ArgKind::Invalid
    }
}

pub struct Arg {
    pub name: String,
    pub kind: ArgKind,
    pub usage: Option<String>,
}

impl Arg {
    pub fn from_field(field: &Field) -> Self {
        Arg {
            name: field.ident.as_ref().unwrap().to_string(),
            kind: ArgKind::from_field(field),
            usage: field.find_attr_str("usage")
        }
    }

    pub fn usage_quote(&self) -> TokenStream {
        match &self.usage {
            None => parse_quote! { "" },
            Some(usage) => parse_quote! { #usage }
        }
    }
}

trait FieldExt: Sized {
    fn find_attr_str(&self, name: &str) -> Option<String>;
}

impl FieldExt for syn::Field {
    fn find_attr_str(&self, name: &str) -> Option<String> {
        // Look for the req, opt, and flag attributes, and in that order of
        // priority.
        for attr in &self.attrs {
            if attr.path().is_ident(name) {
                // TODO: propagate out compiler error instead of unwrapping.
                let arg = attr.parse_args::<syn::LitStr>().unwrap();
                return Some(arg.value());
            }
        }
        None
    }
}

trait PathExt: Sized {
    fn simple_path(&self) -> String;
}
impl PathExt for syn::Path {
    fn simple_path(&self) -> String {
        self
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}

trait TypeExt: Sized {
    fn option_type(&self) -> Option<Self>;
}
impl TypeExt for syn::Type {
    fn option_type(&self) -> Option<Self> {
        match self {
            syn::Type::Path(ref path) => {
                match path.path.simple_path().as_str() {
                    "Option" | "std::option::Option" | "core::option::Option" =>
                        Some({
                            let path_args = &path
                                .path
                                .segments
                                .last()?
                                .arguments;
                            if let PathArguments::AngleBracketed(params) = path_args {
                                if let GenericArgument::Type(inner_ty) = params.args.first()? {
                                    inner_ty.clone()
                                } else {
                                    None?
                                }
                            } else {
                                None?
                            }
                        }),
                    _ => None
                }
            },
            _ => None
        }
    }
}
