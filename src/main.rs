use std::env;

fn main() {
  let args: Vec<String> = env::args().collect();

  println!("Arguments:");

  for argument in args {
    println!("  {argument}");
  }
}