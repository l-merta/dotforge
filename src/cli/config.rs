//! `dotforge config ...` commands: DotForge's own settings.

use std::fs;

use clap::Subcommand;
use serde::Serialize;

use crate::editor;
use crate::error::AppError;
use crate::output::{Render, Report};
use crate::settings::{self, DEFAULT_EDITOR, TEMPLATE};

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
  /// Show the settings in use and where they come from
  Show,

  /// Open the settings file in your editor (it is created if missing)
  Edit,
}

pub fn handle(command: ConfigCommands) -> Result<Report, AppError> {
  match command {
    ConfigCommands::Show => show(),
    ConfigCommands::Edit => edit(),
  }
}

#[derive(Serialize)]
struct ConfigShow {
  file: String,
  file_exists: bool,
  settings: Vec<SettingRow>,
}

#[derive(Serialize)]
struct SettingRow {
  key: &'static str,
  value: String,
  /// `file` when set in config.yml, `default` otherwise.
  source: &'static str,
}

fn show() -> Result<Report, AppError> {
  let file = settings::settings_file()?;
  let loaded = settings::load_from(&file)?;

  let editor_source = if loaded
    .editor
    .as_deref()
    .is_some_and(|e| !e.trim().is_empty())
  {
    "file"
  } else {
    "default"
  };

  Ok(Report::new(&ConfigShow {
    file: file.display().to_string(),
    file_exists: file.exists(),
    settings: vec![SettingRow {
      key: "editor",
      value: loaded.editor().to_string(),
      source: editor_source,
    }],
  }))
}

impl Render for ConfigShow {
  fn render_text(&self) -> String {
    let mut text = if self.file_exists {
      format!("Settings file: {}\n\n", self.file)
    } else {
      format!(
        "Settings file: {} (not created yet, using defaults)\n\n",
        self.file
      )
    };

    for row in &self.settings {
      let note = if row.source == "default" {
        "  (default)"
      } else {
        ""
      };
      text.push_str(&format!("  {}: {}{note}\n", row.key, row.value));
    }

    if !self.file_exists {
      text.push_str("\nChange settings with: dotforge config edit\n");
    }

    text
  }
}

#[derive(Serialize)]
struct Edited {
  file: String,
  editor: String,
  created: bool,
}

fn edit() -> Result<Report, AppError> {
  let file = settings::settings_file()?;

  // Read the current settings first: if the file is broken we still open it
  // (with the default editor) so it can be fixed.
  let editor_command = settings::load_from(&file)
    .map(|loaded| loaded.editor().to_string())
    .unwrap_or_else(|_| DEFAULT_EDITOR.to_string());

  let created = !file.exists();
  if created {
    if let Some(parent) = file.parent() {
      fs::create_dir_all(parent).map_err(|source| AppError::Write {
        path: parent.to_path_buf(),
        source,
      })?;
    }
    fs::write(&file, TEMPLATE).map_err(|source| AppError::Write {
      path: file.clone(),
      source,
    })?;
  }

  editor::open(&editor_command, &file)?;

  // Check the result right away, so a typo is reported now and not later.
  settings::load_from(&file)?;

  Ok(Report::new(&Edited {
    file: file.display().to_string(),
    editor: editor_command,
    created,
  }))
}

impl Render for Edited {
  fn render_text(&self) -> String {
    let created = if self.created { " (new file)" } else { "" };
    format!("Saved {}{created}. The settings are valid.\n", self.file)
  }
}
