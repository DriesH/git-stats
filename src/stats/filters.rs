//! Shared predicate for skipping lock / generated / vendored files in
//! churn-style leaderboards.

/// Exact basenames of dependency lock files.
const LOCK_BASENAMES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "go.sum",
    "flake.lock",
];

/// Path suffixes for minified / generated artifacts.
const GENERATED_SUFFIXES: &[&str] = &[".min.js", ".min.css", ".map"];

/// Vendored directory names; matched against any `/`-separated path component.
const VENDOR_DIRS: &[&str] = &["vendor", "node_modules", "dist", "build"];

/// True when `path` is a lock file, minified/generated artifact, or lives in a
/// vendored directory — churn noise rather than authored code. `path` is the
/// repo-relative, `/`-separated string stored in `FileChurn.path`.
pub fn is_generated_path(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    if LOCK_BASENAMES.contains(&basename) {
        return true;
    }
    if GENERATED_SUFFIXES.iter().any(|s| path.ends_with(s)) {
        return true;
    }
    path.split('/').any(|c| VENDOR_DIRS.contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_lock_basenames_anywhere_in_tree() {
        assert!(is_generated_path("Cargo.lock"));
        assert!(is_generated_path("frontend/package-lock.json"));
        assert!(is_generated_path("a/b/go.sum"));
    }

    #[test]
    fn matches_minified_and_map_suffixes() {
        assert!(is_generated_path("static/app.min.js"));
        assert!(is_generated_path("static/app.min.css"));
        assert!(is_generated_path("bundle.js.map"));
    }

    #[test]
    fn matches_vendored_directory_components() {
        assert!(is_generated_path("node_modules/left-pad/index.js"));
        assert!(is_generated_path("dist/main.js"));
        assert!(is_generated_path("go/vendor/foo/bar.go"));
    }

    #[test]
    fn keeps_authored_source_files() {
        assert!(!is_generated_path("src/main.rs"));
        assert!(!is_generated_path("README.md"));
        // "rebuild" contains "build" as a substring but not as a component.
        assert!(!is_generated_path("src/rebuild/mod.rs"));
    }
}
