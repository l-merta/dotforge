//! Running commands while showing and logging their output.
//!
//! Output of every command is shown live (on stderr, so `--json` output on
//! stdout stays clean) and written to a log file in
//! `~/.local/state/dotforge/logs/`, so problems can be looked at later.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use super::command::CommandSpec;

/// What happened when a command ran.
#[derive(Debug, Clone, Default)]
pub struct RunOutcome {
  pub success: bool,
  pub exit_code: Option<i32>,
  /// Set when the command could not be started at all.
  pub error: Option<String>,
  /// The last few output lines, for the final report.
  pub tail: Vec<String>,
}

/// Something that can run commands. The real one starts processes; tests
/// use a fake that only records what it was asked to do.
pub trait Runner {
  fn run(&mut self, command: &CommandSpec) -> RunOutcome;
  /// Write a note to the log.
  fn note(&mut self, text: &str);
  fn log_path(&self) -> Option<&Path>;
}

pub struct RealRunner {
  log: Option<File>,
  log_path: Option<PathBuf>,
  /// Show command output on the terminal (stderr).
  echo: bool,
}

impl RealRunner {
  /// Opens a new log file in `<state_dir>/logs/`. If that fails, DotForge
  /// still works, just without a log file.
  pub fn new(state_dir: &Path, echo: bool) -> Self {
    let dir = state_dir.join("logs");
    let path = dir.join(format!("apply-{}.log", file_stamp()));
    let log = fs::create_dir_all(&dir)
      .ok()
      .and_then(|_| File::create(&path).ok());

    let log_path = log.as_ref().map(|_| path);
    let mut runner = Self {
      log,
      log_path,
      echo,
    };
    runner.note(&format!("DotForge apply started {}", readable_stamp()));
    runner
  }

  fn write_log(&mut self, line: &str) {
    if let Some(file) = &mut self.log {
      let _ = writeln!(file, "{line}");
    }
  }
}

impl Runner for RealRunner {
  fn run(&mut self, command: &CommandSpec) -> RunOutcome {
    let argv = command.argv();
    self.write_log(&format!("\n$ {command}"));
    if self.echo {
      eprintln!("\n→ {command}");
    }

    let mut child = Command::new(&argv[0]);
    if let Some(dir) = &command.cwd {
      child.current_dir(dir);
    }
    let child = child
      .args(&argv[1..])
      .stdin(Stdio::inherit())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn();

    let mut child = match child {
      Ok(child) => child,
      Err(error) => {
        let message = if error.kind() == std::io::ErrorKind::NotFound {
          format!("the program '{}' was not found", argv[0])
        } else {
          error.to_string()
        };
        self.write_log(&format!("! could not start: {message}"));
        return RunOutcome {
          error: Some(message),
          ..RunOutcome::default()
        };
      }
    };

    // Read stdout and stderr at the same time, line by line.
    let (sender, receiver) = mpsc::channel::<String>();
    let readers: Vec<_> = [
      child
        .stdout
        .take()
        .map(|s| Box::new(s) as Box<dyn Read + Send>),
      child
        .stderr
        .take()
        .map(|s| Box::new(s) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|stream| {
      let sender = sender.clone();
      thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
          if sender.send(line).is_err() {
            break;
          }
        }
      })
    })
    .collect();
    drop(sender);

    let mut tail: Vec<String> = Vec::new();
    for line in receiver {
      if self.echo {
        eprintln!("  {line}");
      }
      self.write_log(&line);
      tail.push(line);
      if tail.len() > 5 {
        tail.remove(0);
      }
    }
    for reader in readers {
      let _ = reader.join();
    }

    let status = child.wait();
    let (success, exit_code) = match &status {
      Ok(status) => (status.success(), status.code()),
      Err(_) => (false, None),
    };
    self.write_log(&format!(
      "= exit code {}",
      exit_code.map_or("none (stopped by a signal)".to_string(), |c| c.to_string())
    ));

    RunOutcome {
      success,
      exit_code,
      error: status.err().map(|e| e.to_string()),
      tail,
    }
  }

  fn note(&mut self, text: &str) {
    self.write_log(&format!("# {text}"));
  }

  fn log_path(&self) -> Option<&Path> {
    self.log_path.as_deref()
  }
}

/// `20261124-024159`, in UTC.
fn file_stamp() -> String {
  let (y, mo, d, h, mi, s) = utc_now();
  format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

/// `2026-11-24 02:41:59 UTC`
pub fn readable_stamp() -> String {
  let (y, mo, d, h, mi, s) = utc_now();
  format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}

fn utc_now() -> (i64, u32, u32, u32, u32, u32) {
  let secs = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0);
  let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
  let rem = secs.rem_euclid(86_400) as u32;
  (y, mo, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
  let z = z + 719_468;
  let era = z.div_euclid(146_097);
  let doe = z.rem_euclid(146_097);
  let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
  let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
  let mp = (5 * doy + 2) / 153;
  let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
  let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
  let y = yoe + era * 400 + i64::from(m <= 2);
  (y, m, d)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dates() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(20_416), (2025, 11, 24));
    assert_eq!(civil_from_days(11_016), (2000, 2, 29));
  }

  #[test]
  fn runs_and_logs_a_command() {
    let dir = tempfile::tempdir().unwrap();
    let mut runner = RealRunner::new(dir.path(), false);
    let ok = runner.run(&CommandSpec::user(
      "sh",
      ["-c"],
      &["echo hello; echo oops >&2".into()],
    ));
    assert!(ok.success);
    assert_eq!(ok.tail.len(), 2);

    let fail = runner.run(&CommandSpec::user("sh", ["-c"], &["exit 3".into()]));
    assert!(!fail.success);
    assert_eq!(fail.exit_code, Some(3));

    let missing = runner.run(&CommandSpec::user("no-such-program-xyz", [], &[]));
    assert!(missing.error.unwrap().contains("was not found"));

    let log = fs::read_to_string(runner.log_path().unwrap()).unwrap();
    assert!(log.contains("$ sh -c echo hello"));
    assert!(log.contains("hello"));
    assert!(log.contains("= exit code 3"));
  }
}
