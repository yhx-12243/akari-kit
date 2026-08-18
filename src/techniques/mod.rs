// techniques/ — 点灯解法的技巧层

pub mod clue_cases;
pub mod lamp_conflict;
pub mod only_source;
pub mod source_cases;

use serde::Serialize;

use crate::{
    answer_model::Model,
    board::{clue_neighbors, lamp_beam},
};
pub use clue_cases::clue_case_analysis;
pub use lamp_conflict::place_breaks;
pub use only_source::unique_source;
pub use source_cases::source_case_analysis;

#[derive(Debug, Copy)]
#[cfg_attr(feature = "nightly", derive_const(Serialize, Clone, PartialEq, Eq))]
#[cfg_attr(not(feature = "nightly"), derive(Serialize, Clone, PartialEq, Eq))]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Completed,
    Stopped,
    Broken,
}

pub type Technique = fn(&mut Model) -> Status;

/// 技巧链（按难度从低到高）。
pub const ALL_TECHNIQUES: [Technique; 6] = [
    clue_neighbors,
    lamp_beam,
    unique_source,
    place_breaks,
    clue_case_analysis,
    source_case_analysis,
];
