use crate::model::CommitRecord;
use crate::stats::{
    biggest::{biggest_commit, BiggestCommit},
    churn::{churn_hotspots, ChurnStat},
    committers::{top_committers, CommitterStat},
    nightowl::{night_owls, NightOwlStats},
    ownership::{ownership, OwnershipStat},
    streaks::{longest_streaks, StreakStat},
    vitals::{vitals, Vitals},
    words::{top_words, WordCount},
};

pub struct Scoreboard {
    pub committers: Vec<CommitterStat>,
    pub churn: Vec<ChurnStat>,
    pub biggest: Option<BiggestCommit>,
    pub nightowls: NightOwlStats,
    pub streaks: Vec<StreakStat>,
    pub words: Vec<WordCount>,
    pub ownership: Vec<OwnershipStat>,
    pub vitals: Option<Vitals>,
}

/// Run all 8 independent analyzers concurrently over the same commit slice.
pub fn analyze(records: &[CommitRecord]) -> Scoreboard {
    let mut committers = None;
    let mut churn = None;
    let mut biggest = None;
    let mut nightowls = None;
    let mut streaks = None;
    let mut words = None;
    let mut ownership_ = None;
    let mut vitals_ = None;

    rayon::scope(|s| {
        s.spawn(|_| committers = Some(top_committers(records)));
        s.spawn(|_| churn = Some(churn_hotspots(records)));
        s.spawn(|_| biggest = Some(biggest_commit(records)));
        s.spawn(|_| nightowls = Some(night_owls(records)));
        s.spawn(|_| streaks = Some(longest_streaks(records)));
        s.spawn(|_| words = Some(top_words(records, 30)));
        s.spawn(|_| ownership_ = Some(ownership(records)));
        s.spawn(|_| vitals_ = Some(vitals(records)));
    });

    Scoreboard {
        committers: committers.unwrap(),
        churn: churn.unwrap(),
        biggest: biggest.unwrap(),
        nightowls: nightowls.unwrap(),
        streaks: streaks.unwrap(),
        words: words.unwrap(),
        ownership: ownership_.unwrap(),
        vitals: vitals_.unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::rec;

    #[test]
    fn analyze_populates_all_panels() {
        let records = vec![
            rec("alice", 1_704_067_200, &[("a.rs", 5, 1)]),
            rec("bob",   1_704_153_600, &[("a.rs", 2, 0)]),
        ];
        let sb = analyze(&records);
        assert_eq!(sb.committers.len(), 2);
        assert_eq!(sb.churn[0].path, "a.rs");
        assert!(sb.biggest.is_some());
        assert!(sb.vitals.is_some());
        assert_eq!(sb.vitals.unwrap().total_commits, 2);
    }
}
