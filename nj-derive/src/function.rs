use quote::quote;
use syn::FnArg;
use syn::Ident;
use syn::LitInt;
use syn::Type;
use syn::Pat;
use syn::Error;
use syn::LitStr;
use syn::Generics;
use syn::TypeParam;
use syn::ItemFn;
use syn::Signature;
use syn::TypeParamBound;
use syn::PathArguments;
use syn::Receiver;
use syn::AttributeArgs;
use syn::ParenthesizedGenericArguments;
use syn::NestedMeta;
use syn::Meta;
use syn::ReturnType;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::Literal;

use crate::util::MyTypePath;
use crate::util::rust_arg_var;
use crate::util::MyReferenceType;
use crate::util::default_function_property_name;
use crate::ast::FunctionAttributes;

/// parse and generate function
pub fn generate_function(input_fn: ItemFn, attributes: FunctionAttributes) -> TokenStream {
    
    println!("fn: {:#?}",ItemFn);
    let fn_wrapper = FunctionAst::from_ast(input_fn, args);
    fn_wrapper.as_token_stream()
}

pub enum FunctionAttribute {
    Getter,
    Setter,
    Constructor,
    Name(Literal),
    Register,
    Other,
}

struct FunctionAst {
    fn_item: ItemFn,
    args: AttributeArgs,
}

impl FunctionAst {

    /// parse from ast
    fn from_ast(fn_item: ItemFn, args: AttributeArgs) -> Self {
        Self { fn_item, args }
    }

    fn is_async(&self) -> bool {
        self.fn_item.sig.asyncness.is_some()
    }

    /// check whether this function return ()
    fn has_default_output(&self) -> bool {
        match self.fn_item.sig.output {
            ReturnType::Default => true,
            _ => false
        }
    }

    /// check if this method should be constructor
    fn is_constructor(&self) -> bool {
        self.has_attribute("constructor")
    }

    /// check if closure should support multi-threaded
    fn support_multi_threaded(&self) -> bool {
        self.has_attribute("mt")
    }

    /// check if we have attribute, this should be refactored to use attribute parser
    fn has_attribute(&self,attribute: &str) -> bool {
        self.args
            .iter()
            .find(|arg| match arg {
                NestedMeta::Meta(meta) => match meta {
                    Meta::Path(path) => path
                        .segments
                        .iter()
                        .find(|seg| seg.ident == attribute)
                        .is_some(),
                    _ => false,
                },
                _ => false,
            })
            .is_some()
    }

    /// name of the function
    fn name(&self) -> &Ident {
        &self.fn_item.sig.ident
    }

    /// identifier for napi wrapper function
    fn napi_fn_id(&self) -> Ident {
        let n_fn_name = format!("napi_{}", self.name());
        Ident::new(&n_fn_name, Span::call_site())
    }


    fn analyze_args(&self) -> Result<FunctionArgMetadata, Error> {
        FunctionArgMetadata::parse(&self.fn_item.sig,self.support_multi_threaded())
    }

    /// start of code generation
    pub fn as_token_stream(&self) -> TokenStream {

        if self.is_constructor() {
            let item = &self.fn_item;
            return quote! {
                #item
            };
        }

        let args = match self.analyze_args() {
            Ok(data) => data,
            Err(err) => return err.to_compile_error(),
        };

        let napi_code =  self.generate_napi_code(&mut &args);
        let property_code = self.generate_property_code(&args);

        quote! {

            #napi_code

            #property_code

        }
    }


    /// generate native code to be invoked by napi
    fn generate_napi_code(&self, args: &FunctionArgMetadata) -> TokenStream {


        let input_fn = &self.fn_item;
        let ident_n_api_fn = self.napi_fn_id();

        let mut ctx = FunctionContext {
            is_async: self.is_async(),
            is_multi_threaded: args.is_multi_threaded(),
            name: self.name().to_string(),
            ..Default::default()
        };

        let rust_invocation = self.generate_rust_invocation(args,&mut ctx);
        let rust_args_struct= &ctx.cb_args;

        if args.is_method() {

            // if function is method, we can't put rust function inside our napi because we need to preserver self
            // in the rust method.
            let napi_fn = raw_napi_function_template(
                self.napi_fn_id(), 
                quote! {}, 
                rust_args_struct,
                rust_invocation);

            quote! {
                #input_fn

                #napi_fn
            }
        } else {
           
            // otherwise we can put rust function inside to make it tidy
            raw_napi_function_template(
                ident_n_api_fn, 
                quote! { #input_fn }, 
                rust_args_struct,
                rust_invocation)
        }
    }



    /// this code generation does following:
    ///
    /// * extract arguments from napi environment based on rust function arguments type (including type checking)
    /// * invoke rust function (including async wrapper)
    /// * convert rust result back to JS
    fn generate_rust_invocation(&self, function_args: &FunctionArgMetadata,ctx: &mut FunctionContext) -> TokenStream {

    
        // code to convert extract rust values from Js Env
        let js_to_rust_values = function_args.as_arg_token(&ctx);

        // express to invoke rust function
        let receiver = function_args.receiver();
        let rust_invoke = function_args.as_token_stream(self.name(),ctx);
        
        // if this is async, wrap with JsFuture
        let rust_invoke_ft_wrapper = if ctx.is_async {

            if self.has_default_output() {

                // since this doesn't have any output, we don't need return promise, we just
                // spawn async and return null ptr
                quote! {

                    node_bindgen::core::future::spawn(async move {
                        #rust_invoke.await;
                    });

                    Ok(std::ptr::null_mut())
                }

            } else {

                let async_name = format!("{}_ft", self.name());
                let async_lit = LitStr::new(&async_name, Span::call_site());
                quote! {
                    (node_bindgen::core::JsPromiseFuture::new(
                        #rust_invoke,#async_lit
                    )).try_to_js(&js_env)
                }
            }
        } else {
            quote! {
                #rust_invoke.try_to_js(&js_env)
            }
           
        };

        quote! {

            let result: Result<node_bindgen::sys::napi_value,node_bindgen::core::NjError> = ( move || {

                #js_to_rust_values

                #receiver

                #rust_invoke_ft_wrapper

            })();


            result.to_js(&js_env)
        }
    }


    /// generate code to register this function property to global property
    fn generate_property_code(&self, args: &FunctionArgMetadata) -> TokenStream {


        if args.is_method() {
            return quote! {};
        }

        let ident_n_api_fn = self.napi_fn_id();

        let register_fn_name = format!("register_{}", ident_n_api_fn);
        let property_name = default_function_property_name(&self.name().to_string());
        let ident_register_fn = Ident::new(&register_fn_name, Span::call_site());
        let property_name_literal = LitStr::new(&property_name, Span::call_site());

        quote! {
            #[node_bindgen::core::ctor]
            fn #ident_register_fn() {

                let property = node_bindgen::core::Property::new(#property_name_literal).method(#ident_n_api_fn);
                node_bindgen::core::submit_property(property);
            }

        }
    }

    
}

#[derive(Default)]
pub struct FunctionContext {
    is_async: bool,
    is_multi_threaded: bool,
    name: String,
    cb_args: Vec<TokenStream>            // accumulated arg structure
}

/// Represents functional arguments
pub struct FunctionArgMetadata {
    receiver: Option<Receiver>,
    args: Vec<FunctionArg>,
    multi_threaded: bool
}

impl FunctionArgMetadata {
    pub fn parse(sig: &Signature,multi_threaded: bool) -> Result<FunctionArgMetadata, Error> {
        let generics = &sig.generics;

        let mut args: Vec<FunctionArg> = vec![];

        let receiver = Self::find_receiver(sig);

        // extract arguments,
        for arg in &sig.inputs {
            match arg {
                FnArg::Receiver(_) => {}
                FnArg::Typed(arg_type) => {
                 
                    match &*arg_type.pat {
                        Pat::Ident(identity) => {
                         
                            if let Some(arg) = FunctionArg::new(
                                identity.ident.clone(),
                                arg_type.ty.clone(),
                                GenerericInfo::new(generics),
                                multi_threaded
                            ) {
                                args.push(arg);
                            } else {
                                return Err(Error::new(
                                    Span::call_site(),
                                    "not supported arg types",
                                ));
                            }
                        }
                        _ => return Err(Error::new(Span::call_site(), "not supported type")),
                    }
                }
            }
        }

        Ok(FunctionArgMetadata { receiver, args, multi_threaded })
    }

    /// find receiver if any, this will be used to indicate if this is method
    fn find_receiver(sig: &Signature) -> Option<Receiver> {
        for arg in &sig.inputs {
            match arg {
                FnArg::Receiver(rec) => return Some(rec.clone()),
                _ => {}
            }
        }

        None
    }

    fn is_method(&self) -> bool {
        self.receiver.is_some()
    }

    fn is_multi_threaded(&self) -> bool {
        self.multi_threaded
    }

    /// used as argument
    pub fn constructor_args(&self) -> TokenStream {
        let args = self
            .args
            .iter()
            .filter(|arg| !arg.is_callback())
            .enumerate()
            .map(|(i, arg)| arg.as_rust_fn_arg(i));

        quote! {
            #(#args)*
        }
    }

    /// suitable for used in constructor create
    pub fn constructor_new(&self) -> TokenStream {
        let args = self
            .args
            .iter()
            .filter(|arg| !arg.is_callback())
            .enumerate()
            .map(|(i, arg)| arg.as_struct_arg(i));

        quote! {
            #(#args)*
        }
    }

    /// generate tokens for extracting arguments from JS
    pub fn rust_args_input(&self,ctx: &mut FunctionContext) -> Vec<TokenStream> {
        let mut arg_index = 0;
        self.args
            .iter()
            .map(|arg| arg.as_arg_token_stream(&mut arg_index,ctx))
            .collect()
    }

    /// size of the rust arguments
    fn rust_args_len(&self) -> usize {
        self.args.iter().filter(|arg| !arg.is_callback()).count()
    }

    /// generate code to extract cb based on method signature
    /// first it extract cb info based on argument count
    /// then it generates rust variables based on arguments conversion.
    pub fn as_arg_token(&self, ctx: &FunctionContext) -> TokenStream {
        let arg_len = self.rust_args_len();
        let count_literal = arg_len.to_string();
        let js_count = LitInt::new(&count_literal, Span::call_site());

        let rust_args: Vec<TokenStream> = self
            .args
            .iter()
            .filter(|arg| !arg.is_callback())
            .enumerate()
            .map(|(i, arg)| arg.js_to_rust_token_stream(i, ctx))
            .collect();

        quote! {

            let js_cb = js_env.get_cb_info(cb_info, #js_count)?;
            #(#rust_args)*

        }
    }

    /// generate expression to convert constructor into new instance
    pub fn as_constructor_try_to_js(&self) -> TokenStream {
        let arg_len = self.rust_args_len();
        let args: Vec<TokenStream> = (0..arg_len)
            .map(|index| {
                let arg_name = Ident::new(&format!("arg{}", index), Span::call_site());
                quote! {
                    let #arg_name = self.#arg_name.try_to_js(js_env)?;
                }
            })
            .collect();

        quote! {

            #(#args)*

        }
    }

    pub fn as_constructor_invocation(&self) -> TokenStream {
        let arg_len = self.rust_args_len();
        let args: Vec<TokenStream> = (0..arg_len)
            .map(|index| {
                let arg_name = Ident::new(&format!("arg{}", index), Span::call_site());
                quote! {
                    #arg_name,
                }
            })
            .collect();

        quote! {

            #(#args)*

        }
    }


    fn receiver(&self) -> TokenStream {

        if self.is_method() {
            quote! {
                let receiver = (js_cb.unwrap_mut::<Self>()?);
            }
        } else {
            quote! {}
        }
    }

    /// convert myself as rust code which can convert JS arguments into rust.
    fn as_token_stream(&self, rust_fn_ident: &Ident,ctx: &mut FunctionContext) -> TokenStream {
        
        let rust_args_input = self.rust_args_input(ctx);

        if self.is_method() {
            quote! {
                receiver.#rust_fn_ident( #(#rust_args_input)* )
            }    
        } else {
            quote! {
                #rust_fn_ident( #(#rust_args_input)* )
            }
        }
        
    }
}

/// generate napi function invocation whether it is method or just free standing function
fn raw_napi_function_template(
    ident_n_api_fn: Ident,
    input_fn: TokenStream,
    rust_args_struct: &Vec<TokenStream>,
    rust_invocation: TokenStream,
) -> TokenStream {

    quote! {

        extern "C" fn #ident_n_api_fn(env: node_bindgen::sys::napi_env,cb_info: node_bindgen::sys::napi_callback_info) -> node_bindgen::sys::napi_value
        {
            use node_bindgen::core::TryIntoJs;
            use node_bindgen::core::IntoJs;
            use node_bindgen::core::val::JsCallbackFunction;

            #input_fn

            #(#rust_args_struct)*

            let js_env = node_bindgen::core::val::JsEnv::new(env);

            #rust_invocation
        }
    }
}

/// Categorize function argument
enum FunctionArg {
    Path(MyTypePath),            // normal type
    Ref(MyReferenceType),                          // reference type
    Closure(ClosureType),        // callback
    JSCallback(MyReferenceType), // indicating that we want to receive typed JsCallback
}

impl FunctionArg {
    /// given this, convert into normalized type signature
    fn new(ident: Ident, ty: Box<Type>, generics: GenerericInfo,multi_threaded: bool) -> Option<Self> {
        match *ty {
            Type::Path(path_type) => {
                let my_type = MyTypePath::new(path_type);

                // check whether type references in the generic indicates this is closure
                if let Some(param) = generics.find_generic(&my_type.type_name().unwrap()) {
                    if let Some(closure) = ClosureType::from(ident,param,multi_threaded) {
                        Some(Self::Closure(closure))
                    } else {
                        None
                    }
                } else {
                    Some(Self::Path(my_type))
                }
            }
            Type::Reference(ref_type) => {
                let my_type = MyReferenceType::new(ref_type);
                if my_type.is_callback() {
                    Some(Self::JSCallback(my_type))
                } else {
                    Some(Self::Ref(my_type))
                }
            }
            _ => None,
        }
    }

    /// generate as argument to rust function
    /// ex:   arg0: f64, arg1: String,
    fn as_rust_fn_arg(&self, index: usize) -> TokenStream {
        let name = Ident::new(&format!("arg{}", index), Span::call_site());
        match self {
            Self::Path(ty) => {
                let inner = ty.inner();
                quote! {
                    #name: #inner,
                }
            }
            Self::Closure(_) => quote! { compile_error!("closure can't be used in constructor ")},
            Self::JSCallback(_) => quote! { compile_error!("JsCallback can't be used in constructor")},
            Self::Ref(_) => quote! { compile_error!("ref can't be used in constructor")}
        }
    }

    fn as_struct_arg(&self, index: usize) -> TokenStream {
        let name = Ident::new(&format!("arg{}", index), Span::call_site());
        quote! {
            #name,
        }
    }

    /// generate code as part of invoking rust function
    /// for normal argument, it is just variable
    /// other may like closure may need to convert to rust closure
    /// for example,
    fn as_arg_token_stream(&self, index: &mut usize,ctx: &mut FunctionContext) -> TokenStream {
        match self {
            Self::Path(t) => {
                let output = t.as_arg_token_stream(*index);
                *index = *index + 1;
                output
            },
            Self::Ref(t) => {
                let output = t.as_arg_token_stream(*index);
                *index = *index + 1;
                output
            }
            Self::Closure(t) => {
                let arg_name = rust_arg_var(*index);
                let output = t.as_arg_token_stream(*index, &arg_name,ctx);
                *index = *index + 1;
                output
            }
            Self::JSCallback(_) => {
                let js_cb = Ident::new("js_cb", Span::call_site());
                quote! { &#js_cb, }
            }
        }
    }

    fn is_callback(&self) -> bool {
        match self {
            Self::JSCallback(_) => true,
            _ => false,
        }
    }

    /// generate expression to extract rust value from Js env
    /// example as below:
    ///     let r_arg1 = cb.get_value::<f64>(1)?;
    ///
    fn js_to_rust_token_stream(&self, arg_index: usize, ctx: &FunctionContext) -> TokenStream {

        // generate expression to convert napi value to rust value from callback
        fn rust_value(possible_type_name: Option<Ident>, index: usize) -> TokenStream {
            if let Some(type_name) = possible_type_name {
                let rust_value = rust_arg_var(index);
                let index_literal = index.to_string();
                let index_ident = LitInt::new(&index_literal, Span::call_site());
                quote! {
                    let #rust_value = js_cb.get_value::<#type_name>(#index_ident)?;
                }
            } else {
                quote! {
                    compile_error!("not supported type ")
                }
            }
        }

        /// generate expression to convert napi_value as rust ref
        fn rust_ref(possible_type_name: Option<Ident>, index: usize) -> TokenStream {
            if let Some(type_name) = possible_type_name {
                let rust_value = rust_arg_var(index);
                let index_literal = index.to_string();
                let index_ident = LitInt::new(&index_literal, Span::call_site());
                quote! {
                    let #rust_value = js_cb.get_ref::<#type_name>(#index_ident)?;
                }
            } else {
                quote! {
                    compile_error!("not supported type ")
                }
            }
        }

        match self {
            Self::Closure(ty) => {
                if ctx.is_async {
                    ty.generate_as_async_token_stream(arg_index,ctx)
                } else if ctx.is_multi_threaded {
                    ty.generate_as_async_token_stream(arg_index,ctx)
                } else {
                    rust_value(Some(ty.type_name()), arg_index)
                }
            }
            Self::Path(ty) => rust_value(ty.type_name(), arg_index),
            Self::Ref(ty) => rust_ref(ty.type_name(), arg_index),
            Self::JSCallback(_ty) => quote! { compile_error!("should not be converted from JS")},
        }
    }
}

struct GenerericInfo<'a> {
    generics: &'a Generics,
}

impl<'a> GenerericInfo<'a> {
    fn new(generics: &'a Generics) -> Self {
        Self { generics }
    }

    /// find generic with identifier
    fn find_generic(&self, ident: &Ident) -> Option<TypeParam> {
        for ty in self.generics.type_params() {
            if ty.ident.to_string() == ident.to_string() {
                return Some(ty.clone());
            }
        }

        None
    }
}


struct ClosureType {
    ty: ParenthesizedGenericArguments,
    ident: Ident,
    multi_threaded: bool
}

impl ClosureType {
    // try to see if we can find closure, otherwise return none
    fn from(ident: Ident,param: TypeParam,multi_threaded: bool) -> Option<Self> {
        for bound in param.bounds {
            match bound {
                TypeParamBound::Trait(tt) => {
                    for segment in tt.path.segments {
                        match segment.arguments {
                            PathArguments::Parenthesized(path) => return Some(Self {
                                ident,
                                ty: path,
                                multi_threaded
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

    fn type_name(&self) -> Ident {
        
        let callback_type = if self.multi_threaded {
            "JsMultiThreadedCallbackFunction"
        } else {
            "JsCallbackFunction"
        };

        Ident::new(callback_type, Span::call_site())
    }


    fn as_arg_token_stream(&self, arg_index: usize, closure_var: &Ident,ctx: &mut FunctionContext) -> TokenStream {

        let args: Vec<TokenStream> = self
            .ty
            .inputs
            .iter()
            .enumerate()
            .map(|(index, path)| {
                match path {
                    Type::Path(path_type) => {
                        let ty = MyTypePath::new(path_type.clone());
                        let arg_name = format!("cb_arg{}", index);
                        let var_name = Ident::new(&arg_name, Span::call_site());
                        let type_name = ty.type_name().unwrap();
                        quote! {
                            #var_name: #type_name,
                        }
                    }
                    _ => quote! {},
                }
            })
            .collect();

        let inner_closure = if ctx.is_async || ctx.is_multi_threaded {
            self.as_async_arg_token_stream(arg_index,closure_var,ctx) 
        } else {
            self.as_sync_arg_token_stream(arg_index,closure_var)
        };

        quote! {
            move | #(#args)* | {
 
                 #inner_closure
            }
 
         }

    }

    /// generate as argument to sync rust function or method
    /// since this is closure, we generate closure
    fn as_sync_arg_token_stream(&self, _i: usize, closure_var: &Ident) -> TokenStream {

        let js_conversions: Vec<TokenStream> = self
            .ty
            .inputs
            .iter()
            .enumerate()
            .map(|(index, _path)| {
                let arg_name = format!("cb_arg{}", index);
                let var_name = Ident::new(&arg_name, Span::call_site());
                quote! {
                    #var_name,
                }
            })
            .collect();

        quote! {

            // invoke sync closure
            let result = (|| {
                let args = vec![
                    #(#js_conversions)*
                ];
                #closure_var.call(args)
            })();

            result.to_js(&js_env);

        }
    }

    // name of function is used by thread safe function to complete closure
    fn async_js_callback_identifier(&self) -> Ident {
        Ident::new(&format!("thread_safe_{}_complete",self.ident),Span::call_site())
    }

    fn as_async_arg_token_stream(&self, _index: usize, closure_var: &Ident,ctx: &mut FunctionContext) -> TokenStream {

        let arg_struct_name = Ident::new(&format!("Arg{}",self.ident),Span::call_site());
        let arg_cb_complete = self.async_js_callback_identifier();
        let struct_fields: Vec<TokenStream> = self
            .ty
            .inputs
            .iter()
            .enumerate()
            .map(|(index, path)| {
                match path {
                    Type::Path(path_type) => {
                        let ty = MyTypePath::new(path_type.clone());
                        let var_name = Ident::new(&format!("arg{}", index), Span::call_site());
                        let type_name = ty.type_name().unwrap();
                        quote! {
                            #var_name: #type_name,
                        }
                    }
                    _ => quote! {},
                }
            })
            .collect();

            
        let js_complete_conversions: Vec<TokenStream> = self
            .ty
            .inputs
            .iter()
            .enumerate()
            .map(|(index, _path)| { 
                let js_var_iden = Ident::new(&format!("js_arg{}",index),Span::call_site());
                let arg_idn = Ident::new(&format!("arg{}",index),Span::call_site());
                quote !{
                    let #js_var_iden = my_val.#arg_idn.try_to_js(&js_env)?;
                }
            })
            .collect();

        let js_call: Vec<TokenStream> = self
            .ty
            .inputs
            .iter()
            .enumerate()
            .map(|(index, _path)| { 
                let js_var_iden = Ident::new(&format!("js_arg{}",index),Span::call_site());
                quote !{
                    #js_var_iden,
                }
            })
            .collect();

        ctx.cb_args.push(quote!{

            #[derive(Debug)]
            struct #arg_struct_name {
                #(#struct_fields)*
            }

            extern "C" fn #arg_cb_complete(
                env: node_bindgen::sys::napi_env,
                js_cb: node_bindgen::sys::napi_value, 
                _context: *mut ::std::os::raw::c_void,
                data: *mut ::std::os::raw::c_void) {
        
                if env != std::ptr::null_mut() {
        
                    node_bindgen::core::log::debug!("async cb invoked");
                    let js_env = node_bindgen::core::val::JsEnv::new(env);
        
                    let result: Result<(), node_bindgen::core::NjError> = (move ||{
        
                        let global = js_env.get_global()?;
                        let my_val: Box<#arg_struct_name> = unsafe { Box::from_raw(data as *mut #arg_struct_name) };
                        node_bindgen::core::log::trace!("arg: {:#?}",my_val);
                        #(#js_complete_conversions)*

                        node_bindgen::core::log::debug!("async cb, invoking js cb");
                        js_env.call_function(global,js_cb,vec![#(#js_call)*])?;
                        node_bindgen::core::log::trace!("async cb, done");
                        Ok(())
        
                    })();
                    
                    node_bindgen::core::assert_napi!(result)
            
                }
                
            }

        });
        
        

        let args: Vec<TokenStream> = self
            .ty
            .inputs
            .iter()
            .enumerate()
            .map(|(index, _path)| {
                let arg_name = Ident::new(&format!("arg{}",index),Span::call_site());
                let cb_name: Ident = Ident::new(&format!("cb_arg{}",index),Span::call_site());
                quote! {
                    #arg_name: #cb_name,
                }
            })
            .collect();

        quote!  {

            let arg = #arg_struct_name {
                #(#args)*
            };

            node_bindgen::core::log::trace!("converting rust to raw ptr");
            let my_box = Box::new(arg);
            let ptr = Box::into_raw(my_box);

            #closure_var.call(Some(ptr as *mut core::ffi::c_void)).expect("callback should work");

        }

    }

    /// generate thread safe function when callback are used in the async
    /// for example:
    ///     let r_arg1 = cb.create_thread_safe_function("hello_sf",0,Some(hello_callback_js))?;
    fn generate_as_async_token_stream(&self,index: usize,ctx: &FunctionContext) -> TokenStream {

        let sf_identifier = LitStr::new(&format!("{}_sf", ctx.name), Span::call_site());
        let rust_var_name = rust_arg_var(index);
        let js_cb_completion = self.async_js_callback_identifier();
        
        quote! {
            let #rust_var_name = js_cb.create_thread_safe_function(#sf_identifier,#index,Some(#js_cb_completion))?;
        }

    }
}