//! Exit codes used by every DotForge command.
//!
//! Scripts (and AI assistants) can check the exit code of `dotforge`
//! to find out what happened without reading the text output.
//! These numbers are part of the public interface: do not change them.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitCode {
  /// Everything worked.
  Success,
  /// Something went wrong that does not fit another code
  /// (for example a file could not be read).
  Failure,
  /// The command line itself was wrong (unknown command, missing argument).
  /// This code is produced by the argument parser (clap), which exits by
  /// itself, so DotForge never builds this value directly.
  #[allow(dead_code)]
  Usage,
  /// The requested profile or catalog entry does not exist.
  NotFound,
  /// A profile or catalog file exists but is invalid
  /// (broken YAML, missing fields, unknown catalog IDs, ...).
  Invalid,
  /// The command exists but is not implemented yet.
  NotImplemented,
  /// Something with that name already exists (for example a profile).
  AlreadyExists,
  /// You answered "no" when DotForge asked for confirmation.
  Cancelled,
}

impl ExitCode {
  pub fn code(self) -> u8 {
    match self {
      ExitCode::Success => 0,
      ExitCode::Failure => 1,
      ExitCode::Usage => 2,
      ExitCode::NotFound => 3,
      ExitCode::Invalid => 4,
      ExitCode::NotImplemented => 5,
      ExitCode::AlreadyExists => 6,
      ExitCode::Cancelled => 7,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn codes_are_stable() {
    assert_eq!(ExitCode::Success.code(), 0);
    assert_eq!(ExitCode::Failure.code(), 1);
    assert_eq!(ExitCode::Usage.code(), 2);
    assert_eq!(ExitCode::NotFound.code(), 3);
    assert_eq!(ExitCode::Invalid.code(), 4);
    assert_eq!(ExitCode::NotImplemented.code(), 5);
    assert_eq!(ExitCode::AlreadyExists.code(), 6);
    assert_eq!(ExitCode::Cancelled.code(), 7);
  }
}
