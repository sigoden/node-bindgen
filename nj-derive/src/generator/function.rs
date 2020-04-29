use quote::quote;
use proc_macro2::TokenStream;
use syn::ItemFn;
use syn::Ident;

use crate::ast::FunctionArgs;
use crate::ast::FunctionAttributes;

pub struct FunctionGenerator<'a> {
    args: FunctionArgs<'a>,
    attributes: FunctionAttributes,
    input_fn: &'a ItemFn
}

impl <'a>FunctionGenerator<'a> {

    pub fn new(input_fn: &'a ItemFn,args: FunctionArgs<'a>,attributes: FunctionAttributes) -> Self {
        Self {
            input_fn,
            args,
            attributes
        }
    }

    
    pub fn generate(&self) -> TokenStream {

        quote! {
            #(self.input_fn)
        }

    }

    /// function name identifier
    fn name(&self) -> &Ident {
        &self.input_fn.sig.ident
    }
    
}


 




