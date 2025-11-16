use crate::{
    ai_query::AI, fragment::Fragment, fragment_evaluation::FragmentEvaluation, tui::TuiEvent,
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
        let value = ai.query(&fragment.content).await?;
        tx_tui.send(TuiEvent::GatherNextValue(value)).await?;
        tx_tui.send(TuiEvent::GatherIncrementCount).await?;
        eval.push(FragmentEvaluation {
            fragment: fragment.clone(),
            value,
        });
    }

    eval.sort_by(|a, b| b.value.partial_cmp(&a.value).expect("Order expected"));

    Ok(eval)
}

async fn finish(eval: Vec<FragmentEvaluation>, tx_tui: &Sender<TuiEvent>) -> anyhow::Result<()> {
    tx_tui.send(TuiEvent::SwitchToDisplayData(eval)).await?;
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
    loop {
        select! {
            main_result = &mut main => {
                // when main is done, we must still wait for input to finish
                main_result?;
            },
            input_result = &mut input => {
                // when input is done, we can return
                return input_result;
            }
        }
    }
}

async fn process_input(tx_tui: &Sender<TuiEvent>) -> anyhow::Result<()> {
    let mut reader = crossterm::event::EventStream::new();

    loop {
        match reader.next().await {
            Some(Ok(event)) => match event {
                crossterm::event::Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            crossterm::event::KeyCode::Char('q')
                            | crossterm::event::KeyCode::Esc => {
                                tx_tui.send(TuiEvent::Quit).await?;
                                break;
                            }
                            crossterm::event::KeyCode::Up => {
                                tx_tui.send(TuiEvent::Up).await?;
                            }
                            crossterm::event::KeyCode::Down => {
                                tx_tui.send(TuiEvent::Down).await?;
                            }
                            _ => {}
                        }
                    }
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

    let system_prompt = "You are an evaluation model. Respond only with a floating point number from 0 to 1 with 3 decimal places. It should represent the probability that the question following in the system prompt is true for the code in the user prompt.";

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

    let (tx_tui, rx_tui) = tokio::sync::mpsc::channel(2);
    let tui = tokio::spawn(
        tui::Tui::new(fragments.len(), std::time::Duration::from_millis(50)).run(rx_tui),
    );

    input_and_process(fragments, &std::convert::identity(tx_tui), ai).await?;

    tui.await??;

    Ok(())
}
