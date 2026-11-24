//! Looking at the machine DotForge runs on: which distribution it is, what
//! is installed, and how it compares with a profile. Nothing here changes
//! the system; it only reads.

pub mod backend;
pub mod compare;
pub mod host;
pub mod os_release;
pub mod pacman;
