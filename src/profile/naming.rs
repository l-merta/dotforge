//! Rules for profile names.
//!
//! A profile's *name* is the name of its folder in
//! `~/.config/dotforge/profiles/`. It is what you type in commands such as
//! `dotforge profile show desktop`. The `name:` field inside `profile.yml`
//! is a separate, free-text display name.

use crate::catalog::conventions::is_valid_id;

pub const MAX_LENGTH: usize = 64;

/// Profile names follow the same rules as catalog IDs: lowercase letters,
/// digits and single dashes, for example `desktop` or `work-laptop`.
pub fn is_valid(name: &str) -> bool {
  name.len() <= MAX_LENGTH && is_valid_id(name)
}

/// Explains the rules, for error messages.
pub fn rules() -> String {
  format!(
    "profile names may only use lowercase letters, digits and single dashes, up to {MAX_LENGTH} characters (for example 'desktop' or 'work-laptop')"
  )
}

/// Suggests a valid name, e.g. `My Laptop` → `my-laptop`.
pub fn suggest(name: &str) -> Option<String> {
  let mut result = String::new();

  for c in name.trim().chars() {
    if c.is_ascii_alphanumeric() {
      result.push(c.to_ascii_lowercase());
    } else if !result.ends_with('-') && !result.is_empty() {
      result.push('-');
    }
  }

  let result: String = result
    .trim_end_matches('-')
    .chars()
    .take(MAX_LENGTH)
    .collect();
  let result = result.trim_end_matches('-').to_string();

  if is_valid(&result) && result != name {
    Some(result)
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid_and_invalid_names() {
    assert!(is_valid("desktop"));
    assert!(is_valid("work-laptop-2"));
    assert!(!is_valid("Desktop"));
    assert!(!is_valid("my laptop"));
    assert!(!is_valid("../evil"));
    assert!(!is_valid(&"a".repeat(65)));
  }

  #[test]
  fn suggestions() {
    assert_eq!(suggest("My Laptop").as_deref(), Some("my-laptop"));
    assert_eq!(suggest("work_pc!").as_deref(), Some("work-pc"));
    assert_eq!(suggest("desktop"), None);
    assert_eq!(suggest("!!!"), None);
  }
}
