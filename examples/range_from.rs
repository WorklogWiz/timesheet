
fn main() {
    let v = get_data(5);
    println!("Vector len: {}", v.len());
    for (n,i) in v.iter().enumerate() {
        println!("{n} : {i}");
    }
}

fn get_data(i: usize) -> Vec<usize>{

    ( 3 * i..5 * (i+1)).collect()
}