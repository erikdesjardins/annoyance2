use crate::config;
use std::collections::VecDeque;
use std::fmt::Write;

pub struct State {
    charts: Vec<(i32, Chart)>,
    logs: VecDeque<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            charts: Vec::new(),
            logs: VecDeque::with_capacity(config::SCROLLBACK_LINES),
        }
    }
}

impl State {
    pub fn charts(&self) -> impl ExactSizeIterator<Item = &Chart> {
        self.charts.iter().map(|(_, chart)| chart)
    }

    pub fn get_or_create_chart(&mut self, chart_id: i32) -> &mut Chart {
        match self.charts.iter().position(|(id, _)| *id == chart_id) {
            Some(i) => {
                // Found existing chart
                &mut self.charts[i].1
            }
            None => {
                // Insert new chart
                let i = self.charts.partition_point(|(id, _)| *id < chart_id);
                self.charts.insert(i, (chart_id, Chart::default()));
                &mut self.charts[i].1
            }
        }
    }

    pub fn logs(&self) -> &VecDeque<String> {
        &self.logs
    }

    pub fn push_log(&mut self, log: &str) {
        let log = if log.len() > config::MAX_LINE_LENGTH {
            &log[0..config::MAX_LINE_LENGTH]
        } else {
            log
        };

        let string = if self.logs.len() >= config::SCROLLBACK_LINES {
            // Reuse existing string allocation
            match self.logs.pop_front() {
                Some(mut existing_string) => {
                    existing_string.clear();
                    existing_string.push_str(log);
                    existing_string
                }
                None => unreachable!("cannot be empty"),
            }
        } else {
            // Not full yet, allocate new string
            log.to_string()
        };
        self.logs.push_back(string);
    }
}

#[derive(Default)]
pub struct Chart {
    pub chart_name: String,
    pub series_name: String,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub coords: Vec<(f64, f64)>,
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
