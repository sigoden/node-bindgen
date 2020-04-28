use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
 
#[node_bindgen(name="hello2")]
fn hello(count: i32) -> i32 {        
    count
}

#[node_bindgen(constructor)]
fn hello(count: i32) -> i32 {        
    count
}

#[node_bindgen(getter)]
fn hello(count: i32) -> String {        
    count
}

#[node_bindgen(setter)]
fn hello(count: i32) -> String {        
    count
}

#[node_bindgen(mt)]
fn hello(count: i32) -> String {        
    count
}

fn main() {
    
}