use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
 
#[node_bindgen(name="hello2")]
fn hello(count: i32) -> i32 {        
    count
}