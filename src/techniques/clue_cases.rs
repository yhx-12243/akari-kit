// techniques/clue_cases.rs — 数字周围情况分析（分 2）
// 枚举数字墙邻格的放灯组合，求所有有效组合的共同确定格。
//
// 语义：
//   - 全部数字墙都基于入口时的状态快照做组合测试（不逐墙提交后继续）。
//   - 每个组合：复制快照 → 选中格放灯、其余候选格禁灯 → 只对组合灯格的
//     照程禁灯（state==0→2，1/2 跳过，不冲突）→ 一致性校验。
//   - 每墙确定格 = 该墙所有有效组合在同一格给出相同状态（放灯或禁灯）；
//     各墙确定格求并进全局提交列表（冲突由 apply 处理 → 矛盾）。
//   - 无有效组合 → 跳过继续（不返回 2）；无确定格 → 返回 1。

use crate::{
    answer_methods::Technique, answer_model::Model, board::is_consistent, techniques::Status,
};

/// 从 cands 中选 k 个的全部组合。
fn combinations(cands: &[usize], k: usize) -> Vec<Vec<usize>> {
    fn rec(
        cands: &[usize],
        k: usize,
        start: usize,
        current: &mut Vec<usize>,
        out: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == k {
            out.push(current.clone());
            return;
        }
        for i in start..cands.len() {
            current.push(cands[i]);
            rec(cands, k, i + 1, current, out);
            current.pop();
        }
    }
    let mut out = Vec::new();
    let mut current = Vec::new();
    rec(cands, k, 0, &mut current, &mut out);
    out
}

pub fn clue_case_analysis(model: &mut Model) -> Status {
    let n = model.state.len();
    // 所有数字墙基于入口快照测试
    let entry = model.state.clone();
    let walls = model.board.digit_walls();
    let mut determined: Vec<(usize, u8)> = Vec::new();
    let mut any_wall = false;

    for (wall, digit) in walls {
        let mut placed_lights = 0u32;
        let mut cands = Vec::new();
        for &neighbor in &model.board.neighbors[wall] {
            match entry[neighbor] {
                1 => placed_lights += 1,
                0 => cands.push(neighbor),
                _ => (),
            }
        }
        // need = digit - placed_lights；仅当 0 < need < cands.len() 才处理（placed_lights>digit → 跳过）
        if placed_lights > digit {
            continue;
        }
        let need = (digit - placed_lights) as usize;
        if need == 0 || need >= cands.len() {
            continue;
        }
        // 枚举全部组合；每个组合：选中=放灯，其余候选=禁灯
        let combos = combinations(&cands, need);
        // 该墙 A/B 位图（全 1 初始化）：A 在格为 1/0 时清除（→ 恒放灯）；
        // B 在格为 2/0 时清除（→ 恒禁灯）
        let mut wall_a = vec![1u8; n];
        let mut wall_b = vec![1u8; n];
        let mut valid_count = 0usize;
        for combo in &combos {
            let mut copy = entry.clone();
            for &cand in &cands {
                let value = if combo.contains(&cand) { 1u8 } else { 2u8 };
                debug_assert_eq!(copy[cand], 0);
                copy[cand] = value;
            }
            // 只对组合灯格做照程禁灯（state==0→2，其余跳过；不检测冲突）
            for &lamp in combo {
                for &reached in &model.board.lamp_reach[lamp] {
                    if copy[reached] == 0 {
                        copy[reached] = 2;
                    }
                }
            }
            if is_consistent(model.board, &copy) {
                valid_count += 1;
                for i in 0..n {
                    match copy[i] {
                        0 => {
                            wall_a[i] = 0;
                            wall_b[i] = 0;
                        }
                        1 => wall_a[i] = 0,
                        _ => wall_b[i] = 0,
                    }
                }
            }
        }
        if valid_count == 0 {
            // 无有效组合 → 跳过本墙，不返回 2
            continue;
        }
        any_wall = true;
        // 该墙确定格（仅未决格）；各墙结果求并
        for i in 0..n {
            if entry[i] != 0 {
                continue;
            }
            if wall_b[i] == 1 {
                determined.push((i, 1));
            } else if wall_a[i] == 1 {
                determined.push((i, 2));
            }
        }
    }

    if !any_wall || determined.is_empty() {
        return Status::Stopped;
    }
    let mut applied = false;
    for (cell, value) in determined {
        match model.apply(cell, value, Technique::ClueCaseAnalysis) {
            Status::Completed => applied = true,
            Status::Stopped => (),
            Status::Broken => return Status::Broken,
        }
    }
    if applied {
        Status::Completed
    } else {
        Status::Stopped
    }
}
