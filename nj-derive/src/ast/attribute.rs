

use syn::AttributeArgs;
use syn::Result;
use syn::spanned::Spanned;
use syn::Error;
use syn::NestedMeta;
use syn::Meta;
use syn::MetaNameValue;
use syn::Lit;
use syn::LitStr;
use syn::Ident;
use syn::Path;

/// Represents function attribute
/// Attribute can be
/// name="my_function"
/// constructor
/// setter
/// mt
pub enum FunctionAttribute {
    Getter(Ident),
    Setter(Ident),
    Constructor(Ident),
    Name(LitStr),
    MT(Ident)
}

impl FunctionAttribute {

    fn from_ast(attr: NestedMeta) -> Result<Self> {

        match attr {
            NestedMeta::Meta(meta)=> {
                // check meta name value
                match meta {
                    Meta::NameValue(name_value) => {
                        if has_attribute(&name_value,"name") {
                            // check make sure name is str literal
                            match name_value.lit {
                                Lit::Str(str) => Ok(Self::Name(str)),
                                _ => Err(Error::new(name_value.span(), "name is not string"))
                            }
                        } else {
                            Err(Error::new(name_value.span(), "unrecognized attribute"))
                        }
                    },
                    Meta::Path(p) => {
                        
                        let ident = find_any_identifier(p)?;
                        if ident == "constructor" {
                                Ok(Self::Constructor(ident))
                        } else if  ident == "getter" {
                            Ok(Self::Getter(ident))
                        } else if ident == "setter" {
                            Ok(Self::Setter(ident))
                        } else if ident == "mt" {
                            Ok(Self::MT(ident))
                        } else {
                            Err(Error::new(ident.span(), "unrecognized attribute name"))
                        } 
                    }
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

fn find_any_identifier(path: Path) -> Result<Ident> {
    
    if path.segments.len() == 0 {
        Err(Error::new(path.span(),"invalid attribute"))
    } else {
        Ok(path
            .segments
            .into_iter()
            .find(|_| true)
            .map(|segment| segment.ident )
            .unwrap())
    }

                    
}


pub struct FunctionAttributes {
    attrs: Vec<FunctionAttribute>
}

impl FunctionAttributes {

    pub fn from_ast(args: AttributeArgs) -> Result<Self> {

        println!("attrs: {:#?}",args);
        let mut attrs: Vec<FunctionAttribute> = vec![];
    
        for attr in args {
            attrs.push(FunctionAttribute::from_ast(attr)?);
        }
        
        Ok(Self {
            attrs
        })
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