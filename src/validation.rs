//! Shared types for reporting validation problems.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
  /// Must be fixed: the file cannot be used as it is.
  Error,
  /// Probably a mistake, but DotForge can still work with it.
  Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
  pub severity: Severity,
  /// Where the problem is, e.g. `applications.gaming[1]` or a catalog ID.
  pub location: String,
  pub message: String,
}

impl Issue {
  pub fn error(location: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      severity: Severity::Error,
      location: location.into(),
      message: message.into(),
    }
  }

  pub fn warning(location: impl Into<String>, message: impl Into<String>) -> Self {
    Self {
      severity: Severity::Warning,
      location: location.into(),
      message: message.into(),
    }
  }
}

pub fn count(issues: &[Issue], severity: Severity) -> usize {
  issues
    .iter()
    .filter(|issue| issue.severity == severity)
    .count()
}

/// Formats a list of issues for the terminal.
pub fn render_issues(issues: &[Issue]) -> String {
  let mut text = String::new();

  for issue in issues {
    let label = match issue.severity {
      Severity::Error => "error",
      Severity::Warning => "warning",
    };
    text.push_str(&format!(
      "  {label}: {}: {}\n",
      issue.location, issue.message
    ));
  }

  text
}
