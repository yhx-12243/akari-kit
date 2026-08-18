// answer_methods.rs — 技巧记录表
// 每次技巧应用记录一次「技巧 + 权重」；权重 = 该技巧本次覆盖的格子数。
// 技巧链每一轮开始时清空，最后存活的方法表决定难度分 tech = Σ(技巧分 × 权重)。

use std::collections::HashMap;

/// 点灯解法的技巧。每种技巧有固定分数（分数值对应难度计算）。
#[derive(Debug, Copy, Hash)]
#[cfg_attr(feature = "nightly", derive_const(Clone, PartialEq, Eq))]
#[cfg_attr(not(feature = "nightly"), derive(Clone, PartialEq, Eq))]
pub enum Technique {
    /// 灯照路径禁灯：灯的可照格不能再放灯。分 0
    LampBeam,
    /// 剩余刚好够数：数字墙的未决邻格数等于缺口 → 全部放灯。分 2
    JustEnough,
    /// 试置会造成无法照亮的格：该格放灯会留下照不到的格 → 禁灯。分 2
    WouldCreateUnlit,
    /// 数字周围情况分析：枚举数字墙邻格的放灯组合，求共同确定格。分 2
    ClueCaseAnalysis,
    /// 数字已放够：数字墙邻格灯数已达上限 → 其余禁灯。分 8
    ClueFull,
    /// 试置会造成数字不足：该格放灯会使某数字墙无法满足 → 禁灯。分 12
    WouldMakeShort,
    /// 光源情况分析：枚举未照亮格的候选光源，求共同确定格。分 24
    SourceCaseAnalysis,
    /// 唯一光源：未照亮格只剩一个可行放灯处 → 放灯。分 64
    OnlySource,
}

impl Technique {
    pub const fn score(self) -> u32 {
        match self {
            Self::LampBeam => 0,
            Self::JustEnough | Self::WouldCreateUnlit | Self::ClueCaseAnalysis => 2,
            Self::ClueFull => 8,
            Self::WouldMakeShort => 12,
            Self::SourceCaseAnalysis => 24,
            Self::OnlySource => 64,
        }
    }
}

#[derive(Default)]
pub struct Methods {
    weights: HashMap<Technique, u32>,
}

impl Methods {
    /// 记录一次技巧应用（同技巧去重，权重累加）。
    pub fn record(&mut self, tech: Technique) {
        *self.weights.entry(tech).or_insert(0) += 1;
    }

    /// tech = Σ 技巧分 × 权重。
    pub fn tech(&self) -> u32 {
        self.weights.iter().map(|(&t, &w)| t.score() * w).sum()
    }
}
