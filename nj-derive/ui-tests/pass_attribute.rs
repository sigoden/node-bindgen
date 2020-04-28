use node_bindgen::derive::node_bindgen;
use node_bindgen::core::NjError;
 
#[node_bindgen(name="hello2")]
fn example1(count: i32) -> i32 {        
    count
}

#[node_bindgen(constructor)]
fn example2(count: i32) -> i32 {        
    count
}

#[node_bindgen(getter)]
fn example3(count: i32) -> i32 {        
    count
}

#[node_bindgen(setter)]
fn example4(count: i32) -> i32 {        
    count
}

#[node_bindgen(mt)]
fn example5(count: i32) -> i32 {        
    count
}

fn main() {
    
}