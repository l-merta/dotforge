//! Opening a file in the user's editor.

use std::path::Path;
use std::process::Command;

use crate::error::AppError;

/// Opens `file` with `editor` and waits until the editor is closed.
///
/// `editor` may contain arguments separated by spaces, e.g. `code --wait`.
/// The file path is added as the last argument. (Quotes are not
/// understood; use a small wrapper script for anything fancier.)
pub fn open(editor: &str, file: &Path) -> Result<(), AppError> {
  let mut parts = editor.split_whitespace();
  let Some(program) = parts.next() else {
    return Err(editor_error(editor, "the editor setting is empty".into()));
  };

  let status = Command::new(program)
    .args(parts)
    .arg(file)
    .status()
    .map_err(|error| {
      let reason = if error.kind() == std::io::ErrorKind::NotFound {
        format!("the program '{program}' was not found")
      } else {
        error.to_string()
      };
      editor_error(editor, reason)
    })?;

  if !status.success() {
    let reason = match status.code() {
      Some(code) => format!("it exited with code {code}"),
      None => "it was stopped by a signal".into(),
    };
    return Err(editor_error(editor, reason));
  }

  Ok(())
}

fn editor_error(editor: &str, reason: String) -> AppError {
  AppError::Editor {
    editor: editor.to_string(),
    reason,
  }
}
