// techniques/only_source.rs — 唯一光源（分 64）
// 未照亮白格恰有一个可行光源 → 放灯。
//
// 语义：
//   先扫描全部格：对每个 白格 && 未照亮 的格，统计其 reach（含自身）中未决的
//   候选数：
//     - 候选 == 1 → 记录该候选（唯一光源）
//     - 候选 == 0 → 直接返回 2（矛盾：未照亮格无任何可行光源）
//     - 候选 >= 2 → 跳过
//   扫描完成后，才统一对记录的每个候选放灯。
//   （关键：先收集后应用——应用过程不得影响候选判定）

use crate::{answer_methods::Technique, answer_model::Model, techniques::Status};

pub fn unique_source(model: &mut Model) -> Status {
    let lit = model.board.lit(&model.state);
    let n = model.state.len();
    let mut candidates = Vec::new();
    for i in 0..n {
        if model.board.cells[i] != 0 {
            continue;
        }
        if lit[i] != 0 {
            continue;
        }
        // 候选：reach（含自身）中未决的格。
        // 扫描 reach 找第一个未决候选，再检查是否还有第二个
        // （恰好 1 个才记录；>=2 跳过；0 矛盾）。
        let mut sole_source = 0usize;
        let mut cand_count = 0u8;
        for &cand in &model.board.reach[i] {
            if model.state[cand] == 0 {
                cand_count += 1;
                if cand_count == 1 {
                    sole_source = cand;
                } else if cand_count >= 2 {
                    break;
                }
            }
        }
        match cand_count {
            0 => return Status::Broken, // 未照亮格无任何可行光源 → 矛盾
            1 => candidates.push(sole_source),
            _ => (),
        }
    }
    let mut applied = false;
    for &cand in &candidates {
        match model.apply(cand, 1, Technique::OnlySource) {
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
