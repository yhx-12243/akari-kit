#![feature(const_clone, const_cmp, derive_const)]

// lib.rs — 点灯（akari）求解器入口
// 搜索主循环（难度逐级升级）+ 技巧链 + 难度评分

pub mod answer_methods;
pub mod answer_model;
pub mod board;
pub mod solution;
pub mod techniques;

use core::slice;

use serde::Serialize;

use answer_methods::Methods;
use answer_model::{Model, trial_placement, TrialTally};
use board::{is_solution, Board};
use solution::SolutionCount;
use techniques::{ALL_TECHNIQUES, Status};

#[derive_const(Serialize)]
pub struct LpResult {
    status: Status,
    solutions: SolutionCount,
    dp: u64,
    level: u32,
    unknown: usize,
    rating_hundredths: Option<u32>,
    difficulty_version: usize,
}

// 难度分：solutions 为 "0"/"2+" 时 → null；
// 否则对 dp 分段线性插值。
// 锚点 (x1,y1)->(x2,y2)：(0,50)->(1000726,150)->(3001150,250)->
// (4004408,350)->(6000000,450)->(7064704,549)。
fn rating_from_dp(dp: u64) -> u32 {
    let dp = dp as i64;
    if dp <= 0 {
        return 50;
    }
    if dp > 7064703 {
        return 549;
    }
    // (x1, y1, x2, y2)
    let segs: [(i64, i64, i64, i64); 5] = [
        (0, 50, 1000726, 150),
        (1000726, 150, 3001150, 250),
        (3001150, 250, 4004408, 350),
        (4004408, 350, 6000000, 450),
        (6000000, 450, 7064704, 549),
    ];
    for &(x1, y1, x2, y2) in segs.iter() {
        if dp < x2 {
            let num = 2 * (dp - x1) * (y2 - y1) + (x2 - x1);
            let den = 2 * (x2 - x1);
            return (y1 + num / den) as u32;
        }
    }
    549
}

pub fn solve_by_logicpuzzle(width: usize, height: usize, data: *const u8) -> LpResult {
    let n = width * height;
    let slice = unsafe { slice::from_raw_parts(data, n) };
    let board = Board::new(width, height, slice);

    // 状态：白格=0，非白格=2（墙）
    let mut state = vec![0u8; n];
    for i in 0..n {
        if board.cells[i] != 0 {
            state[i] = 2;
        }
    }

    // 搜索配置：无限重试；允许第 7 级嵌套试置；第 6 级阈值 5
    let max_retries: u32 = 0; // 技巧链重试上限（0=无限）
    let allow_level7: u32 = 1; // 允许 level7 嵌套
    let trial_threshold: u32 = 5; // 进入试置的未决格数阈值
    let stop_before_trial: u32 = 0;

    let mut level: u32 = 1; // 当前难度等级
    let mut chain_pass: u32 = 0; // 技巧链 pass 计数
    let mut level7_done: u32 = 0; // 是否已进入第 7 级
    let mut snapshot_gate: u32 = 0; // 第 6 级升级门控（未决格数快照只记一次）
    let mut trial_tally = TrialTally::default(); // 试置阶段难度计数
    let mut level6_undecided: u32 = 0; // 进入第 6 级试置时的未决格数
    let mut level7_undecided: u32 = 0; // 第 7 级试置后的未决格数
    let mut methods = Methods::default();

    // 0=COMPLETED 1=STOPPED 2=BROKEN
    let status = 'outer: loop {
        let iteration_level = level;
        let level_min = iteration_level.min(5) as usize;
        // 方法表随求解全程累积，不随 level/pass 清空
        // 技巧链 pass
        'chain: loop {
            chain_pass += 1;
            let mut all_no_change = true;
            for &tech in unsafe { ALL_TECHNIQUES.get_unchecked(..level_min + 1) } {
                let mut model = Model {
                    board: &board,
                    state: state.clone(),
                    methods: Methods::default(),
                };
                // 技巧在共享 methods 上记录
                model.methods = std::mem::take(&mut methods);
                let ret = tech(&mut model);
                methods = std::mem::take(&mut model.methods);
                state = model.state;
                match ret {
                    Status::Completed => all_no_change = false,
                    Status::Stopped => (),
                    Status::Broken => break 'outer Status::Broken,
                }
            }
            if all_no_change {
                // 不动点：数未决
                let undecided = state.iter().filter(|&&s| s == 0).count();
                if undecided == 0 {
                    // 全部确定 → 校验盘面是否真正合法（数字墙恰好、灯不互射、白格全亮）。
                    // 不合法 → BROKEN（如 4/2/..a10a：禁灯格未照亮）。
                    break 'outer if is_solution(&board, &state) {
                        Status::Completed
                    } else {
                        Status::Broken
                    };
                }
                // 未决格数达到阈值 → 进入试置
                if iteration_level >= trial_threshold {
                    if snapshot_gate & 1 == 0 {
                        level6_undecided = undecided as u32;
                    }
                    if stop_before_trial != 0 {
                        break 'outer Status::Completed;
                    }
                    level = level.max(6);
                    snapshot_gate = 1;
                    let mut model = Model {
                        board: &board,
                        state: state.clone(),
                        methods: std::mem::take(&mut methods),
                    };
                    let ret = trial_placement(&mut model, &mut trial_tally, 0);
                    methods = std::mem::take(&mut model.methods);
                    state = model.state;
                    match ret {
                        Status::Completed => continue 'outer,
                        Status::Broken => break 'outer Status::Completed,
                        Status::Stopped => {
                            if level7_done == 0 {
                                level7_undecided = state.iter().filter(|&&s| s == 0).count() as u32;
                            }
                            if allow_level7 == 0 {
                                break 'outer Status::Completed;
                            }
                            level7_done = 1;
                            level = 7;
                            let mut model = Model {
                                board: &board,
                                state: state.clone(),
                                methods: std::mem::take(&mut methods),
                            };
                            let ret = trial_placement(&mut model, &mut trial_tally, allow_level7);
                            methods = std::mem::take(&mut model.methods);
                            state = model.state;
                            if ret == Status::Completed {
                                continue 'outer;
                            }
                            break 'outer ret;
                        }
                    }
                } else {
                    level += 1;
                    continue 'outer;
                }
            } else {
                // 有技巧应用 → 重试（max_retries<=0 无限；否则 chain_pass<max_retries）
                if max_retries == 0 || chain_pass < max_retries {
                    continue 'chain;
                }
                break 'outer Status::Completed;
            }
        }
    };

    // 收尾：STOPPED 置等级 8；COMPLETED 时若仍有未决 → 也算
    let undecided = state.iter().filter(|&&s| s == 0).count();
    let (level, unknown) = match status {
        Status::Completed => (level, 0),
        Status::Stopped => (8, undecided),
        Status::Broken => (level, undecided),
    };

    // 评分
    let dp: u64 = if status == Status::Broken {
        0
    } else {
        let tech = methods.tech();
        let mut difficulty_score = tech + trial_tally.nested * 4 + level6_undecided * 12 + (level7_undecided + trial_tally.conclusions) * 40;
        if status == Status::Stopped {
            difficulty_score += (unknown as u32) * 90;
        }
        (level - 1) as u64 * 1_000_000 + difficulty_score.min(999999) as u64
    };

    // 解数
    let solutions = match status {
        Status::Completed => SolutionCount::Unique,
        Status::Stopped => solution::count(&board, &state),
        Status::Broken => SolutionCount::Empty,
    };

    // 难度分：solutions "0"/"2+" → null；否则对 dp 插值
    let rating_hundredths = match solutions {
        SolutionCount::Empty | SolutionCount::Multiple => None,
        SolutionCount::Unique | SolutionCount::Unknown => Some(rating_from_dp(dp)),
    };

    LpResult {
        status,
        solutions,
        dp,
        level,
        unknown,
        rating_hundredths,
        difficulty_version: 1,
    }
}
