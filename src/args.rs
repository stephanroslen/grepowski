use clap::Parser;

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(
        short,
        long,
        default_value = "10",
        env = "GREPOWSKI_LINES_PER_BLOCK",
        value_name = "LINES",
        help = "Number of lines per block"
    )]
    pub lines_per_block: usize,

    #[clap(
        short,
        long,
        default_value = "3",
        env = "GREPOWSKI_BLOCKS_PER_FRAGMENT",
        value_name = "BLOCKS",
        help = "Number of blocks per fragment"
    )]
    pub blocks_per_fragment: usize,

    #[clap(
        short,
        long,
        value_name = "MODEL",
        env = "GREPOWSKI_MODEL",
        help = "Model to use for the chat completion"
    )]
    pub model: String,

    #[clap(
        short,
        long,
        value_name = "TEMPERATURE",
        env = "GREPOWSKI_TEMPERATURE",
        default_value = "0.2",
        help = "Temperature for the chat completion"
    )]
    pub temperature: f32,

    #[clap(
        short,
        long,
        value_name = "URL",
        env = "GREPOWSKI_URL",
        default_value = "http://127.0.0.1:8080/v1/chat/completions",
        help = "URL of the chat completion endpoint",
        value_hint = clap::ValueHint::Url,
    )]
    pub url: String,

    #[clap(value_name = "QUESTION", help = "Question to ask the model")]
    pub question: String,

    #[clap(value_name = "FILES", required = true, help = "Input files to analyze", value_hint = clap::ValueHint::FilePath
    )]
    pub files: Vec<String>,
}

pub fn parse() -> Args {
    Args::parse()
}