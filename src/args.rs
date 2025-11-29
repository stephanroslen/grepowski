use clap::{Args as ClapArgs, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(about = "Ask a question to the configured model")]
    Ask(AskArgs),
    #[command(about = "Generate shell completions")]
    Completions {
        #[clap(value_enum, help = "Shell to generate completions for")]
        shell: Shell,
    },
}

#[derive(ClapArgs, Debug)]
pub struct AskArgs {
    #[clap(
        short,
        long = "accessibility",
        help = "Use accessibility mode theme",
        env = "GREPOWSKI_ACCESSIBILITY_MODE",
        default_value = "false"
    )]
    pub accessibility_mode: bool,

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
        default_value = "0.0",
        help = "Temperature for the chat completion"
    )]
    pub temperature: f32,

    #[clap(
        short,
        long,
        value_name = "URL",
        env = "GREPOWSKI_URL",
        default_value = "http://127.0.0.1:8080/v1",
        help = "URL of the chat completion endpoint",
        value_hint = clap::ValueHint::Url,
    )]
    pub url: String,

    #[clap(
        short = 't',
        long,
        value_name = "TOKEN",
        env = "GREPOWSKI_AUTH_TOKEN",
        hide_env_values = true,
        help = "Bearer token for the chat completion endpoint - if not set, the model will be used anonymously"
    )]
    pub auth_token: Option<String>,

    #[clap(value_name = "QUESTION", help = "Question to ask the model")]
    pub question: String,

    #[clap(value_name = "FILES", required = true, help = "Input files to analyze", value_hint = clap::ValueHint::FilePath
    )]
    pub files: Vec<String>,
}

pub fn parse() -> Cli {
    Cli::parse()
}
