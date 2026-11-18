//! The error type shared by all DotForge commands.
//!
//! Every error knows which exit code it maps to and can be shown either
//! as readable text or as JSON.

use std::path::PathBuf;

use serde::Serialize;
use serde_json::json;

use crate::exit_code::ExitCode;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
  /// A profile or catalog entry with this name/ID does not exist.
  #[error("{what} '{name}' was not found")]
  NotFound {
    what: &'static str,
    name: String,
    hint: Option<String>,
  },

  /// A file could not be read from disk.
  #[error("could not read '{}': {source}", path.display())]
  Io {
    path: PathBuf,
    source: std::io::Error,
  },

  /// A YAML file could not be understood.
  /// `field` is the path to the field that caused the problem
  /// (for example `device.os.family`) when it is known.
  #[error("{}", format_parse_error(file, field.as_deref(), *line, *column, message))]
  Parse {
    file: String,
    field: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
    message: String,
  },

  /// The data was read correctly but breaks a rule.
  #[error("{message}")]
  Invalid {
    message: String,
    hint: Option<String>,
  },

  /// DotForge could not work out where the user's config folder is.
  #[error("could not find your home folder (neither $HOME nor $XDG_CONFIG_HOME is set)")]
  NoConfigDir,

  /// The command is planned but does not exist yet.
  #[error("'{command}' is not implemented yet (planned for {milestone})")]
  NotImplemented {
    command: &'static str,
    milestone: &'static str,
  },
}

fn format_parse_error(
  file: &str,
  field: Option<&str>,
  line: Option<usize>,
  column: Option<usize>,
  message: &str,
) -> String {
  let mut location = file.to_string();

  if let Some(line) = line {
    location.push_str(&format!(":{line}"));

    if let Some(column) = column {
      location.push_str(&format!(":{column}"));
    }
  }

  match field {
    Some(field) => format!("{location}: problem in field '{field}': {message}"),
    None => format!("{location}: {message}"),
  }
}

impl AppError {
  pub fn exit_code(&self) -> ExitCode {
    match self {
      AppError::NotFound { .. } => ExitCode::NotFound,
      AppError::Io { .. } | AppError::NoConfigDir => ExitCode::Failure,
      AppError::Parse { .. } | AppError::Invalid { .. } => ExitCode::Invalid,
      AppError::NotImplemented { .. } => ExitCode::NotImplemented,
    }
  }

  /// A short, stable name for the kind of error, used in JSON output.
  pub fn kind(&self) -> &'static str {
    match self {
      AppError::NotFound { .. } => "not_found",
      AppError::Io { .. } => "io",
      AppError::Parse { .. } => "parse",
      AppError::Invalid { .. } => "invalid",
      AppError::NoConfigDir => "no_config_dir",
      AppError::NotImplemented { .. } => "not_implemented",
    }
  }

  /// An optional tip telling the user what they can do next.
  pub fn hint(&self) -> Option<&str> {
    match self {
      AppError::NotFound { hint, .. } | AppError::Invalid { hint, .. } => hint.as_deref(),
      _ => None,
    }
  }

  pub fn to_json(&self) -> serde_json::Value {
    let mut error = json!({
      "kind": self.kind(),
      "message": self.to_string(),
    });

    if let Some(hint) = self.hint() {
      error["hint"] = json!(hint);
    }

    if let AppError::Parse {
      file,
      field,
      line,
      column,
      message,
    } = self
    {
      error["file"] = json!(file);
      error["field"] = json!(field);
      error["line"] = json!(line);
      error["column"] = json!(column);
      error["reason"] = json!(message);
    }

    error
  }
}

/// Turns a YAML/serde error into an `AppError::Parse` that names the file,
/// the line and the field that caused it.
pub fn parse_error(file: &str, error: serde_path_to_error::Error<serde_yaml::Error>) -> AppError {
  let path = error.path().to_string();
  let field = if path == "." || path.is_empty() {
    None
  } else {
    Some(path)
  };

  let inner = error.into_inner();
  let location = inner.location();
  let mut message = strip_location(&inner.to_string());

  // serde_yaml sometimes starts the message with the field path too.
  if let Some(field) = &field
    && let Some(rest) = message.strip_prefix(&format!("{field}: "))
  {
    message = rest.to_string();
  }

  AppError::Parse {
    file: file.to_string(),
    field,
    line: location.as_ref().map(|location| location.line()),
    column: location.as_ref().map(|location| location.column()),
    message,
  }
}

/// serde_yaml adds "at line X column Y" to its messages. We already show the
/// location in a nicer way, so remove it from the message.
fn strip_location(message: &str) -> String {
  match message.find(" at line ") {
    Some(index) => message[..index].to_string(),
    None => message.to_string(),
  }
}

/// Small helper so `Serialize` types can be turned into JSON without
/// repeating `.expect()` everywhere.
pub fn to_json_value<T: Serialize>(value: &T) -> serde_json::Value {
  serde_json::to_value(value).expect("DotForge output types always serialize to JSON")
}
