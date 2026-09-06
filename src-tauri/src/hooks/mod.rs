//! Claude Code hook integration: install a tiny receiver script, watch the
//! spool directory it writes into, and turn events into notifications.

mod fsutil;
pub mod installer;
pub mod notify;
pub mod watcher;
