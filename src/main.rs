use crate::{
    ai_query::AI,
    fragment::Fragment,
    fragment_evaluation::FragmentEvaluation,
    tui::{Nav, TuiEvent},
};
use crossterm::event::KeyEventKind;
use futures_util::{FutureExt, StreamExt};
use tokio::{select, sync::mpsc::Sender};

mod ai_query;
mod args;
mod fragment;
mod fragment_evaluation;
mod tui;

async fn gather_data(
    fragments: impl AsRef<[Fragment]>,
    tx_tui: &Sender<TuiEvent>,
    ai: AI,
) -> anyhow::Result<Vec<FragmentEvaluation>> {
    let mut eval = Vec::new();
    for fragment in fragments.as_ref() {
        tx_tui
            .send(TuiEvent::GatherNextFragment(fragment.clone()))
            .await?;
        tx_tui.send(TuiEvent::Render).await?;
        let value = ai.query(&fragment.content).await?;
        tx_tui.send(TuiEvent::GatherNextValue(value)).await?;
        tx_tui.send(TuiEvent::GatherIncrementCount).await?;
        eval.push(FragmentEvaluation {
            fragment: fragment.clone(),
            value,
        });
    }
    tx_tui.send(TuiEvent::Render).await?;

    eval.sort_by(|a, b| b.value.partial_cmp(&a.value).expect("Order expected"));

    Ok(eval)
}

async fn finish(eval: Vec<FragmentEvaluation>, tx_tui: &Sender<TuiEvent>) -> anyhow::Result<()> {
    tx_tui.send(TuiEvent::SwitchToDisplayData(eval)).await?;
    tx_tui.send(TuiEvent::Render).await?;
    Ok(())
}

async fn main_flow(
    fragments: impl AsRef<[Fragment]>,
    tx_tui: &Sender<TuiEvent>,
    ai: AI,
) -> anyhow::Result<()> {
    finish(gather_data(fragments, tx_tui, ai).await?, tx_tui).await
}

async fn input_and_process(
    fragments: impl AsRef<[Fragment]>,
    tx_tui: &Sender<TuiEvent>,
    ai: AI,
) -> anyhow::Result<()> {
    let main = main_flow(fragments, tx_tui, ai).fuse();
    let input = process_input(tx_tui);

    futures::pin_mut!(main, input);
    let input_result = loop {
        select! {
            main_result = &mut main => {
                // when main is done, we must still wait for input to finish
                main_result?;
            },
            input_result = &mut input => {
                // when input is done, we can return
                break input_result;
            }
        }
    };
    tx_tui.send(TuiEvent::Quit).await?;
    input_result
}

async fn process_input(tx_tui: &Sender<TuiEvent>) -> anyhow::Result<()> {
    enum RenderDecision {
        DoRender,
        DontRender,
    }

    let mut reader = crossterm::event::EventStream::new();

    loop {
        match reader.next().await {
            Some(Ok(event)) => match event {
                crossterm::event::Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        let render_decision = match key.code {
                            crossterm::event::KeyCode::Char('q')
                            | crossterm::event::KeyCode::Esc => {
                                break;
                            }
                            crossterm::event::KeyCode::Up => {
                                tx_tui.send(TuiEvent::Nav(Nav::Up)).await?;
                                RenderDecision::DoRender
                            }
                            crossterm::event::KeyCode::Down => {
                                tx_tui.send(TuiEvent::Nav(Nav::Down)).await?;
                                RenderDecision::DoRender
                            }
                            crossterm::event::KeyCode::PageUp => {
                                tx_tui.send(TuiEvent::Nav(Nav::PageUp)).await?;
                                RenderDecision::DoRender
                            }
                            crossterm::event::KeyCode::PageDown => {
                                tx_tui.send(TuiEvent::Nav(Nav::PageDown)).await?;
                                RenderDecision::DoRender
                            }
                            crossterm::event::KeyCode::Home => {
                                tx_tui.send(TuiEvent::Nav(Nav::Home)).await?;
                                RenderDecision::DoRender
                            }
                            crossterm::event::KeyCode::End => {
                                tx_tui.send(TuiEvent::Nav(Nav::End)).await?;
                                RenderDecision::DoRender
                            }
                            _ => RenderDecision::DontRender,
                        };
                        if matches!(render_decision, RenderDecision::DoRender) {
                            tx_tui.send(TuiEvent::Render).await?;
                        };
                    }
                }
                crossterm::event::Event::Resize(_, _) => {
                    tx_tui.send(TuiEvent::Render).await?;
                }
                _ => {}
            },
            Some(Err(e)) => {
                return Err(e.into());
            }
            None => {
                break;
            }
        }
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = args::parse();

    println!("{:?}", args);

    let system_prompt = "You are an evaluation model. Output only a floating point number in the range 0 to 1 with exactly three decimal places. The number must measure how strongly the question stated in the system prompt applies to the code provided in the user prompt. Use the scale as follows: 0.000 → the statement is entirely false for the code. 0.250 → weak indication. 0.500 → partially true / ambiguous. 0.750 → strongly supported. 1.000 → fully and unambiguously true. Do not use only extreme values. Spread your outputs across the full range when appropriate. Do not default to the given numbers. Interpolate according to your certainty between them. Use intermediate values whenever the evidence is partial or suggestive. Respond with the number only.";

    let ai = ai_query::AI::new(
        args.model,
        args.url,
        args.temperature,
        10,
        system_prompt,
        args.question,
    );

    let fragments = args
        .files
        .iter()
        .flat_map(|file| -> anyhow::Result<Vec<fragment::Fragment>> {
            fragment::file_to_fragments(file, args.lines_per_block, args.blocks_per_fragment)
        })
        .flatten()
        .collect::<Vec<_>>();

    let (tx_tui, rx_tui) = tokio::sync::mpsc::channel(8);
    let tui = tokio::spawn(tui::Tui::new(fragments.len()).run(rx_tui));

    let result = input_and_process(fragments, &std::convert::identity(tx_tui), ai).await;

    tui.await??;

    result
}
