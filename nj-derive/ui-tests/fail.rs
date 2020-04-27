use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
 
#[node_bindgen(name2="hello")]
fn hello(count: i32) -> String {        
    format!("hello world {}", count)
}

fn main() {
    
}