
use syn::FnArg;
use syn::Ident;
use syn::Type;
use syn::Pat;
use syn::Error;
use syn::Generics;
use syn::TypeParam;
use syn::ItemFn;
use syn::Signature;
use syn::TypeParamBound;
use syn::PathArguments;
use syn::Result;
use syn::spanned::Spanned;
use syn::ParenthesizedGenericArguments;
use proc_macro2::Span;

use crate::util::MyTypePath;
use crate::util::MyReferenceType;


/// Represents functional arguments
#[derive(Debug)]
pub struct FunctionArgs<'a> {
    args: Vec<FunctionArg<'a>>,
    is_method: bool
}

impl <'a>FunctionArgs<'a> {

    pub fn from_ast(input_fn: &'a ItemFn) -> Result<Self> {

        println!("fn: {:#?}",input_fn);

        let sig = &input_fn.sig;
        let generics = &sig.generics;

        let mut args: Vec<FunctionArg> = vec![];

        let is_method = has_receiver(sig);

        // extract arguments,
        let i = 0;
        for ref arg in &sig.inputs {
            match arg {
                FnArg::Receiver(_) => {}
                FnArg::Typed(arg_type) => {
                 
                    match &*arg_type.pat {
                        Pat::Ident(identity) => {
                         
                            let arg = FunctionArg::new(
                                i,
                                &identity.ident,
                                &*arg_type.ty,
                                generics
                            )?;
                            args.push(arg);
                        },
                        _ => return Err(Error::new(arg_type.span(), "not supported type")),
                    }
                }
            }
        }

        Ok(Self {
            args,
            is_method
        })

    }
    

    fn is_method(&self) -> bool {
        self.is_method
    }


}


/// find receiver if any, this will be used to indicate if this is method
fn has_receiver(sig: &Signature) -> bool {

    sig.inputs.iter().find(|input| {
        match input {
            FnArg::Receiver(rec) => true,
            _ => false
        }
    }).is_some()
}


#[derive(Debug)]
pub struct FunctionArg<'a> {
    arg_index: u32,
    typ: FunctionArgType<'a>,
}

impl <'a> FunctionArg<'a>  {

    /// given this, convert into normalized type signature
    fn new(arg_index: u32, ident: &'a Ident, ty: &'a Type, generics: &'a Generics) -> Result<Self> {

        match ty {
            Type::Path(path_type) => {
                let my_type = MyTypePath::new(path_type);

                // check whether type references in the generic indicates this is closure
                if let Some(param) = find_generic(generics,my_type.type_name().as_ref().unwrap()) {
                    if let Some(closure) = ClosureType::from(ident,param) {
                        Ok(Self {
                            arg_index,
                            typ: FunctionArgType::Closure(closure)
                        })
                    } else {
                        Err(Error::new(ty.span(), "not supported closure type"))
                    }
                } else {
                    Ok(Self {
                        arg_index,
                        typ: FunctionArgType::Path(my_type)
                    })
                }
            }
            Type::Reference(ref_type) => {
                let my_type = MyReferenceType::new(ref_type);
                if my_type.is_callback() {
                    Ok(Self {
                        arg_index,
                        typ: FunctionArgType::JSCallback(my_type)
                    })
                } else {
                    Ok(Self {
                        arg_index,
                        typ: FunctionArgType::Ref(my_type)
                    })
                }
            }
            _ => Err(Error::new(ty.span(), "not supported type"))
        }
    }
}


/// Categorize function argument
#[derive(Debug)]
enum FunctionArgType<'a> {
    Path(MyTypePath<'a>),            // normal type
    Ref(MyReferenceType<'a>),                          // reference type
    Closure(ClosureType<'a>),        // callback
    JSCallback(MyReferenceType<'a>), // indicating that we want to receive typed JsCallback
}


/// find generic with match ident
fn find_generic<'a,'b>(generics: &'a Generics, ident: &'b Ident) -> Option<&'a TypeParam> {

    generics.type_params().find(|ty| ty.ident.to_string() == ident.to_string() )

}


#[derive(Debug)]
struct ClosureType<'a> {
    ty: &'a ParenthesizedGenericArguments,
    ident: &'a Ident,
}

impl <'a>ClosureType<'a> {
    // try to see if we can find closure, otherwise return none
    fn from(ident: &'a Ident,param: &'a TypeParam) -> Option<Self> {
        for ref bound in &param.bounds {
            match bound {
                TypeParamBound::Trait(tt) => {
                    for ref segment in &tt.path.segments {
                        match segment.arguments {
                            PathArguments::Parenthesized(ref path) => return Some(Self {
                                ident,
                                ty: path,
                            }),
                            _ => return None,
                        }
                    }
                    return None;
                }
                TypeParamBound::Lifetime(_) => return None,
            }
        }
        None
    }

    /*
    fn type_name(&self) -> Ident {
        
        let callback_type = if self.multi_threaded {
            "JsMultiThreadedCallbackFunction"
        } else {
            "JsCallbackFunction"
        };

        Ident::new(callback_type, Span::call_site())
    }
    */

}