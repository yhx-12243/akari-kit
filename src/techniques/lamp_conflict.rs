// techniques/lamp_conflict.rs — 试置灯即破綻（分 2/12）
// 对每个未决白格假置灯，若造成数字墙不足（分 12）
// 或出现无法照亮的白格（分 2）→ 该格禁灯。

use crate::{answer_methods::Technique, answer_model::Model, techniques::Status};

pub fn place_breaks(model: &mut Model) -> Status {
    let n = model.state.len();
    let lit = model.board.lit(&model.state);
    let walls = model.board.digit_walls();
    let mut forbid = Vec::new(); // (cell, tech)

    for trial_cell in 0..n {
        if model.state[trial_cell] != 0 {
            continue;
        }
        // 假置灯 trial_cell 的照程
        let mut trial_lit = vec![0u8; n];
        trial_lit[trial_cell] = 1;
        for &c in &model.board.lamp_reach[trial_cell] {
            trial_lit[c] = 1;
        }
        // 路径 B：数字墙计数（已有灯+试置灯 不超过数字，且未决且不被试置灯覆盖的格能补足）
        let mut ok = true;
        for &(wall, digit) in &walls {
            let mut placed_lights = 0u32;
            let mut open_slots = 0u32;
            for &c in &model.board.neighbors[wall] {
                if c == trial_cell || model.state[c] == 1 {
                    placed_lights += 1;
                } else if model.state[c] == 0 && trial_lit[c] == 0 {
                    open_slots += 1;
                }
            }
            if !(placed_lights <= digit && digit <= placed_lights + open_slots) {
                ok = false;
                break;
            }
        }
        if !ok {
            forbid.push((trial_cell, Technique::WouldMakeShort)); // 会造成数字不足
            continue;
        }
        // 路径 A：白格（含禁灯格）失去所有可行光源
        let mut all_ok = true;
        for white_cell in 0..n {
            if model.board.cells[white_cell] != 0 {
                continue;
            }
            if lit[white_cell] != 0 || trial_lit[white_cell] != 0 {
                continue;
            }
            let mut viable = false;
            // 候选列表用 reach（含自身）：自身作为候选可行
            for &c in &model.board.reach[white_cell] {
                if model.state[c] == 0 && trial_lit[c] == 0 {
                    viable = true;
                    break;
                }
            }
            if !viable {
                all_ok = false;
                break;
            }
        }
        if !all_ok {
            forbid.push((trial_cell, Technique::WouldCreateUnlit)); // 会出现无法照亮的格
        }
    }

    let mut applied = false;
    for &(cell, tech) in &forbid {
        match model.apply(cell, 2, tech) {
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
