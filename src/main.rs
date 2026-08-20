use std::path::Path;

use catalog::model::Category;

mod catalog;

fn main() {
  let path = Path::new("catalog");

  match catalog::catalog::Catalog::load(path) {
    Ok(catalog) => {
      println!("Catalog loaded!");
      println!("Total entries: {}", catalog.all().len());

      println!();
      println!("Compositors:");

      for entry in catalog.by_category(&Category::Compositor) {
        println!("  {} - {}", entry.id, entry.name);
      }

      println!();
      println!("Shells:");

      for entry in catalog.by_category(&Category::Shell) {
        println!("  {} - {}", entry.id, entry.name);
      }
    }

    Err(error) => {
      eprintln!("Failed to load catalog: {error}");
    }
  }
}