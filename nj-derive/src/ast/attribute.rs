
use proc_macro2::Literal;
use syn::AttributeArgs;
use syn::{ Result, Token};
use syn::parse::{Parse, ParseStream};
use syn::ItemImpl;
use syn::ItemFn;
use syn::spanned::Spanned;
use syn::Error;
use syn::NestedMeta;
use syn::Meta;
use syn::MetaNameValue;

/// Represents function attribute
/// Attribute can be
/// name="my_function"
/// constructor
/// setter
/// mt
pub enum FunctionAttribute {
    Getter(NestedMeta),
    Setter(NestedMeta),
    Constructor(NestedMeta),
    Name(MetaNameValue),
    MT(NestedMeta)
}

impl FunctionAttribute {

    fn from_ast(attr: NestedMeta) -> Result<Self> {

        match attr {
            NestedMeta::Meta(meta)=> {
                // check meta name value
                match meta {
                    Meta::NameValue(name_value) => {
                        if has_attribute(&name_value,"name") {
                            Ok(Self::Name(name_value))
                        } else {
                            Err(Error::new(name_value.span(), "unrecognized attribute"))
                        }
                    },
                    Meta::Path(p) => Err(Error::new(p.span(), "unrecognized attribute")),
                    Meta::List(l) => Err(Error::new(l.span(),"unrecognized attribute"))
                }
            },
            NestedMeta::Lit(lit) => Err(Error::new(lit.span(),"unrecognized attribute"))
        }

    }
}

fn has_attribute(name_value: &MetaNameValue,attr_name: &str) -> bool {

    name_value.path
        .segments
        .iter()
        .find(|seg| seg.ident == attr_name)
        .is_some()
}


pub struct FunctionAttributes {
    attr: Vec<FunctionAttribute>
}

impl FunctionAttributes {

    pub fn from_ast(attrs: AttributeArgs) -> Result<()> {

        println!("attrs: {:#?}",attrs);
        let mut f_attrs: Vec<FunctionAttribute> = vec![];
    
        for attr in attrs {
            f_attrs.push(FunctionAttribute::from_ast(attr)?);
        }
        
        Ok(())

    }
}



/*
impl Parse for FunctionAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let args = input.parse::<AttributeArgs>()?;
            
        Ok(args)
    }
}

*/