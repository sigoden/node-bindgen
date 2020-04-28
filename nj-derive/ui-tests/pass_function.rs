use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
 
/// do nothing
#[node_bindgen]
fn example1() {        
}


/// single argument with result
#[node_bindgen]
fn example2(arg1: bool) -> i32 {        
    4
}

/// multiple arguments
#[node_bindgen]
fn example3(arg1: bool,arg2: i32,arg3: String) -> i32 {        
    4
}

/// with callback
#[node_bindgen]
fn example4<F: Fn(i32)>(cb: F,second: i32) {        
    cb(second)
}


/// async callback
#[node_bindgen]
async fn example5<F: Fn(f64,String)>( seconds: i32, cb: F) {
}

/// as promise
#[node_bindgen]
async fn example6(arg: f64) -> f64 {
    0.0
}


fn main() {
    
}