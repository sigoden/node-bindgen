use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
 

#[node_bindgen]
fn hello(arg1: i32) -> i32 {        
    arg1*2
}



fn main() {
    
}