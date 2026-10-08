use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to bind to
    #[arg(short, long, default_value_t = 4567)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    println!("Suwayomi Server starting up on port {}...", args.port);

    Ok(())
}
