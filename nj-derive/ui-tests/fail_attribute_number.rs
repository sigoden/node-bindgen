use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;

/// name must be string
#[node_bindgen(name=20)]
fn example2(count: i32) -> i32 {        
    count
}



fn main() {
    
}