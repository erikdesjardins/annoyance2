use crate::config;
use std::collections::VecDeque;
use std::fmt::Write;

pub struct State {
    pub chart_name: String,
    pub series_name: String,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub coords: Vec<(f64, f64)>,
    logs: VecDeque<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            chart_name: Default::default(),
            series_name: Default::default(),
            x_axis: Default::default(),
            y_axis: Default::default(),
            coords: Default::default(),
            logs: VecDeque::with_capacity(config::SCROLLBACK_LINES),
        }
    }
}

impl State {
    pub fn logs(&self) -> &VecDeque<String> {
        &self.logs
    }

    pub fn push_log(&mut self, log: String) {
        if self.logs.len() >= config::SCROLLBACK_LINES {
            self.logs.pop_front();
        }
        self.logs.push_back(log);
    }
}

#[derive(Default)]
pub struct Axis {
    pub name: String,
    pub range: AxisRange,
}

pub struct AxisRange {
    min: f64,
    min_name: String,
    max: f64,
    max_name: String,
}

impl Default for AxisRange {
    fn default() -> Self {
        Self::new(0., 0.)
    }
}

impl AxisRange {
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            min_name: min.to_string(),
            max,
            max_name: max.to_string(),
        }
    }

    pub fn min(&self) -> f64 {
        self.min
    }

    pub fn min_name(&self) -> &str {
        &self.min_name
    }

    pub fn max(&self) -> f64 {
        self.max
    }

    pub fn max_name(&self) -> &str {
        &self.max_name
    }

    pub fn set_min(&mut self, min: f64) {
        if min != self.min {
            self.min = min;
            self.min_name.clear();
            write!(self.min_name, "{}", min).unwrap();
        }
    }

    pub fn set_max(&mut self, max: f64) {
        if max != self.max {
            self.max = max;
            self.max_name.clear();
            write!(self.max_name, "{}", max).unwrap();
        }
    }
}
