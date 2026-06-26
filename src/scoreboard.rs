use crate::model::CommitRecord;
use crate::stats::{
    battlefield::{file_battlefield, Battlefield},
    biggest::{biggest_commit, BiggestCommit},
    busiest::{busiest_day, BusiestDay},
    churn::{churn_hotspots, ChurnStat},
    committers::{top_committers, CommitterStat},
    nightowl::{night_owls, NightOwlStats},
    oops::{oops_board, OopsBoard},
    ownership::{ownership, OwnershipStat},
    streaks::{longest_streaks, StreakStat},
    vitals::{vitals, Vitals},
    words::{commit_types, top_bigrams, top_words, TypeCount, WordCount},
};

pub struct Scoreboard {
    pub committers: Vec<CommitterStat>,
    pub churn: Vec<ChurnStat>,
    pub biggest: Option<BiggestCommit>,
    pub nightowls: NightOwlStats,
    pub streaks: Vec<StreakStat>,
    pub words: Vec<WordCount>,
    pub types: Vec<TypeCount>,
    pub bigrams: Vec<WordCount>,
    pub ownership: Vec<OwnershipStat>,
    pub vitals: Option<Vitals>,
    pub oops: OopsBoard,
    pub busiest: Option<BusiestDay>,
    pub battlefield: Vec<Battlefield>,
}

/// Run all independent analyzers concurrently over the same commit slice.
pub fn analyze(records: &[CommitRecord], include_generated: bool) -> Scoreboard {
    let mut committers = None;
    let mut churn = None;
    let mut biggest = None;
    let mut nightowls = None;
    let mut streaks = None;
    let mut words = None;
    let mut types = None;
    let mut bigrams = None;
    let mut ownership_ = None;
    let mut vitals_ = None;
    let mut oops_ = None;
    let mut busiest_ = None;
    let mut battlefield_ = None;

    rayon::scope(|s| {
        s.spawn(|_| committers = Some(top_committers(records)));
        s.spawn(|_| churn = Some(churn_hotspots(records, include_generated)));
        s.spawn(|_| biggest = Some(biggest_commit(records)));
        s.spawn(|_| nightowls = Some(night_owls(records)));
        s.spawn(|_| streaks = Some(longest_streaks(records)));
        s.spawn(|_| words = Some(top_words(records, 30)));
        s.spawn(|_| types = Some(commit_types(records)));
        s.spawn(|_| bigrams = Some(top_bigrams(records, 6)));
        s.spawn(|_| ownership_ = Some(ownership(records)));
        s.spawn(|_| vitals_ = Some(vitals(records)));
        s.spawn(|_| oops_ = Some(oops_board(records)));
        s.spawn(|_| busiest_ = Some(busiest_day(records)));
        s.spawn(|_| battlefield_ = Some(file_battlefield(records, include_generated)));
    });

    Scoreboard {
        committers: committers.unwrap(),
        churn: churn.unwrap(),
        biggest: biggest.unwrap(),
        nightowls: nightowls.unwrap(),
        streaks: streaks.unwrap(),
        words: words.unwrap(),
        types: types.unwrap(),
        bigrams: bigrams.unwrap(),
        ownership: ownership_.unwrap(),
        vitals: vitals_.unwrap(),
        oops: oops_.unwrap(),
        busiest: busiest_.unwrap(),
        battlefield: battlefield_.unwrap(),
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
            rec("bob", 1_704_153_600, &[("a.rs", 2, 0)]),
        ];
        let sb = analyze(&records, false);
        assert_eq!(sb.committers.len(), 2);
        assert_eq!(sb.churn[0].path, "a.rs");
        assert!(sb.biggest.is_some());
        assert_eq!(sb.types.iter().map(|t| t.count).sum::<usize>(), 2);
        assert!(sb.bigrams.is_empty());
        assert!(sb.vitals.is_some());
        assert_eq!(sb.vitals.unwrap().total_commits, 2);
        // New panels exist (a.rs has 2 authors -> battlefield).
        assert_eq!(sb.oops.total_oops, 0);
        assert!(sb.busiest.is_some());
        assert!(sb.battlefield.iter().any(|b| b.path == "a.rs"));
    }
}
