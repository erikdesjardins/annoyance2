use crate::state::State;
use crate::terminal::Redraw;
use std::num::ParseIntError;

pub fn handle_line(state: &mut State, line: &str) -> Redraw {
    enum ParseError<'a> {
        NotACommand,
        MissingChartId,
        MissingCommand,
        InvalidCommand(&'a str),
        MissingDelimiter(char),
        MissingArgument,
        InvalidInt(ParseIntError),
    }

    impl From<ParseIntError> for ParseError<'_> {
        fn from(e: ParseIntError) -> Self {
            Self::InvalidInt(e)
        }
    }

    fn drop_eol(s: &str) -> &str {
        if let Some(s) = s.strip_suffix("\r\n") {
            return s;
        }
        if let Some(s) = s.strip_suffix('\n') {
            return s;
        }
        if let Some(s) = s.strip_suffix('\r') {
            return s;
        }
        s
    }

    fn parse_command<'a>(state: &mut State, line: &'a str) -> Result<Redraw, ParseError<'a>> {
        // Drop EOL
        let line = drop_eol(line);

        // Parse prefix
        let Some(line) = line.strip_prefix(".vz ") else { return Err(ParseError::NotACommand) };

        // Parse chart id
        let Some((chart_id, line)) = line.split_once(' ') else { return Err(ParseError::MissingChartId) };
        let chart_id = chart_id.parse()?;
        let mut chart = state.get_or_create_chart(chart_id);

        // Parse command
        let Some((command, args)) = line.split_once(' ') else { return Err(ParseError::MissingCommand) };

        Ok(match command {
            // Hottest commands first:
            "xs" => {
                // Must provide X coords first
                chart.coords.clear();

                let mut min_x = chart.x_axis.range.min();
                let mut max_x = chart.x_axis.range.max();

                let Some(args) = args.strip_prefix('[') else { return Err(ParseError::MissingDelimiter('[')) };
                let Some(args) = args.strip_suffix(']') else { return Err(ParseError::MissingDelimiter(']')) };
                for arg in args.split(", ") {
                    let x: i32 = arg.parse()?;
                    let x = f64::from(x);
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    chart.coords.push((x, 0.));
                }

                chart.x_axis.range.set_min(min_x);
                chart.x_axis.range.set_max(max_x);

                Redraw::No
            }
            "ys" => {
                // Must provide Y coords second
                let coords = chart.coords.iter_mut();

                let mut min_y = chart.y_axis.range.min();
                let mut max_y = chart.y_axis.range.max();

                let Some(args) = args.strip_prefix('[') else { return Err(ParseError::MissingDelimiter('[')) };
                let Some(args) = args.strip_suffix(']') else { return Err(ParseError::MissingDelimiter(']')) };
                for (arg, coord) in args.split(", ").zip(coords) {
                    let y: i32 = arg.parse()?;
                    let y = f64::from(y);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    coord.1 = y;
                }

                chart.y_axis.range.set_min(min_y);
                chart.y_axis.range.set_max(max_y);

                Redraw::Yes
            }
            // Remaining (cold) alternatives:
            "cn" => {
                chart.chart_name = args.to_string();
                Redraw::Yes
            }
            "sn" => {
                chart.series_name = args.to_string();
                Redraw::Yes
            }
            "xn" => {
                chart.x_axis.name = args.to_string();
                Redraw::Yes
            }
            "yn" => {
                chart.y_axis.name = args.to_string();
                Redraw::Yes
            }
            "xr" => {
                let Some((min, max)) = args.split_once(' ') else { return Err(ParseError::MissingArgument) };
                let min: i32 = min.parse()?;
                let max: i32 = max.parse()?;
                chart.x_axis.range.set_min(f64::from(min));
                chart.x_axis.range.set_max(f64::from(max));
                Redraw::Yes
            }
            "yr" => {
                let Some((min, max)) = args.split_once(' ') else { return Err(ParseError::MissingArgument) };
                let min: i32 = min.parse()?;
                let max: i32 = max.parse()?;
                chart.y_axis.range.set_min(f64::from(min));
                chart.y_axis.range.set_max(f64::from(max));
                Redraw::Yes
            }
            _ => return Err(ParseError::InvalidCommand(command)),
        })
    }

    match parse_command(state, line) {
        Ok(r) => r,
        Err(e) => {
            state.push_log(line);
            match e {
                ParseError::NotACommand => {}
                ParseError::MissingChartId => state.push_log("Error: missing chart id"),
                ParseError::MissingCommand => state.push_log("Error: missing command"),
                ParseError::InvalidCommand(cmd) => {
                    state.push_log(&format!("Error: invalid command `{}`", cmd))
                }
                ParseError::MissingDelimiter(delim) => {
                    state.push_log(&format!("Error: missing delimiter `{}`", delim))
                }
                ParseError::MissingArgument => state.push_log("Error: missing argument"),
                ParseError::InvalidInt(e) => state.push_log(&format!("Error: {:?}", e)),
            }
            Redraw::Yes
        }
    }
}
