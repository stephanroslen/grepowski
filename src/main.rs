use crate::tui::TuiEvent;

mod fragment;
mod args;
mod ai_query;
mod tui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = args::parse();

    println!("{:?}", args);

    let system_prompt = "You are an evaluation model. Respond only with a floating point number from 0 to 1 with 3 decimal places. It should represent the probability that the question following in the system prompt is true for the code in the user prompt.";

    let ai = ai_query::AI::new(args.model, args.url, args.temperature, 10, system_prompt, args.question);

    let fragments = args
        .files
        .iter()
        .flat_map(|file| -> anyhow::Result<Vec<fragment::Fragment>> {
            fragment::file_to_fragments(file, args.lines_per_block, args.blocks_per_fragment)
        })
        .flatten()
        .collect::<Vec<_>>();

    let (tx_tui, rx_tui) = tokio::sync::mpsc::channel(2);
    let tui = tokio::spawn(tui::run(rx_tui, fragments.len(), std::time::Duration::from_millis(50)));

    for fragment in &fragments {
        tx_tui.send(TuiEvent::GatherNextFragment(fragment.clone())).await?;
        let x = ai.query(&fragment.content).await?;
        tx_tui.send(TuiEvent::GatherNextValue(x)).await?;
        tx_tui.send(TuiEvent::GatherIncrementCount).await?;
    }

    tx_tui.send(TuiEvent::SwitchToDisplayData).await?;

    tx_tui.send(TuiEvent::Quit).await?;

    tui.await??;

    Ok(())
}
