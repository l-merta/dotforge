//! Separates *what a command does* from *how its result is printed*.
//!
//! Every command returns a [`Report`]: structured data plus a readable
//! text version. `main` then prints either the text or, with `--json`,
//! the JSON version. New commands get JSON output for free this way.

use serde::Serialize;
use serde_json::json;

use crate::error::{AppError, to_json_value};
use crate::exit_code::ExitCode;

/// Anything a command returns must be printable as text for humans.
/// (JSON comes from `Serialize`.)
pub trait Render {
  fn render_text(&self) -> String;
}

/// The finished result of a command.
pub struct Report {
  pub data: serde_json::Value,
  pub text: String,
  pub exit_code: ExitCode,
}

impl Report {
  pub fn new<T: Serialize + Render>(data: &T) -> Self {
    Self {
      data: to_json_value(data),
      text: data.render_text(),
      exit_code: ExitCode::Success,
    }
  }

  /// Use a different exit code, for example when validation found problems.
  pub fn with_exit_code(mut self, exit_code: ExitCode) -> Self {
    self.exit_code = exit_code;
    self
  }
}

/// Prints a command result and returns the exit code to use.
pub fn print_result(result: Result<Report, AppError>, json_mode: bool) -> ExitCode {
  match result {
    Ok(report) => {
      if json_mode {
        let output = json!({
          "ok": report.exit_code == ExitCode::Success,
          "exit_code": report.exit_code.code(),
          "data": report.data,
        });
        println!("{}", pretty(&output));
      } else {
        print!("{}", ensure_newline(&report.text));
      }

      report.exit_code
    }

    Err(error) => {
      let exit_code = error.exit_code();

      if json_mode {
        // In JSON mode everything goes to stdout, so scripts only need to
        // read one stream.
        let output = json!({
          "ok": false,
          "exit_code": exit_code.code(),
          "error": error.to_json(),
        });
        println!("{}", pretty(&output));
      } else {
        eprintln!("error: {error}");

        if let Some(hint) = error.hint() {
          eprintln!("hint: {hint}");
        }
      }

      exit_code
    }
  }
}

fn pretty(value: &serde_json::Value) -> String {
  serde_json::to_string_pretty(value).expect("JSON values always serialize")
}

fn ensure_newline(text: &str) -> String {
  if text.ends_with('\n') {
    text.to_string()
  } else {
    format!("{text}\n")
  }
}
