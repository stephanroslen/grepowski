use crate::{fragment::Fragment, fragment_evaluation::FragmentEvaluation};
use ratatui::{
    layout::{Constraint, Direction},
    style::{Color, Styled},
    symbols::Marker,
    widgets::{
        Axis, Block, BorderType, Chart, Dataset, Gauge, ListItem, ListState, Paragraph, Wrap,
    },
    {
        DefaultTerminal, Frame,
        style::{Stylize, palette::tailwind},
    },
};
use std::collections::VecDeque;
use tokio::select;

const COLOR_TITLE: Color = tailwind::AMBER.c50;
const COLOR_HIGHLIGHT: Color = tailwind::AMBER.c100;
const COLOR_TEXT: Color = tailwind::AMBER.c200;
const COLOR_BORDER: Color = tailwind::AMBER.c800;
const COLOR_BACKGROUND: Color = tailwind::BLACK;

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
struct DisplayDataState {
    eval: Vec<FragmentEvaluation>,
    current_idx: usize,
    list_state: ListState,
}

impl DisplayDataState {
    fn new(eval: Vec<FragmentEvaluation>) -> Self {
        let current_idx = 0;
        let list_state = ListState::default();
        Self {
            eval,
            current_idx,
            list_state,
        }
    }
}

#[derive(Debug, Clone)]
enum TuiState {
    GatherData(GatherDataState),
    DisplayData(DisplayDataState),
}

impl TuiState {
    fn new(count_max: usize) -> Self {
        Self::GatherData(GatherDataState::new(count_max))
    }

    fn render(&mut self, frame: &mut Frame) {
        match self {
            TuiState::GatherData(state) => {
                Self::render_gather_data(frame, state);
            }
            TuiState::DisplayData(state) => {
                Self::render_display_data(frame, state);
            }
        }
    }

    fn render_display_data(frame: &mut Frame, state: &mut DisplayDataState) {
        let items_strings = state
            .eval
            .iter()
            .map(|e| {
                format!(
                    "{}:{} {:.3}",
                    e.fragment.path.to_str().unwrap(),
                    e.fragment.first_line,
                    e.value
                )
            })
            .collect::<Vec<_>>();
        let max_len = items_strings.iter().map(|s| s.len()).max().unwrap_or(0);

        let layout = ratatui::layout::Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(max_len as u16 + 2)].as_ref())
            .split(frame.area());

        let code = Self::make_code(state.eval.get(state.current_idx).map(|e| &e.fragment));

        frame.render_widget(code, layout[0]);

        let items = items_strings.into_iter().map(ListItem::new);

        let list = ratatui::widgets::List::new(items)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .set_style(COLOR_BORDER)
                    .title("Fragments".set_style(COLOR_TITLE)),
            )
            .set_style(COLOR_TEXT)
            .highlight_style(COLOR_HIGHLIGHT)
            .bg(COLOR_BACKGROUND);

        state.list_state.select(Some(state.current_idx));

        frame.render_stateful_widget(list, layout[1], &mut state.list_state);
    }

    fn render_gather_data(frame: &mut Frame, state: &GatherDataState) {
        let layout = ratatui::layout::Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Fill(1),
                    Constraint::Length(4),
                    Constraint::Length(5),
                ]
                .as_ref(),
            )
            .split(frame.area());

        let current_fragment = state.current_fragment.as_ref();

        let code = Self::make_code(current_fragment);

        frame.render_widget(code, layout[0]);

        let data: Vec<_> = state
            .value_history
            .iter()
            .copied()
            .rev()
            .take((layout[1].width as usize - 2) * 2)
            .rev()
            .enumerate()
            .map(|(idx, val)| (idx as f64, val as f64))
            .collect();
        let data = vec![
            Dataset::default()
                .marker(Marker::Braille)
                .style(COLOR_TEXT)
                .data(&data),
        ];

        let chart = Chart::new(data)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title("Value history".set_style(COLOR_TITLE)),
            )
            .x_axis(
                Axis::default()
                    .style(COLOR_TEXT)
                    .bounds([0.0, (layout[1].width as f64 - 2.0) * 2.0 - 1.0]),
            )
            .y_axis(Axis::default().style(COLOR_TEXT).bounds([0.0, 1.0]))
            .style(COLOR_BORDER)
            .bg(COLOR_BACKGROUND);

        frame.render_widget(chart, layout[1]);

        frame.render_widget(
            Gauge::default()
                .gauge_style(COLOR_BORDER)
                .block(
                    Block::bordered()
                        .set_style(COLOR_BORDER)
                        .border_type(BorderType::Rounded)
                        .title("Progress".set_style(COLOR_TITLE)),
                )
                .ratio(state.count as f64 / state.count_max as f64)
                .label(format!("{}/{}", state.count, state.count_max).set_style(COLOR_TEXT))
                .use_unicode(true)
                .bg(COLOR_BACKGROUND),
            layout[2],
        );
    }

    fn make_code(current_fragment: Option<&Fragment>) -> Paragraph<'_> {
        match current_fragment {
            Some(fragment) => {
                let lines: Vec<_> = fragment
                    .content
                    .lines()
                    .map(|l| ratatui::text::Line::from(ratatui::text::Span::raw(l)))
                    .collect();
                let code = Paragraph::new(lines)
                    .set_style(COLOR_TEXT)
                    .wrap(Wrap { trim: false });
                let code = code
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .set_style(COLOR_BORDER)
                            .title(
                                format!(
                                    "{}:{}",
                                    fragment.path.to_str().unwrap(),
                                    fragment.first_line
                                )
                                .set_style(COLOR_TITLE),
                            ),
                    )
                    .bg(COLOR_BACKGROUND);
                code
            }
            None => Paragraph::new("").block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .set_style(COLOR_BORDER)
                    .title("Current code fragment".set_style(COLOR_TITLE))
                    .bg(COLOR_BACKGROUND),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Nav {
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Debug, Clone)]
pub enum TuiEvent {
    GatherNextFragment(Fragment),
    GatherNextValue(f32),
    GatherIncrementCount,
    SwitchToDisplayData(Vec<FragmentEvaluation>),
    Nav(Nav),
    Quit,
}

pub struct Tui {
    timer: tokio::time::Interval,
    tui_state: TuiState,
}

impl Tui {
    pub fn new(count_max: usize, refresh_interval: std::time::Duration) -> Self {
        let mut timer = tokio::time::interval(refresh_interval);
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let tui_state = TuiState::new(count_max);
        Self { timer, tui_state }
    }

    async fn main_loop(
        &mut self,
        mut rx: tokio::sync::mpsc::Receiver<TuiEvent>,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<()> {
        loop {
            select! {
                _ = self.timer.tick() => {
                        terminal.draw(|frame| self.tui_state.render(frame))?;
                    },
                event = rx.recv() => {
                    match event {
                        Some(TuiEvent::GatherNextFragment(fragment)) => {
                            let TuiState::GatherData(state) = &mut self.tui_state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                            state.current_fragment = Some(fragment);
                        },
                        Some(TuiEvent::GatherNextValue(value)) => {
                            let TuiState::GatherData(state) = &mut self.tui_state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                            state.value_history.push_back(value);
                        },
                        Some(TuiEvent::GatherIncrementCount) => {
                            let TuiState::GatherData(state) = &mut self.tui_state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                            state.count += 1;
                        },
                        Some(TuiEvent::SwitchToDisplayData(data)) => {
                            self.tui_state = TuiState::DisplayData(DisplayDataState::new(data));
                        }
                        Some(TuiEvent::Quit) | None => {
                            return Ok(())
                        },
                        Some(TuiEvent::Nav(nav)) => {
                            if let TuiState::DisplayData(state) = &mut self.tui_state {
                                match nav {
                                    Nav::Up => {
                                    state.current_idx = state.current_idx.saturating_sub(1);
                                    }
                                    Nav::Down => {
                                            state.current_idx = std::cmp::min(state.current_idx.saturating_add(1), state.eval.len() - 1);
                                        }
                                    Nav::PageUp | Nav::PageDown => {
                                        let items = terminal.get_frame().area().height as usize - 2;
                                            match nav {
                                                Nav::PageUp => state.current_idx = state.current_idx.saturating_sub(items),
                                                Nav::PageDown => state.current_idx = std::cmp::min(state.current_idx.saturating_add(items), state.eval.len() - 1),
                                                _ => unreachable!()
                                            }
                                    }
                                    Nav::Home => {
                                            state.current_idx = 0;
                                        }
                                    Nav::End => {
                                            state.current_idx = state.eval.len() - 1;
                                        }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn run(mut self, rx: tokio::sync::mpsc::Receiver<TuiEvent>) -> anyhow::Result<()> {
        let mut terminal = ratatui::init();

        let result = self.main_loop(rx, &mut terminal).await;

        ratatui::restore();

        result
    }
}
