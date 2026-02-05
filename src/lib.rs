//! # portr
//!
//! Lightning-fast port inspector and process killer.
//! See what's using any port and kill it instantly.
//!
//! ## Usage
//!
//! ```bash
//! portr              # List all listening ports
//! portr 3000         # Inspect port 3000
//! portr 3000 --kill  # Kill process on port 3000
//! portr 3000-3010    # Scan port range
//! ```

pub mod config;
pub mod display;
#[cfg(feature = "docker")]
pub mod docker;
pub mod error;
pub mod export;
pub mod interactive;
pub mod port;
pub mod process;
pub mod services;
pub mod tui;

pub use config::*;
pub use display::*;
#[cfg(feature = "docker")]
pub use docker::*;
pub use error::*;
pub use export::*;
pub use interactive::*;
pub use port::*;
pub use process::*;
pub use services::*;
