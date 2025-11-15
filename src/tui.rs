use crate::fragment::Fragment;
use ratatui::layout::{Constraint, Direction};
use ratatui::style::{Styled, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line};
use ratatui::widgets::{
    Axis, Block, BorderType, Borders, Chart, Dataset, Gauge, Padding, Paragraph, Wrap,
};
use ratatui::{Frame, style};
use std::collections::VecDeque;
use style::palette::tailwind;
use tokio::select;

#[derive(Debug, Clone)]
struct GatherDataState {
    value_history: VecDeque<f32>,
    current_fragment: Option<Fragment>,
    count: usize,
    count_max: usize,
}

impl GatherDataState {
    fn new(count_max: usize) -> Self {
        Self {
            value_history: VecDeque::new(),
            current_fragment: None,
            count: 0,
            count_max,
        }
    }
}

#[derive(Debug, Clone)]
enum TuiState {
    GatherData(GatherDataState),
    DisplayData,
}

fn title_block(title: &str) -> Block<'_> {
    let title = Line::from(title).centered();
    Block::new()
        .borders(Borders::NONE)
        .padding(Padding::vertical(1))
        .title(title)
        .fg(tailwind::AMBER.c50)
}

impl TuiState {
    fn new(count_max: usize) -> Self {
        Self::GatherData(GatherDataState::new(count_max))
    }

    fn render(&self, frame: &mut Frame) {
        match self {
            TuiState::GatherData(state) => {
                let layout = ratatui::layout::Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [Constraint::Fill(1), Constraint::Length(10), Constraint::Length(7)].as_ref(),
                    )
                    .split(frame.area());

                let code = match &state.current_fragment {
                    Some(fragment) => {
                        let lines: Vec<_> = fragment
                            .content
                            .lines()
                            .map(|l| ratatui::text::Line::from(ratatui::text::Span::raw(l)))
                            .collect();
                        let code = Paragraph::new(lines)
                            .style(tailwind::AMBER.c50)
                            .wrap(Wrap { trim: false });
                        let code = code.block(
                            Block::bordered().border_type(BorderType::Rounded).title(
                                format!(
                                    "{}:{}",
                                    fragment.path.to_str().unwrap(),
                                    fragment.first_line
                                )
                                .set_style(tailwind::AMBER.c800),
                            ),
                        );
                        code
                    }
                    None => Paragraph::new("").block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title("Current code fragment"),
                    ),
                };

                frame.render_widget(code, layout[0]);

                let data: Vec<_> = state
                    .value_history
                    .iter()
                    .copied()
                    .rev()
                    .take(layout[1].width as usize)
                    .rev()
                    .enumerate()
                    .map(|(idx, val)| (idx as f64, val as f64))
                    .collect();
                let data = vec![
                    Dataset::default()
                        .marker(Marker::Dot)
                        .style(tailwind::AMBER.c50)
                        .data(&data),
                ];

                let chart = Chart::new(data)
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title("Value history".set_style(tailwind::AMBER.c800)),
                    )
                    .x_axis(
                        Axis::default()
                            .style(tailwind::AMBER.c800)
                            .bounds([0.0, layout[1].width as f64]),
                    )
                    .y_axis(
                        Axis::default()
                            .style(tailwind::AMBER.c800)
                            .bounds([0.0, 1.0]),
                    )
                    .style(tailwind::AMBER.c50);

                frame.render_widget(chart, layout[1]);

                frame.render_widget(
                    Gauge::default()
                        .block(title_block("Progress"))
                        .gauge_style(tailwind::AMBER.c800)
                        .ratio(state.count as f64 / state.count_max as f64)
                        .label(
                            format!("{}/{}", state.count, state.count_max)
                                .set_style(tailwind::AMBER.c200),
                        )
                        .use_unicode(true),
                    layout[2],
                );
            }
            TuiState::DisplayData => {
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TuiEvent {
    GatherNextFragment(Fragment),
    GatherNextValue(f32),
    GatherIncrementCount,
    SwitchToDisplayData,
    Quit,
}

pub async fn run(
    mut rx: tokio::sync::mpsc::Receiver<TuiEvent>,
    count_max: usize,
    refresh_interval: std::time::Duration,
) -> anyhow::Result<()> {
    let mut timer = tokio::time::interval(refresh_interval);
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut tui_state = TuiState::new(count_max);

    let mut terminal = ratatui::init();

    let result = loop {
        select! {
            _ = timer.tick() => {
                match
                terminal.draw(|frame| tui_state.render(frame)) {
                    Ok(_) => {}
                    Err(e) => {
                        break Err(anyhow::Error::msg(e));
                    }
                }
            },
            event = rx.recv() => {
                match event {
                    Some(TuiEvent::GatherNextFragment(fragment)) => {
                        let TuiState::GatherData(state) = &mut tui_state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                        state.current_fragment = Some(fragment);
                    },
                    Some(TuiEvent::GatherNextValue(value)) => {
                        let TuiState::GatherData(state) = &mut tui_state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                        state.value_history.push_back(value);
                    },
                    Some(TuiEvent::GatherIncrementCount) => {
                        let TuiState::GatherData(state) = &mut tui_state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                        state.count += 1;
                    },
                    Some(TuiEvent::SwitchToDisplayData) => {
                        tui_state = TuiState::DisplayData;
                    }
                    Some(TuiEvent::Quit) => {
                        break Ok(())
                    },
                    None => {
                        break Err(anyhow::anyhow!("No event received!"))
                    }
                }
            }
        }
    };

    ratatui::restore();

    result
}
