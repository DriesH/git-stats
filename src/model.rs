#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChurn {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecord {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: i64,
    pub tz_offset_minutes: i32,
    pub message: String,
    pub files: Vec<FileChurn>,
}

impl CommitRecord {
    pub fn lines_changed(&self) -> u64 {
        self.files
            .iter()
            .map(|f| u64::from(f.added) + u64::from(f.removed))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_changed_sums_added_and_removed() {
        let r = CommitRecord {
            sha: "abc".into(),
            author_name: "alice".into(),
            author_email: "a@x".into(),
            timestamp: 0,
            tz_offset_minutes: 0,
            message: "m".into(),
            files: vec![
                FileChurn {
                    path: "a.rs".into(),
                    added: 3,
                    removed: 1,
                },
                FileChurn {
                    path: "b.rs".into(),
                    added: 10,
                    removed: 0,
                },
            ],
        };
        assert_eq!(r.lines_changed(), 14);
    }
}
