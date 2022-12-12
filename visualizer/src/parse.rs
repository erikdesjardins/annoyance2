use crate::{state::State, terminal::Redraw};
use std::num::ParseFloatError;

pub fn handle_line(state: &mut State, line: &str) -> Redraw {
    enum ParseError {
        NotACommand,
        InvalidFloat(ParseFloatError),
    }

    impl From<ParseFloatError> for ParseError {
        fn from(e: ParseFloatError) -> Self {
            Self::InvalidFloat(e)
        }
    }

    fn parse_command(state: &mut State, line: &str) -> Result<Redraw, ParseError> {
        let example_command = ".vz xx ";

        let line = line.trim_end_matches('\n');
        let line = line.trim_end_matches('\r');

        if line.len() < example_command.len() {
            return Err(ParseError::NotACommand);
        }

        let (cmd, arguments) = line.split_at(example_command.len());
        match cmd {
            ".vz cn " => {
                state.chart_name = arguments.to_string();
                Ok(Redraw::Yes)
            }
            ".vz sn " => {
                state.series_name = arguments.to_string();
                Ok(Redraw::Yes)
            }
            ".vz xn " => {
                state.x_axis.name = arguments.to_string();
                Ok(Redraw::Yes)
            }
            ".vz yn " => {
                state.y_axis.name = arguments.to_string();
                Ok(Redraw::Yes)
            }
            ".vz xs " => {
                // Must provide X coords first
                state.coords.clear();

                let arguments = arguments.trim_start_matches('[');
                let arguments = arguments.trim_end_matches(']');
                let xs = arguments.split(',');

                let mut min_x = state.x_axis.range.min();
                let mut max_x = state.x_axis.range.max();

                for x in xs {
                    let x = x.trim_start_matches(' ').parse()?;

                    min_x = min_x.min(x);
                    max_x = max_x.max(x);

                    state.coords.push((x, 0.));
                }

                state.x_axis.range.set_min(min_x);
                state.x_axis.range.set_max(max_x);

                Ok(Redraw::No)
            }
            ".vz ys " => {
                // Must provide Y coords second

                let arguments = arguments.trim_start_matches('[');
                let arguments = arguments.trim_end_matches(']');
                let ys = arguments.split(',');

                let mut min_y = state.y_axis.range.min();
                let mut max_y = state.y_axis.range.max();

                for (y, (_, old_y)) in ys.zip(state.coords.iter_mut()) {
                    let y = y.trim_start_matches(' ').parse()?;

                    min_y = min_y.min(y);
                    max_y = max_y.max(y);

                    *old_y = y;
                }

                state.y_axis.range.set_min(min_y);
                state.y_axis.range.set_max(max_y);

                Ok(Redraw::Yes)
            }
            _ => Err(ParseError::NotACommand),
        }
    }

    match parse_command(state, line) {
        Ok(r) => r,
        Err(e) => {
            state.push_log(line.to_string());
            match e {
                ParseError::NotACommand => {}
                ParseError::InvalidFloat(e) => state.push_log(format!("Error: {:?}", e)),
            }
            Redraw::Yes
        }
    }
}
