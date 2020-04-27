extern crate proc_macro;

mod function;
mod class;
mod util;
mod parser;
mod ast;

use function::generate_function;
use ast::FunctionAttributes;
use class::generate_class;
use function::FunctionAttribute;
use parser::NodeItem;
use function::FunctionArgMetadata;
use function::FunctionContext;
use proc_macro::TokenStream;

    


/// This turns regular rust function into N-API compatible native module
/// 
/// For example; given rust following here
/// 
///      fn sum(first: i32, second: i32) -> i32 {
///           return first+second
///      }
/// 
/// into N-API module
///     #[no_mangle]
///     pub extern "C" fn n_sum(env: napi_env, cb_info: napi_callback_info) -> napi_value {
///         fn sum(first: i32, second: i32) -> i32 {
///           return first+second
///         }
///         let js_env = JsEnv::new(env);
///         let js_cb = result_to_napi!(js_env.get_cb_info(cb_info, 2),&js_env);
///         let first = result_to_napi!(js_cb.get_value::<i32>(0),&js_env);
///         let second = result_to_napi!(js_cb.get_value::<i32>(0),&js_env);
///         sum(msg).to_js(&js_env)
///     }
#[proc_macro_attribute]
pub fn node_bindgen(args: TokenStream, item: TokenStream) -> TokenStream {

    use syn::AttributeArgs;
    use quote::quote;
    use crate::ast::FunctionAttributes;

    println!("token stream 1: args: {:#?}",args);
// let parsed_item = syn::parse_macro_input!(item as NodeItem);
    let attribute_args = syn::parse_macro_input!(args as AttributeArgs);
    match FunctionAttributes::from_ast(attribute_args) {
        Ok(_) => {},
        Err(err) => return err.to_compile_error().into()
    }

    /*
    let expand_expression = match parsed_item {
        NodeItem::Function(fn_item) => generate_function(fn_item,attribute_args),
        NodeItem::Impl(impl_item) => generate_class(impl_item)
    };
    */
    
    let expand_expression = quote! {

    };
    
    expand_expression.into()
}
