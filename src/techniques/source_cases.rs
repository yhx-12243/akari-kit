// techniques/source_cases.rs — 光源情况分析（分 24）
// 未照亮白格的候选光源逐一试置，求所有有效试置的共同确定格。
//
// 语义：
//   - 目标 = 未照亮白格（可为未决或禁灯），候选 = 其 reach 中未决白格
//   - 候选数 2..12 才成为目标；目标按 (候选数, 单元格索引) 稳定排序，只取前 64 个
//   - 依序处理每个目标：逐一试置候选（trial=主状态+候选放灯）→ 数字墙邻格约束/
//     灯照路径禁灯/唯一光源传播到不动点 → 一致性校验；失败或传播矛盾 → 该候选无效
//   - A[u]==1 ⇔ 所有有效试置 trial[u]==1（放灯）；B[u]==1 ⇔ 全部 trial[u]==2（禁）
//   - 有效候选数==0 或确定格==0 → 下一个目标；有确定格 → 提交后立即返回

use crate::{
    answer_methods::{Methods, Technique},
    answer_model::Model,
    board::{clue_neighbors, is_consistent, lamp_beam},
    techniques::{Status, unique_source},
};

pub fn source_case_analysis(model: &mut Model) -> Status {
    let n = model.state.len();
    let lit = model.board.lit(&model.state);
    // 目标 = 未照亮白格；候选 = reach 中未决白格；候选数 2..12
    let mut targets: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        if model.board.cells[i] != 0 || lit[i] != 0 {
            continue;
        }
        let cands: Vec<usize> = model.board.reach[i]
            .iter()
            .copied()
            .filter(|&c| model.state[c] == 0)
            .collect();
        if cands.len() >= 2 && cands.len() <= 12 {
            targets.push((i, cands));
        }
    }
    if targets.is_empty() {
        return Status::Stopped;
    }
    // 按 (候选数, 单元格索引) 稳定排序；只保留前 64 个目标
    targets.sort_by_key(|(idx, cands)| (cands.len(), *idx));
    targets.truncate(64);

    let mut determined = Vec::new();

    for (_, cands) in &targets {
        let mut always_forbidden = vec![1u8; n]; // 全部试置禁灯 → 恒禁
        let mut always_placed = vec![1u8; n]; // 全部试置放灯 → 恒放
        let mut valid_count = 0usize;
        for &cand in cands {
            let mut copy = model.state.clone();
            copy[cand] = 1;
            let mut prev = usize::MAX;
            let mut bad = false;
            loop {
                let mut prop_model = Model {
                    board: model.board,
                    state: copy,
                    methods: Methods::default(),
                };
                let neighbor_ret = clue_neighbors(&mut prop_model);
                let beam_ret = lamp_beam(&mut prop_model);
                let source_ret = unique_source(&mut prop_model);
                let undecided = prop_model.undecided();
                copy = prop_model.state;
                if neighbor_ret == Status::Broken
                    || beam_ret == Status::Broken
                    || source_ret == Status::Broken
                {
                    bad = true;
                    break;
                }
                if undecided == prev {
                    break;
                }
                prev = undecided;
            }
            // 校验失败或传播矛盾 → 该候选无效（跳过，不参与 A/B）
            if bad || !is_consistent(model.board, &copy) {
                continue;
            }
            valid_count += 1;
            for i in 0..n {
                match copy[i] {
                    0 => {
                        always_forbidden[i] = 0;
                        always_placed[i] = 0;
                    }
                    1 => always_forbidden[i] = 0, // 放灯 → 清"全部禁"
                    _ => always_placed[i] = 0,    // 禁 → 清"全部放"
                }
            }
        }
        if valid_count == 0 {
            // 全部候选无效 → 返回 2
            return Status::Broken;
        }

        for i in 0..n {
            if model.state[i] != 0 {
                continue;
            }
            if always_placed[i] == 1 {
                determined.push((i, 1));
            } else if always_forbidden[i] == 1 {
                determined.push((i, 2));
            }
        }
        if !determined.is_empty() {
            break;
        }
    }

    let mut applied = false;
    for (cell, value) in determined {
        match model.apply(cell, value, Technique::SourceCaseAnalysis) {
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
