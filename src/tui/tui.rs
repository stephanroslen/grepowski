use crate::tui::{FxFilter, Theme};
use crate::{fragment::Fragment, fragment_evaluation::FragmentEvaluation};
use ratatui::{
    layout::{Constraint, Direction, Margin},
    style::Styled,
    symbols::Marker,
    widgets::{
        Axis, Block, BorderType, Chart, Dataset, Gauge, ListItem, ListState, Paragraph, Wrap,
    },
    {DefaultTerminal, Frame, style::Stylize},
};
use std::{collections::VecDeque, time::Instant};
use tachyonfx::{EffectRenderer, color_from_hsl, color_to_hsl};
use tokio::{select, time::MissedTickBehavior};

const EFFECT_WIDTH: f32 = 50.0;
const EFFECT_STRENGTH: f32 = 50.0;
const EFFECT_MILLIS: u32 = 2500;
const EFFECT_DELAY_MILLIS: u32 = 7500;
const INITIAL_EFFECT_MILLIS: u32 = 500;
const INITIAL_EFFECT_DELAY_MILLIS: u32 = 4000;

const EXTRA_RENDER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(15);

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
enum TuiDeepState {
    GatherData(GatherDataState),
    DisplayData(DisplayDataState),
}

#[derive(Debug)]
struct TuiState {
    state: TuiDeepState,
    last_instant: Option<Instant>,
    effect: tachyonfx::Effect,
    fx_filter: FxFilter,
}

impl TuiState {
    fn new(count_max: usize) -> Self {
        let state = TuiDeepState::GatherData(GatherDataState::new(count_max));

        let last_instant = None;

        let effect = tachyonfx::fx::effect_fn(
            (),
            tachyonfx::EffectTimer::from_ms(EFFECT_MILLIS, tachyonfx::Interpolation::Linear),
            |_, context, cells| {
                let area = context.area;
                let diag_area_dim = (area.width + area.height) as f32;
                let diag_range_min = -EFFECT_WIDTH;
                let diag_range_max = diag_area_dim + EFFECT_WIDTH;
                let total_diag_range = diag_range_max - diag_range_min;
                let progress = context.alpha();

                let effect_width_rel = EFFECT_WIDTH / total_diag_range;

                for (position, cell) in cells {
                    let x_rel = position.x - area.x;
                    let y_rel = position.y - area.y;
                    let diag_pos = (x_rel + y_rel) as f32;

                    let pos_rel = (diag_pos - diag_range_min) / (diag_range_max - diag_range_min);

                    let diff = (progress - pos_rel).abs();

                    if diff < effect_width_rel {
                        let (h, s, mut l) = color_to_hsl(&cell.fg);
                        l += EFFECT_STRENGTH * (effect_width_rel - diff) / effect_width_rel;
                        cell.fg = color_from_hsl(h, s, l);
                    }
                }
            },
        )
        .reversed();

        let fx_filter = FxFilter::new(3);

        let effect = effect.with_filter(fx_filter.border_filter());

        let sleep = tachyonfx::fx::sleep(EFFECT_DELAY_MILLIS);
        let effect = tachyonfx::fx::sequence(&[effect, sleep]);
        let effect = tachyonfx::fx::repeating(effect);

        let initial_effect = tachyonfx::fx::coalesce(INITIAL_EFFECT_MILLIS);
        let sleep = tachyonfx::fx::sleep(INITIAL_EFFECT_DELAY_MILLIS);
        let initial_effect = tachyonfx::fx::sequence(&[initial_effect, sleep]);

        let initial_effect = initial_effect.with_filter(fx_filter.main_filter());

        let effect = tachyonfx::fx::sequence(&[initial_effect, effect]);

        Self {
            state,
            last_instant,
            effect,
            fx_filter,
        }
    }

    fn render(&mut self, frame: &mut Frame, theme: Theme) -> anyhow::Result<()> {
        self.fx_filter.reset();
        match self.state {
            TuiDeepState::GatherData(_) => {
                self.render_gather_data(frame, theme)?;
            }
            TuiDeepState::DisplayData(_) => {
                self.render_display_data(frame, theme)?;
            }
        }

        let now = Instant::now();
        let elapsed = self
            .last_instant
            .map_or(std::time::Duration::ZERO, |last| now - last)
            .into();
        self.last_instant = Some(now);

        if self.effect.running() {
            frame.render_effect(&mut self.effect, frame.area(), elapsed);
        }

        Ok(())
    }

    fn render_display_data(
        &mut self,
        frame: &mut Frame,
        theme: Theme,
    ) -> anyhow::Result<()> {
        let TuiDeepState::DisplayData(state) = &mut self.state else { anyhow::bail!("DisplayData state expected") };
        let items_strings = state
            .eval
            .iter()
            .map(|e| format!("{} {:.3}", e.fragment.location(), e.value))
            .collect::<Vec<_>>();
        let max_len = items_strings.iter().map(|s| s.len()).max().unwrap_or(0);

        let layout = ratatui::layout::Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(max_len as u16 + 2)].as_ref())
            .split(frame.area());

        for rect in layout.iter() {
            self.fx_filter.assign(rect.inner(Margin::new(1, 1)))?;
        }

        let code = Self::make_code(
            state.eval.get(state.current_idx).map(|e| &e.fragment),
            theme,
        );

        frame.render_widget(code, layout[0]);

        let items = items_strings.into_iter().map(ListItem::new);

        let list = ratatui::widgets::List::new(items)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .set_style(theme.border)
                    .title(" Fragments ".set_style(theme.title)),
            )
            .set_style(theme.text)
            .highlight_style(theme.highlight)
            .bg(theme.background);

        state.list_state.select(Some(state.current_idx));

        frame.render_stateful_widget(list, layout[1], &mut state.list_state);

        Ok(())
    }

    fn render_gather_data(
        &mut self,
        frame: &mut Frame,
        theme: Theme,
    ) -> anyhow::Result<()> {
        let TuiDeepState::GatherData(state) = &mut self.state else { anyhow::bail!("GatherData state expected") };
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

        for rect in layout.iter() {
            self.fx_filter.assign(rect.inner(Margin::new(1, 1)))?;
        }

        let current_fragment = state.current_fragment.as_ref();

        let code = Self::make_code(current_fragment, theme);

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
                .style(theme.text)
                .data(&data),
        ];

        let chart = Chart::new(data)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Value history ".set_style(theme.title)),
            )
            .x_axis(
                Axis::default()
                    .style(theme.text)
                    .bounds([0.0, (layout[1].width as f64 - 2.0) * 2.0 - 1.0]),
            )
            .y_axis(Axis::default().style(theme.text).bounds([0.0, 1.0]))
            .style(theme.border)
            .bg(theme.background);

        frame.render_widget(chart, layout[1]);

        frame.render_widget(
            Gauge::default()
                .gauge_style(theme.gauge)
                .block(
                    Block::bordered()
                        .set_style(theme.border)
                        .border_type(BorderType::Rounded)
                        .title(" Progress ".set_style(theme.title)),
                )
                .ratio(state.count as f64 / state.count_max as f64)
                .label(format!("{}/{}", state.count, state.count_max).set_style(theme.text))
                .use_unicode(true)
                .bg(theme.background),
            layout[2],
        );

        Ok(())
    }

    fn make_code(current_fragment: Option<&Fragment>, theme: Theme) -> Paragraph<'static> {
        match current_fragment {
            Some(fragment) => {
                let lines = fragment.highlighted_content();
                let code = Paragraph::new(lines).wrap(Wrap { trim: false });
                let code = code
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .set_style(theme.border)
                            .title(format!(" {} ", fragment.location()).set_style(theme.title)),
                    )
                    .bg(theme.background);
                code
            }
            None => Paragraph::new("").block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .set_style(theme.border)
                    .title(" Current code fragment ".set_style(theme.title))
                    .bg(theme.background),
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
    Render,
    GatherNextFragment(Fragment),
    GatherNextValue(f32),
    GatherIncrementCount,
    SwitchToDisplayData(Vec<FragmentEvaluation>),
    Nav(Nav),
    Quit,
}

#[derive(Debug)]
pub struct Tui {
    tui_state: TuiState,
    theme: Theme,
}

impl Tui {
    pub fn new(count_max: usize, theme: Theme) -> Self {
        let tui_state = TuiState::new(count_max);
        Self { tui_state, theme }
    }

    fn render(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        terminal.draw(|frame| {
            self.tui_state
                .render(frame, self.theme)
                .expect("Rendering expected")
        })?;

        Ok(())
    }

    async fn main_loop(
        &mut self,
        mut rx: tokio::sync::mpsc::Receiver<TuiEvent>,
        terminal: &mut DefaultTerminal,
    ) -> anyhow::Result<()> {
        let mut extra_render_timer = tokio::time::interval(EXTRA_RENDER_INTERVAL);
        extra_render_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            select! {
                _ = extra_render_timer.tick() => {
                    self.render(terminal)?;
                }
                event = rx.recv() => {
                    match event {
                        Some(TuiEvent::Render) => {
                            self.render(terminal)?;
                        },
                        Some(TuiEvent::GatherNextFragment(fragment)) => {
                            let TuiDeepState::GatherData(state) = &mut self.tui_state.state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                            state.current_fragment = Some(fragment);
                        },
                        Some(TuiEvent::GatherNextValue(value)) => {
                            let TuiDeepState::GatherData(state) = &mut self.tui_state.state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                            state.value_history.push_back(value);
                        },
                        Some(TuiEvent::GatherIncrementCount) => {
                            let TuiDeepState::GatherData(state) = &mut self.tui_state.state else { break Err(anyhow::anyhow!("GatherData state expected"))};
                            state.count += 1;
                        },
                        Some(TuiEvent::SwitchToDisplayData(data)) => {
                            self.tui_state.state = TuiDeepState::DisplayData(DisplayDataState::new(data));
                        }
                        Some(TuiEvent::Quit) | None => {
                            return Ok(())
                        },
                        Some(TuiEvent::Nav(nav)) => {
                            if let TuiDeepState::DisplayData(state) = &mut self.tui_state.state {
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
