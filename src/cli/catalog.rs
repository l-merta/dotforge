//! `dotforge catalog ...` commands.
//!
//! Each command builds a small result struct. The struct is turned into
//! JSON automatically (`Serialize`) and into text by its `Render` impl.

use clap::Subcommand;
use serde::Serialize;

use crate::catalog::catalog::Catalog;
use crate::catalog::model::{CatalogEntry, Category, Source};
use crate::catalog::validate;
use crate::error::AppError;
use crate::exit_code::ExitCode;
use crate::output::{Render, Report};
use crate::validation::{Issue, Severity, count, render_issues};

#[derive(Subcommand, Debug)]
pub enum CatalogCommands {
  /// List available catalog entries
  List {
    /// Only show entries of this category
    category: Option<Category>,
  },

  /// Show details about a catalog entry
  Show {
    /// Catalog entry ID, e.g. hyprland
    id: String,
  },

  /// Check every catalog entry for mistakes
  Validate,
}

pub fn handle(command: CatalogCommands) -> Result<Report, AppError> {
  let catalog = Catalog::load()?;

  match command {
    CatalogCommands::List { category } => Ok(Report::new(&list(&catalog, category))),
    CatalogCommands::Show { id } => show(&catalog, &id).map(|entry| Report::new(&entry)),
    CatalogCommands::Validate => {
      let result = validate_catalog(&catalog);
      let exit_code = if result.errors > 0 {
        ExitCode::Invalid
      } else {
        ExitCode::Success
      };
      Ok(Report::new(&result).with_exit_code(exit_code))
    }
  }
}

// ---------- list ----------

#[derive(Serialize)]
pub struct CatalogList {
  /// The category filter, if one was given.
  pub category: Option<Category>,
  pub total: usize,
  pub entries: Vec<ListItem>,
}

#[derive(Serialize)]
pub struct ListItem {
  pub id: String,
  pub name: String,
  pub category: Category,
  pub source: Source,
}

fn list(catalog: &Catalog, category: Option<Category>) -> CatalogList {
  let selected: Vec<&CatalogEntry> = match category {
    Some(category) => catalog.by_category(category),
    None => catalog.all().iter().collect(),
  };

  let entries: Vec<ListItem> = selected
    .into_iter()
    .map(|entry| ListItem {
      id: entry.id.clone(),
      name: entry.name.clone(),
      category: entry.category,
      source: entry.source.clone(),
    })
    .collect();

  CatalogList {
    category,
    total: entries.len(),
    entries,
  }
}

impl Render for CatalogList {
  fn render_text(&self) -> String {
    let mut text = String::new();
    let categories: Vec<Category> = match self.category {
      Some(category) => vec![category],
      None => Category::ALL.to_vec(),
    };

    text.push_str(&format!("Catalog entries: {}\n", self.total));

    for category in categories {
      text.push_str(&format!("\n{}:\n", category.title()));

      let items: Vec<&ListItem> = self
        .entries
        .iter()
        .filter(|item| item.category == category)
        .collect();

      if items.is_empty() {
        text.push_str("  (none yet)\n");
      }

      for item in items {
        let marker = match item.source.kind {
          crate::catalog::model::SourceKind::User => "  [user]",
          crate::catalog::model::SourceKind::Builtin => "",
        };
        text.push_str(&format!("  {} - {}{marker}\n", item.id, item.name));
      }
    }

    text
  }
}

// ---------- show ----------

fn show(catalog: &Catalog, id: &str) -> Result<CatalogEntry, AppError> {
  match catalog.get(id) {
    Some(entry) => Ok(entry.clone()),
    None => {
      let similar = catalog.similar_ids(id);
      let hint = if similar.is_empty() {
        "run 'dotforge catalog list' to see every entry".to_string()
      } else {
        format!("did you mean '{}'?", similar.join("', '"))
      };

      Err(AppError::NotFound {
        what: "catalog entry",
        name: id.to_string(),
        hint: Some(hint),
      })
    }
  }
}

impl Render for CatalogEntry {
  fn render_text(&self) -> String {
    let mut text = String::new();

    text.push_str(&format!("{}\n", self.name));
    text.push_str("────────────────────────────────\n\n");

    section(&mut text, "ID", [self.id.clone()]);
    section(&mut text, "Category", [self.category.to_string()]);
    section(&mut text, "Description", [self.description.clone()]);
    section(&mut text, "Source", [self.source.to_string()]);
    section(
      &mut text,
      "Platforms",
      self.platforms.iter().map(ToString::to_string),
    );
    section(
      &mut text,
      "Protocols",
      self.protocols.iter().map(ToString::to_string),
    );
    section(
      &mut text,
      "Capabilities",
      self.capabilities.iter().map(ToString::to_string),
    );
    section(&mut text, "Dependencies", self.dependencies.iter().cloned());

    text.push_str("Installation:\n");
    if self.installation.package_managers.is_empty() {
      text.push_str("  (none)\n");
    }
    for (family, installation) in &self.installation.package_managers {
      text.push_str(&format!("  {family}:\n"));
      for package in &installation.packages {
        text.push_str(&format!("    {package}\n"));
      }
    }

    if let Some(configuration) = &self.configuration {
      text.push('\n');
      section(
        &mut text,
        "Configuration",
        [configuration.directory.clone()],
      );
    }

    text
  }
}

fn section(text: &mut String, title: &str, lines: impl IntoIterator<Item = String>) {
  text.push_str(&format!("{title}:\n"));

  let mut empty = true;
  for line in lines {
    text.push_str(&format!("  {line}\n"));
    empty = false;
  }

  if empty {
    text.push_str("  (none)\n");
  }

  text.push('\n');
}

// ---------- validate ----------

#[derive(Serialize)]
pub struct CatalogValidation {
  pub valid: bool,
  pub entries: usize,
  pub errors: usize,
  pub warnings: usize,
  /// Built-in IDs that a user entry replaces.
  pub overridden: Vec<String>,
  pub issues: Vec<Issue>,
}

fn validate_catalog(catalog: &Catalog) -> CatalogValidation {
  let issues = validate::validate(catalog);
  let errors = count(&issues, Severity::Error);

  CatalogValidation {
    valid: errors == 0,
    entries: catalog.all().len(),
    errors,
    warnings: count(&issues, Severity::Warning),
    overridden: catalog.overridden().to_vec(),
    issues,
  }
}

impl Render for CatalogValidation {
  fn render_text(&self) -> String {
    let mut text = format!(
      "Checked {} catalog entries: {} error(s), {} warning(s)\n",
      self.entries, self.errors, self.warnings
    );

    for id in &self.overridden {
      text.push_str(&format!(
        "  note: your entry '{id}' replaces the built-in one\n"
      ));
    }

    text.push_str(&render_issues(&self.issues));

    if self.valid {
      text.push_str("The catalog is valid.\n");
    }

    text
  }
}
