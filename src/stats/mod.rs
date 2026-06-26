pub mod biggest;
pub mod churn;
pub mod committers;
pub mod filters;
pub mod identity;
pub mod nightowl;
pub mod oops;
pub mod ownership;
pub mod streaks;
pub mod vitals;
pub mod words;

#[cfg(test)]
pub(crate) fn rec(author: &str, ts: i64, files: &[(&str, u32, u32)]) -> crate::model::CommitRecord {
    crate::model::CommitRecord {
        sha: format!("{author}{ts}"),
        author_name: author.into(),
        author_email: format!("{author}@x"),
        timestamp: ts,
        tz_offset_minutes: 0,
        message: "msg".into(),
        files: files
            .iter()
            .map(|(p, a, r)| crate::model::FileChurn {
                path: (*p).into(),
                added: *a,
                removed: *r,
            })
            .collect(),
    }
}
