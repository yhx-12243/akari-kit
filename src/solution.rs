// solution.rs — 求解中止（STOPPED）时的解数统计
// 纯暴力 DFS 数解：cap=2，limit=2_000_000。
// 节点超预算 → Unknown（"?"）；否则数解 0/1/2+。
//
// 语义：
//   1. 入口：已计数 >=2 或已超预算 → 直接返回；节点数+1 > limit → 置超预算返回。
//   2. 传播不动点：灯照（灯的可照格中未决 → 禁灯）+ 数字墙（灯数==数字 → 未决禁灯；
//      灯数+未决==数字 → 放灯；越界/冲突 → 剪枝）。
//   3. 传播后灯互射检查 → 剪枝。
//   4. 未照亮白格必须在其 reach（含自身）有未决候选 → 否则剪枝。
//   5. 候选选择（最少候选优先）：对每个未照亮白格，候选 = [自身] + 可照格中未决格；
//      取候选最少者（严格更少才替换，平手取先）；候选数==1 的格立即停下。
//   6. 无候选（全已定）→ 完整盘面判定：合法则计数+1（cap 截断）。
//   7. 递归：候选=[候选0,候选1,...]，对 k：child=父传播态，child[候选_k]=1，
//      child[候选_0..候选_{k-1}]=2，递归；已计数 >=2 或超预算 → 立即停止展开。

use serde::Serialize;

use crate::board::{Board, is_solution};

#[derive(Debug, Copy)]
#[cfg_attr(feature = "nightly", derive_const(Clone, PartialEq, Eq, Serialize))]
#[cfg_attr(not(feature = "nightly"), derive(Clone, PartialEq, Eq, Serialize))]
pub enum SolutionCount {
    #[serde(rename = "0")]
    Empty,
    #[serde(rename = "1")]
    Unique,
    #[serde(rename = "2+")]
    Multiple,
    #[serde(rename = "?")]
    Unknown,
}

const CAP: u32 = 2;
const LIMIT: u64 = 2_000_000;

struct Tally {
    count: u32,
    unknown: bool,
}

/// 灯照传播 + 数字墙传播到不动点。返回 false = 矛盾（剪枝）。
fn propagate(board: &Board, state: &mut [u8]) -> bool {
    let n = state.len();
    loop {
        let mut changed = false;

        // 灯照：灯的可照格中未决 → 禁灯；已放灯跳过（由灯互射检查处理）
        for i in 0..n {
            if state[i] != 1 {
                continue;
            }
            for &c in &board.lamp_reach[i] {
                if state[c] == 0 {
                    state[c] = 2;
                    changed = true;
                }
            }
        }

        // 数字墙：灯数==数字 → 未决禁灯；灯数+未决==数字 → 放灯
        for &(wall, digit) in &board.walls {
            let mut used = 0u32;
            let mut undecided: Vec<usize> = Vec::new();
            for &neighbor in &board.neighbors[wall] {
                match state[neighbor] {
                    1 => used += 1,
                    0 => undecided.push(neighbor),
                    _ => (),
                }
            }
            let undecided_count = undecided.len() as u32;
            if used > digit || used + undecided_count < digit {
                return false;
            }
            let set_value = if used == digit {
                2u8
            } else if used + undecided_count == digit {
                1u8
            } else {
                continue;
            };
            for &neighbor in &undecided {
                match state[neighbor] {
                    0 => {
                        state[neighbor] = set_value;
                        changed = true;
                    }
                    v if v == set_value => (),
                    _ => return false,
                }
            }
        }

        if !changed {
            break;
        }
    }
    true
}

/// 灯互射检查。
fn lamp_conflict(board: &Board, state: &[u8]) -> bool {
    for i in 0..state.len() {
        if state[i] != 1 {
            continue;
        }
        for &c in &board.lamp_reach[i] {
            if state[c] == 1 {
                return true;
            }
        }
    }
    false
}

fn count_dfs(board: &Board, state: &[u8], node_count: &mut u64, tally: &mut Tally) {
    if tally.count >= CAP || tally.unknown {
        return;
    }
    *node_count += 1;
    if *node_count > LIMIT {
        tally.unknown = true;
        return;
    }

    let mut propagated = state.to_vec();
    if !propagate(board, &mut propagated) {
        return;
    }
    if lamp_conflict(board, &propagated) {
        return;
    }

    // 被照亮位图
    let lit = board.lit(&propagated);

    // 候选选择（最少候选优先）：候选 = [自身] + 可照格中未决格
    let mut best: Option<Vec<usize>> = None;
    for i in 0..propagated.len() {
        if board.cells[i] != 0 {
            continue;
        }
        if lit[i] != 0 {
            continue;
        }
        // 未照亮白格：收集候选
        let mut cands: Vec<usize> = Vec::new();
        if propagated[i] == 0 {
            cands.push(i);
        }
        for &c in &board.lamp_reach[i] {
            if propagated[c] == 0 {
                cands.push(c);
            }
        }
        if cands.is_empty() {
            return; // 未照亮白格无可放灯候选 → 死路
        }
        let replace = match &best {
            None => true,
            Some(b) => cands.len() < b.len(),
        };
        if replace {
            let is_single = cands.len() == 1;
            best = Some(cands);
            if is_single {
                break;
            }
        }
    }

    let Some(cands) = best else {
        // 无候选（全已定）→ 完整盘面判定：未决 → 禁灯后校验
        let mut decided = propagated.clone();
        for s in &mut decided {
            if *s == 0 {
                *s = 2;
            }
        }
        if is_solution(board, &decided) {
            tally.count += 1;
        }
        return;
    };

    // 递归：对 k：child[c_k]=1，child[c_0..c_{k-1}]=2
    for k in 0..cands.len() {
        if tally.count >= CAP || tally.unknown {
            break;
        }
        let mut child = propagated.clone();
        // child[c_k] = 1
        match child[cands[k]] {
            0 => child[cands[k]] = 1,
            1 => (),
            _ => continue, // 冲突 → 下一候选
        }
        // child[c_0..c_{k-1}] = 2
        let mut ok = true;
        for &c in &cands[..k] {
            match child[c] {
                0 => child[c] = 2,
                2 => (),
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        count_dfs(board, &child, node_count, tally);
    }
}

pub fn count(board: &Board, state: &[u8]) -> SolutionCount {
    let mut tally = Tally {
        count: 0,
        unknown: false,
    };
    let mut node_count: u64 = 0;
    count_dfs(board, state, &mut node_count, &mut tally);
    if tally.unknown {
        return SolutionCount::Unknown;
    }
    match tally.count {
        0 => SolutionCount::Empty,
        1 => SolutionCount::Unique,
        _ => SolutionCount::Multiple,
    }
}
