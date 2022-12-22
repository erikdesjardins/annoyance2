use crate::state::State;
use std::collections::VecDeque;
use std::io;
use tui::backend::Backend;
use tui::buffer::Buffer;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::symbols::Marker;
use tui::text::Span;
use tui::widgets::{Axis, Block, Borders, Chart, Dataset, Widget};
use tui::{Frame, Terminal};

#[derive(Copy, Clone)]
pub enum Redraw {
    Yes,
    No,
}

pub fn draw(state: &State, terminal: &mut Terminal<impl Backend>) -> Result<(), io::Error> {
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(f.size());

        draw_charts(state, f, chunks[0]);
        draw_log(state, f, chunks[1]);
    })?;

    Ok(())
}

fn draw_charts(state: &State, f: &mut Frame<impl Backend>, area: Rect) {
    let charts = state.charts();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Ratio(1, charts.len() as u32);
            charts.len()
        ])
        .split(area);

    for (chart, area) in charts.zip(chunks) {
        let dataset = Dataset::default()
            .name(&chart.series_name)
            .marker(Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .data(&chart.coords);

        let chart = Chart::new(vec![dataset])
            .block(
                Block::default()
                    .title(Span::styled(
                        &chart.chart_name,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title(chart.x_axis.name.as_str())
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::styled(
                            chart.x_axis.range.min_name(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            chart.x_axis.range.max_name(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ])
                    .bounds([chart.x_axis.range.min(), chart.x_axis.range.max()]),
            )
            .y_axis(
                Axis::default()
                    .title(chart.y_axis.name.as_str())
                    .style(Style::default().fg(Color::Gray))
                    .labels(vec![
                        Span::styled(
                            chart.y_axis.range.min_name(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            chart.y_axis.range.max_name(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                    ])
                    .bounds([chart.y_axis.range.min(), chart.y_axis.range.max()]),
            );

        f.render_widget(chart, area);
    }
}

fn draw_log(state: &State, f: &mut Frame<impl Backend>, area: Rect) {
    struct LogWidget<'a> {
        logs: &'a VecDeque<String>,
    }

    impl<'a> Widget for LogWidget<'a> {
        fn render(self, area: Rect, buf: &mut Buffer) {
            let block = Block::default()
                .title(Span::styled(
                    "Log",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL);

            let area = {
                let a = block.inner(area);
                block.render(area, buf);
                a
            };

            for (i, log) in self
                .logs
                .iter()
                .rev()
                .take(area.height as usize)
                .enumerate()
            {
                buf.set_string(
                    area.left(),
                    area.bottom() - 1 - i as u16,
                    log,
                    Style::default(),
                );
            }
        }
    }

    f.render_widget(LogWidget { logs: state.logs() }, area);
}
