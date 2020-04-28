use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;


#[node_bindgen(gibberish)]
fn example3(count: i32) -> i32 {        
    count
}

fn main() {
    
}