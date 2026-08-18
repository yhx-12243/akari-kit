// answer_model.rs — 求解模型与试置
// 状态编码：0=未决白格, 1=已放灯, 2=禁灯（墙初始也为 2）

use crate::{
    answer_methods::{Methods, Technique},
    board::{clue_neighbors, lamp_beam, Board},
    techniques::{unique_source, Status},
};

pub struct Model<'a, 'b: 'a> {
    pub board: &'a Board<'b>,
    pub state: Vec<u8>,
    pub methods: Methods,
}

impl<'a, 'b: 'a> Model<'a, 'b> {
    /// 设置格子状态并记录技巧：0=应用+记录, 1=同状态无变化, 2=冲突
    pub fn apply(&mut self, idx: usize, value: u8, tech: Technique) -> Status {
        let current = self.state[idx];
        if current == 0 {
            self.state[idx] = value;
            self.methods.record(tech);
            Status::Completed
        } else if current == value {
            Status::Stopped
        } else {
            Status::Broken
        }
    }

    /// 不记录技巧的状态设置（试置传播与尾部分支结论都不进方法表）。
    pub fn apply_raw(&mut self, idx: usize, value: u8) -> Status {
        let current = self.state[idx];
        if current == 0 {
            self.state[idx] = value;
            Status::Completed
        } else if current == value {
            Status::Stopped
        } else {
            Status::Broken
        }
    }

    pub fn undecided(&self) -> usize {
        self.state.iter().filter(|&&s| s == 0).count()
    }
}

fn count_undecided(s: &[u8]) -> usize {
    s.iter().filter(|&&x| x == 0).count()
}

/// 候选格迭代：返回 index >= start 的第一个「4 邻格中有墙或已定格」的未决格；
/// 调用方传 index+1 取下一个。
pub fn next_candidate(board: &Board, state: &[u8], start: usize) -> Option<usize> {
    for i in start..state.len() {
        if state[i] != 0 {
            continue;
        }
        for &nb in &board.neighbors[i] {
            if board.cells[nb] != 0 || state[nb] != 0 {
                return Some(i);
            }
        }
    }
    None
}

/// 扫描两个试置副本，找首个 base 未决且两副本同非零状态的格。
pub fn common_determined(base: &[u8], copy1: &[u8], copy2: &[u8], start: usize) -> (usize, u8) {
    for i in start..base.len() {
        if base[i] != 0 {
            continue;
        }
        let s = copy1[i];
        if s != 0 && s == copy2[i] {
            return (i, s);
        }
    }
    (base.len(), 3)
}

/// 试置阶段的难度计数（参与难度分公式）。
/// conclusions 权重 40，nested 权重 4。
#[derive(Default)]
pub struct TrialTally {
    /// 试置得出确定性结论的次数（恰一分支矛盾落实 / 共同确定格落实）。
    pub conclusions: u32,
    /// 嵌套试置内部累计（递归子试置的结论+嵌套计数之和）。
    pub nested: u32,
}

/// 试置：对候选格逐一假置灯/禁灯各一次，
/// 1) 用候选迭代器收集全部候选格；
/// 2) 每个候选格试置 state=1/2，各自传播（depth>=1 时传播循环内递归试置 depth-1）；
/// 3) 尾部分支：全矛盾→2；恰一矛盾→结论次数++ 后把另一分支的状态落实（试置矛盾）；
///    两分支都可行→找共同确定格，有→结论次数++ 后全部落实（无论如何都确定），无→下一个候选格；
/// 4) 全部候选无结论→返回 1。
/// tally：试置阶段的难度计数（conclusions / nested）。
pub fn trial_placement(model: &mut Model, tally: &mut TrialTally, depth: u32) -> Status {
    // 收集全部候选
    let mut candidates = Vec::new();
    let mut cursor = 0usize;
    while let Some(i) = next_candidate(model.board, &model.state, cursor) {
        candidates.push(i);
        cursor = i + 1;
    }
    if candidates.is_empty() {
        return Status::Stopped;
    }

    for &cell in &candidates {
        if model.state[cell] != 0 {
            continue;
        }
        // 两个试置分支 (state, ret, copy)
        let mut trials: Vec<(u8, Status, Vec<u8>)> = Vec::new();
        for value in [1u8, 2u8] {
            let mut copy = model.state.clone();
            copy[cell] = value;
            let ret = trial_propagate(model, &mut copy, tally, depth);
            trials.push((value, ret, copy));
        }

        let broken0 = trials[0].1 == Status::Broken;
        let broken1 = trials[1].1 == Status::Broken;
        // 全矛盾 → BROKEN
        if broken0 && broken1 {
            return Status::Broken;
        }
        // 恰一分支矛盾 → 用另一分支的状态落实候选格（试置矛盾）
        if broken0 || broken1 {
            tally.conclusions += 1;
            let value = if broken0 { trials[1].0 } else { trials[0].0 };
            return model.apply_raw(cell, value);
        }
        // 两分支都可行 → 找共同确定格（无论如何都确定）
        let mut found = false;
        let mut result = Status::Stopped;
        let mut start = 0usize;
        loop {
            let (idx, value) = common_determined(&model.state, &trials[0].2, &trials[1].2, start);
            if value == 3 {
                break;
            }
            found = true;
            let apply_ret = model.apply_raw(idx, value);
            if apply_ret == Status::Broken {
                result = Status::Broken;
            } else if apply_ret == Status::Completed {
                result = Status::Completed;
            }
            start = idx + 1;
        }
        if found {
            tally.conclusions += 1;
            return result;
        }
        // 无共同确定格 → 下一个候选格
    }
    Status::Stopped
}

/// 试置分支的传播：数字墙邻格约束/灯照路径禁灯/唯一光源到未决数稳定（方法丢弃）。
/// depth>=1：稳定且仍有未决 → 递归试置 depth-1；递归有进展 → 回传播循环。
fn trial_propagate(model: &Model, copy: &mut Vec<u8>, tally: &mut TrialTally, depth: u32) -> Status {
    loop {
        let before = count_undecided(copy);
        let mut prop_model = Model {
            board: model.board,
            state: std::mem::take(copy),
            methods: Methods::default(),
        };
        let neighbor_ret = clue_neighbors(&mut prop_model);
        let beam_ret = lamp_beam(&mut prop_model);
        let source_ret = unique_source(&mut prop_model);
        *copy = prop_model.state;
        if neighbor_ret == Status::Broken || beam_ret == Status::Broken || source_ret == Status::Broken {
            return Status::Broken;
        }
        let after = count_undecided(copy);
        if before != after {
            continue; // 未决数仍在变 → 继续传播
        }
        if depth == 0 {
            return Status::Stopped; // depth=0 不递归
        }
        if after == 0 {
            return Status::Stopped; // 该分支已全解
        }
        // 递归试置 depth-1，作用于当前 copy
        let mut nested_tally = TrialTally::default();
        let mut sub_model = Model {
            board: model.board,
            state: std::mem::take(copy),
            methods: Methods::default(),
        };
        let nested_ret = trial_placement(&mut sub_model, &mut nested_tally, depth - 1);
        *copy = sub_model.state;
        tally.nested += nested_tally.conclusions + nested_tally.nested;
        if nested_ret == Status::Completed {
            continue; // 递归有进展 → 回传播循环
        }
        return nested_ret;
    }
}
