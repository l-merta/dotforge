//! Naming rules for catalog entries. See `docs/catalog-format.md`.

/// A valid ID is lowercase letters and digits, optionally split by single
/// dashes: `kitty`, `kde-plasma`, `xdg-desktop-portal-hyprland`.
pub fn is_valid_id(id: &str) -> bool {
  !id.is_empty()
    && !id.starts_with('-')
    && !id.ends_with('-')
    && !id.contains("--")
    && id
      .chars()
      .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid_ids() {
    for id in [
      "kitty",
      "kde-plasma",
      "xdg-desktop-portal-hyprland",
      "btop2",
    ] {
      assert!(is_valid_id(id), "{id} should be valid");
    }
  }

  #[test]
  fn invalid_ids() {
    for id in [
      "",
      "Kitty",
      "kde_plasma",
      "-kitty",
      "kitty-",
      "kde--plasma",
      "kitty term",
    ] {
      assert!(!is_valid_id(id), "{id:?} should be invalid");
    }
  }
}
