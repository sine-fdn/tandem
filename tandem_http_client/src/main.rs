#![cfg(not(target_arch = "wasm32"))]

use anyhow::Context;
use clap::Parser;
use std::{io::Read, path::PathBuf};
use tandem_http_client::{compute, MpcData, MpcProgram};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[arg(value_parser, help = "Path to a Garble program file")]
    program: PathBuf,

    #[arg(
        long,
        required(true),
        help = "Name of the Garble function to be executed"
    )]
    function: String,

    #[arg(
        long,
        default_value = "https://echo-server.sine.dev",
        help = "Base URL of a remote tandem http server. "
    )]
    url: url::Url,

    #[arg(
        long,
        required(true),
        help = "Garble input literal for this (local) party"
    )]
    input: String,

    #[arg(
        long,
        required(true),
        help = "Metadata to send to the server (as plaintext) to influence the server's input"
    )]
    metadata: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let path = &cli.program;

    let mut source_code = String::new();
    std::fs::File::open(path)
        .with_context(|| format!("Could not open file `{}`", path.display()))?
        .read_to_string(&mut source_code)
        .with_context(|| format!("Could not read file `{}`", path.display()))?;

    let program = MpcProgram::new(source_code, cli.function)
        .with_context(|| "Not a valid 2-Party Garble program".to_string())?;
    let input = MpcData::from_string(&program, cli.input)
        .with_context(|| "Not a valid Garble input".to_string())?;

    let result = compute(cli.url.to_string(), cli.metadata, program, input).await?;
    println!("{}", result.to_literal_string());
    Ok(())
}
