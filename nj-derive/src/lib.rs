extern crate proc_macro;


mod util;
mod ast;
mod generator;

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

    use ast::FunctionAttributes;
    use ast::FunctionArgs;
    use ast::NodeItem;    
    use generator::FunctionGenerator;

    let attribute_args = syn::parse_macro_input!(args as AttributeArgs);
    
    let attribute: FunctionAttributes = 
        match FunctionAttributes::from_ast(attribute_args) {
            Ok(attr) => attr,
            Err(err) => return err.to_compile_error().into()
        };

    let parsed_item = syn::parse_macro_input!(item as NodeItem);
    let out_express = match parsed_item {
        NodeItem::Function(fn_item) => {
            
            match FunctionArgs::from_ast(&fn_item) {
                Err(err) => return err.to_compile_error().into(),
                Ok(args) => {
                    println!("args: {:#?}",args);

                    let generator = FunctionGenerator::new(&fn_item,args,attribute);
                    quote! {
                        #fn_item
                    }
                }
                
            }
        }
        NodeItem::Impl(impl_item) => {
            quote! {

            }
        }
    };
    
   // println!("{:#?}",out_express);
    
    out_express.into()
}
