//! Asking the user questions. Questions go to stderr, answers come from
//! stdin. If there is no answer (stdin closed), the safe choice is used.

use std::io::{BufRead, Write};

use serde::Serialize;

/// What to do with an entry that was removed from the profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DroppedChoice {
  /// Uninstall its packages.
  Remove,
  /// Leave it installed and stop managing it.
  Ignore,
  /// Do nothing now and ask again next time (the safe default).
  Keep,
}

pub trait Prompter {
  /// A yes/no question. Default: no.
  fn confirm(&mut self, question: &str) -> bool;
  /// The remove / ignore / keep question about a dropped entry.
  fn dropped(&mut self, question: &str) -> DroppedChoice;
}

pub struct StdinPrompter;

impl StdinPrompter {
  fn ask(&mut self, question: &str) -> String {
    eprint!("{question} ");
    let _ = std::io::stderr().flush();

    let mut answer = String::new();
    match std::io::stdin().lock().read_line(&mut answer) {
      Ok(0) | Err(_) => {
        eprintln!("(no answer)");
        String::new()
      }
      Ok(_) => answer.trim().to_lowercase(),
    }
  }
}

impl Prompter for StdinPrompter {
  fn confirm(&mut self, question: &str) -> bool {
    matches!(self.ask(&format!("{question} [y/N]")).as_str(), "y" | "yes")
  }

  fn dropped(&mut self, question: &str) -> DroppedChoice {
    loop {
      match self
        .ask(&format!(
          "{question} [r]emove / [i]gnore / [k]eep for now (default)"
        ))
        .as_str()
      {
        "r" | "remove" => return DroppedChoice::Remove,
        "i" | "ignore" => return DroppedChoice::Ignore,
        "" | "k" | "keep" => return DroppedChoice::Keep,
        _ => eprintln!("Please answer r, i or k."),
      }
    }
  }
}

/// Answers every question with the safe choice: used with `--yes`, which
/// confirms the plan but never uninstalls anything.
pub struct YesPrompter;

impl Prompter for YesPrompter {
  fn confirm(&mut self, _question: &str) -> bool {
    true
  }

  fn dropped(&mut self, _question: &str) -> DroppedChoice {
    DroppedChoice::Keep
  }
}
