// board.rs — 棋盘数据结构与基础规则传播

use crate::{answer_methods::*, answer_model::Model, techniques::Status};

pub struct Board<'a> {
    // cells：0=白格, 1=墙, 2..=6=数字墙（值为 2+数字）
    pub cells: &'a [u8],
    // 每格的 4 邻格列表（含墙，用于数字墙邻格约束）
    pub neighbors: Vec<Vec<usize>>,
    // 每格放灯后被照到的格（不含自身、不含墙）
    pub lamp_reach: Vec<Vec<usize>>,
    // 每格放灯后可照到的格 + 自身（用于唯一光源/光源分析候选与 lit 计算）
    pub reach: Vec<Vec<usize>>,
    // 预计算数字墙列表 (格子索引, 数字)
    pub walls: Vec<(usize, u32)>,
}

impl<'a> Board<'a> {
    pub fn new(w: usize, h: usize, data: &'a [u8]) -> Self {
        let n = w * h;

        let idx = |c: usize, r: usize| r * w + c;
        let mut neighbors = vec![Vec::new(); n];
        let mut lamp_reach = vec![Vec::new(); n];
        let mut reach = vec![Vec::new(); n];
        for r in 0..h {
            for c in 0..w {
                let i = idx(c, r);
                let cell = data[i];
                // 4 方向：第一个格（含墙）
                if c > 0 {
                    neighbors[i].push(idx(c - 1, r));
                }
                if c + 1 < w {
                    neighbors[i].push(idx(c + 1, r));
                }
                if r > 0 {
                    neighbors[i].push(idx(c, r - 1));
                }
                if r + 1 < h {
                    neighbors[i].push(idx(c, r + 1));
                }
                // lamp_reach：向 4 方向延伸至墙（不含墙与自身）。
                // 方向顺序：右、左、下、上（灯照冲突时序依赖此顺序）。
                if cell == 0 {
                    for dir in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                        let (mut col, mut row) = (c as i32 + dir.0, r as i32 + dir.1);
                        while col >= 0 && col < w as i32 && row >= 0 && row < h as i32 {
                            let j = idx(col as usize, row as usize);
                            if data[j] != 0 {
                                break;
                            }
                            lamp_reach[i].push(j);
                            reach[i].push(j);
                            col += dir.0;
                            row += dir.1;
                        }
                    }
                    // reach 含自身（灯可照亮自己）
                    reach[i].push(i);
                }
            }
        }
        let walls = data
            .iter()
            .enumerate()
            .filter_map(|(i, &c)| if c >= 2 { Some((i, c as u32 - 2)) } else { None })
            .collect();
        Self {
            cells: data,
            neighbors,
            lamp_reach,
            reach,
            walls,
        }
    }

    /// 收集数字墙列表 (格子索引, 数字)。
    pub fn digit_walls(&self) -> Vec<(usize, u32)> {
        self.walls.clone()
    }

    /// 由当前灯计算被照亮的格（reach 含自身 → 灯格本身也算被照亮）。
    pub fn lit(&self, state: &[u8]) -> Vec<u8> {
        let n = state.len();
        let mut lit = vec![0u8; n];
        for i in 0..n {
            if state[i] != 1 {
                continue;
            }
            for &c in &self.reach[i] {
                lit[c] = 1;
            }
        }
        lit
    }
}

/// 数字墙邻格约束：数字墙的 4 邻格中，
/// 灯数已等于数字 → 其余未决邻格禁灯；
/// 未决数恰好补足缺口 → 全部放灯；否则矛盾。
pub fn clue_neighbors(model: &mut Model) -> Status {
    let mut applied = false;
    for &(wall, digit) in &model.board.walls {
        let mut placed_lights = 0u32;
        let mut undecided_count = 0u32;
        for &neighbor in &model.board.neighbors[wall] {
            match model.state[neighbor] {
                1 => placed_lights += 1,
                0 => undecided_count += 1,
                _ => (),
            }
        }
        if placed_lights > digit || placed_lights + undecided_count < digit {
            return Status::Broken;
        }
        let (value, tech) = if placed_lights == digit {
            (2, Technique::ClueFull)
        } else if placed_lights + undecided_count == digit {
            (1, Technique::JustEnough)
        } else {
            continue;
        };
        for &neighbor in &model.board.neighbors[wall] {
            if model.state[neighbor] == 0 {
                match model.apply(neighbor, value, tech) {
                    Status::Completed => applied = true,
                    Status::Stopped => (),
                    Status::Broken => return Status::Broken,
                }
            }
        }
    }
    if applied {
        Status::Completed
    } else {
        Status::Stopped
    }
}

/// 灯照路径禁灯：灯的可照格不能再放灯。
pub fn lamp_beam(model: &mut Model) -> Status {
    let mut applied = false;
    for i in 0..model.state.len() {
        if model.state[i] != 1 {
            continue;
        }
        for &lit in &model.board.lamp_reach[i] {
            match model.apply(lit, 2, Technique::LampBeam) {
                Status::Completed => applied = true,
                Status::Stopped => (),
                Status::Broken => return Status::Broken,
            }
        }
    }
    if applied {
        Status::Completed
    } else {
        Status::Stopped
    }
}

/// 一致性检查：
/// 1. 灯互射；2. 数字墙 placed_lights<=digit<=placed_lights+未决；3. 未照亮白格必须有未决候选。
pub fn is_consistent(board: &Board, state: &[u8]) -> bool {
    let n = state.len();
    // 1. 灯互射
    for i in 0..n {
        if state[i] != 1 {
            continue;
        }
        for &c in &board.lamp_reach[i] {
            if state[c] == 1 {
                return false;
            }
        }
    }
    // 2. 数字墙
    for &(wall, digit) in &board.walls {
        let mut placed_lights = 0u32;
        let mut undecided = 0u32;
        for &neighbor in &board.neighbors[wall] {
            match state[neighbor] {
                1 => placed_lights += 1,
                0 => undecided += 1,
                _ => (),
            }
        }
        if placed_lights > digit || digit > placed_lights + undecided {
            return false;
        }
    }
    // 3. 未照亮白格必须有未决候选（候选列表 = reach 含自身：灯可照亮自己）
    let lit = board.lit(state);
    for v in 0..n {
        if board.cells[v] != 0 {
            continue;
        }
        if lit[v] != 0 {
            continue;
        }
        if board.reach[v].iter().all(|&c| state[c] != 0) {
            return false;
        }
    }
    true
}

/// 完整盘面判定：所有格已定且构成合法解。
/// 违规项：1) 有未决格；2) 数字墙灯数不等于数字；3) 灯互射；4) 白格未照亮。
pub fn is_solution(board: &Board, state: &[u8]) -> bool {
    let n = state.len();
    if (0..n).any(|i| state[i] == 0) {
        return false;
    }
    for i in 0..n {
        if state[i] != 1 {
            continue;
        }
        for &c in &board.lamp_reach[i] {
            if state[c] == 1 {
                return false;
            }
        }
    }
    for &(wall, digit) in &board.walls {
        let placed_lights = board.neighbors[wall]
            .iter()
            .filter(|&&neighbor| state[neighbor] == 1)
            .count() as u32;
        if placed_lights != digit {
            return false;
        }
    }
    let lit = board.lit(state);
    for i in 0..n {
        if board.cells[i] == 0 && lit[i] == 0 {
            return false;
        }
    }
    true
}
