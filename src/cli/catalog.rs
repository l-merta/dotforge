use std::path::Path;

use clap::Subcommand;

use crate::catalog::catalog::Catalog;
use crate::catalog::model::Category;

#[derive(Subcommand, Debug)]
pub enum CatalogCommands {
  /// List available catalog entries
  List {
    /// Optional category filter
    category: Option<Category>,
  },

  /// Show details about a catalog entry
  Show {
    /// Catalog entry ID
    id: String,
  },
}

pub fn handle(command: CatalogCommands, catalog_path: &Path) {
  match command {
    CatalogCommands::List { category } => {
      list(catalog_path, category);
    }

    CatalogCommands::Show { id } => {
      show(catalog_path, &id);
    }
  }
}

fn list(catalog_path: &Path, category: Option<Category>) {
  let catalog = match Catalog::load(catalog_path) {
    Ok(catalog) => catalog,

    Err(error) => {
      eprintln!("Failed to load catalog: {error}");
      return;
    }
  };

  match category {
    Some(category) => {
      println!("Catalog entries in {:?}:", category);
      println!();

      for entry in catalog.by_category(&category) {
        println!("  {} - {}", entry.id, entry.name);
      }
    }

    None => {
      println!("Catalog loaded!");
      println!("Total entries: {}", catalog.all().len());

      print_category(&catalog, Category::Desktop, "Desktop environments");
      print_category(&catalog, Category::Compositor, "Compositors");
      print_category(&catalog, Category::Shell, "Shells");
      print_category(&catalog, Category::Component, "Components");
      print_category(&catalog, Category::Application, "Applications");
    }
  }
}

fn show(catalog_path: &Path, id: &str) {
  let catalog = match Catalog::load(catalog_path) {
    Ok(catalog) => catalog,

    Err(error) => {
      eprintln!("Failed to load catalog: {error}");
      return;
    }
  };

  let Some(entry) = catalog.get(id) else {
    eprintln!("Catalog entry '{id}' not found.");
    return;
  };

  println!("{}", entry.name);
  println!("────────────────────────────────");
  println!();
  println!("ID:");
  println!("  {}", entry.id);
  println!();
  println!("Category:");
  println!("  {:?}", entry.category);
  println!();
  println!("Description:");
  println!("  {}", entry.description);
}

fn print_category(catalog: &Catalog, category: Category, title: &str) {
  println!();
  println!("{title}:");

  for entry in catalog.by_category(&category) {
    println!("  {} - {}", entry.id, entry.name);
  }
}