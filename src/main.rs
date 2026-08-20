use std::path::Path;

mod catalog;

fn main() {
  let path = Path::new("catalog/compositors/hyprland.yml");

  match catalog::loader::load_entry(path) {
    Ok(entry) => {
      println!("Loaded: {}", entry.name);
      println!("ID: {}", entry.id);
      println!("Description: {}", entry.description);
      println!("Category: {:?}", entry.category);
      println!("Protocols: {:?}", entry.protocols);
      println!("Platforms: {:?}", entry.platforms);
      println!("Dependencies: {:?}", entry.dependencies);

      println!("Packages:");

      for (manager, installation) in &entry.installation.package_managers {
        println!("  {manager}:");

        for package in &installation.packages {
          println!("    - {package}");
        }
      }

      if let Some(configuration) = &entry.configuration {
        println!("Configuration: {}", configuration.directory);
      }

      println!("Capabilities: {:?}", entry.capabilities);

      println!("Integration:");
      println!("  Themes: {}", entry.integration.themes);
      println!("  Wallpapers: {}", entry.integration.wallpapers);
      println!("  Keybindings: {}", entry.integration.keybindings);
    }

    Err(error) => {
      eprintln!("Failed to load catalog entry: {error}");
    }
  }
}