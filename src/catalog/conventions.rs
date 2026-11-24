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

/// Arch package names: lowercase letters, digits and `@ . _ + -`,
/// not starting with `-` or `.`.
pub fn is_valid_package(name: &str) -> bool {
  !name.is_empty()
    && !name.starts_with('-')
    && !name.starts_with('.')
    && name
      .chars()
      .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "@._+-".contains(c))
}

/// Linux group names: start with a lowercase letter or `_`, then lowercase
/// letters, digits, `_` or `-`.
pub fn is_valid_group(name: &str) -> bool {
  let mut chars = name.chars();
  matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
    && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn package_and_group_names() {
    assert!(is_valid_package("libreoffice-fresh"));
    assert!(is_valid_package("gtk+3"));
    assert!(!is_valid_package("Steam"));
    assert!(!is_valid_package("rm -rf"));
    assert!(is_valid_group("docker"));
    assert!(is_valid_group("_flatpak"));
    assert!(!is_valid_group("Bad Group"));
    assert!(!is_valid_group("1abc"));
  }

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
