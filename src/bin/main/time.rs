use crate::config;

pub type Instant = fugit::Instant<u32, 1, { config::clk::SYSCLK_HZ }>;
pub type Duration = fugit::Duration<u32, 1, { config::clk::SYSCLK_HZ }>;
pub type PulseDuration = fugit::Duration<u32, 1, { config::clk::TIM1CLK_HZ }>;
