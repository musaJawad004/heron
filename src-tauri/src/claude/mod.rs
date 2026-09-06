//! Read-only access to the data the Claude Code CLI writes under `~/.claude`.
//! See docs/ARCHITECTURE.md for the observed formats.

pub mod jsonl;
pub mod paths;
pub mod process;
pub mod registry;
pub mod title;
pub mod transcripts;

/// Read-only look at the real `~/.claude`. Run with
/// `cargo test live_dump -- --ignored --nocapture`. Heron's cache goes to a
/// temp dir; nothing under `~/.claude` is written.
#[cfg(test)]
mod live {
    use super::paths::ClaudePaths;
    use super::registry::Registry;
    use super::transcripts::TranscriptIndex;
    use std::time::Instant;

    fn short(id: &str) -> String {
        id.chars().take(8).collect()
    }

    #[test]
    #[ignore]
    fn live_dump() {
        let paths = ClaudePaths::detect();
        println!("claude root: {}", paths.root.display());

        let registry = Registry::new(paths.sessions_dir());
        let t = Instant::now();
        let cold = registry.snapshot();
        let cold_t = t.elapsed();
        let t = Instant::now();
        let warm = registry.snapshot();
        let warm_t = t.elapsed();
        println!("registry: {} live session(s); snapshot cold {cold_t:?}, warm {warm_t:?}", warm.len());
        for s in &warm {
            println!(
                "  {} pid={:<6} status={:<11} tty={:<8} name={:?} cwd={} started={:?} last_active={} v={:?}",
                short(&s.id),
                s.pid.unwrap_or(0),
                format!("{:?}", s.status),
                s.tty.clone().unwrap_or_else(|| "-".into()),
                s.name,
                s.cwd,
                s.started_at,
                s.last_active_at,
                s.version
            );
        }
        assert_eq!(cold.len(), warm.len());

        let cache = tempfile::tempdir().expect("tempdir");
        let index =
            TranscriptIndex::new(paths.projects_dir(), paths.history_file(), cache.path().to_path_buf());
        let t = Instant::now();
        let first = index.recent_sessions(5);
        let first_t = t.elapsed();
        let t = Instant::now();
        let second = index.recent_sessions(5);
        let second_t = t.elapsed();
        println!("transcripts: 5 most recent; recent_sessions cold {first_t:?}, warm {second_t:?}");
        for s in &second {
            println!(
                "  {} status={:?} last_active={} cwd={} branch={:?} title={:?} path={}",
                short(&s.id),
                s.status,
                s.last_active_at,
                s.cwd,
                s.git_branch,
                s.title,
                s.transcript_path.as_deref().map(|p| p.rsplit('/').next().unwrap_or(p)).unwrap_or("-")
            );
        }
        assert_eq!(first, second);

        for s in &warm {
            let e = index.enrich(s.clone());
            println!("  enrich {} -> title={:?} branch={:?}", short(&e.id), e.title, e.git_branch);
        }
    }
}
