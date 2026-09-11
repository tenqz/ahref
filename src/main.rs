use ahref::{
    export::{self, Format},
    CrawlOptions, Graph, Result,
};
use clap::{Args, Parser, Subcommand};
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::PathBuf,
    time::Duration,
};

#[derive(Parser)]
#[command(version, about = "Turn a website into a directed link graph")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Crawl internal HTML pages from a URL or sitemap (.xml / .xml.gz).
    Crawl(Box<CrawlArgs>),
    /// Recompute metrics from a versioned JSON graph, without network requests.
    Analyze {
        file: PathBuf,
        #[arg(long)]
        damping: Option<f64>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Convert a versioned JSON graph to JSON, GraphML or DOT.
    Export {
        file: PathBuf,
        #[arg(long, value_enum)]
        format: Format,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}
#[derive(Args)]
struct CrawlArgs {
    url: String,
    #[arg(long, default_value_t = 1000)]
    max_pages: usize,
    #[arg(long, default_value_t = 10)]
    max_depth: usize,
    #[arg(long, default_value_t = 5)]
    concurrency: usize,
    /// Timeout per HTTP request in seconds, including response body.
    #[arg(long, default_value_t = 20)]
    timeout: u64,
    #[arg(long, default_value = "ahref/1.0.0")]
    user_agent: String,
    #[arg(long)]
    include_subdomains: bool,
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    respect_robots: bool,
    /// Explicit opt-out from robots.txt rules.
    #[arg(long, conflicts_with = "respect_robots")]
    ignore_robots: bool,
    #[arg(long, default_value_t = 5242880)]
    max_body_bytes: usize,
    #[arg(long, default_value_t = 100)]
    max_sitemaps: usize,
    #[arg(long, default_value_t = 100000)]
    max_sitemap_urls: usize,
    #[arg(long, default_value_t = 0.85)]
    damping: f64,
    /// File to write; '-' writes the graph to stdout.
    #[arg(long)]
    output: Option<PathBuf>,
    /// With no output path, an explicit format writes the graph to stdout.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

fn read(path: &PathBuf) -> Result<Graph> {
    let graph: Graph = serde_json::from_reader(BufReader::new(File::open(path)?))?;
    graph.validate()?;
    Ok(graph)
}
fn save(graph: &Graph, format: Format, path: Option<&PathBuf>) -> Result<()> {
    if let Some(path) = path.filter(|p| p.as_os_str() != "-") {
        let mut writer = BufWriter::new(File::create(path)?);
        export::write(graph, format, &mut writer)?;
        writer.flush()?;
    } else {
        let mut out = io::stdout().lock();
        export::write(graph, format, &mut out)?;
        out.flush()?;
    }
    Ok(())
}
fn report(graph: &Graph, mut out: impl Write) -> Result<()> {
    let s = &graph.site.summary;
    writeln!(out, "Graph analysis completed\n")?;
    for (label, value) in [
        ("Pages (known)", s.pages),
        ("Fetched pages", s.fetched_pages),
        ("Internal links", s.internal_links),
        ("External links", s.external_links),
        ("External domains", s.external_domains),
        ("Broken links (checked)", s.broken_links),
        ("Redirects", s.redirects),
        ("Dead-end pages", s.dead_end_pages),
        ("Orphan candidates", s.orphan_pages),
        ("Weakly-linked pages", s.weakly_linked_pages),
        ("Pending pages", s.pending_pages),
        ("Failed requests", s.failed_pages),
        ("Robots-blocked pages", s.blocked_pages),
        ("Max depth", s.max_depth),
    ] {
        writeln!(out, "{label:<25} {value}")?;
    }
    writeln!(
        out,
        "Average reachable depth   {:.2}\n\nTop pages by internal PageRank (sum = 1):",
        s.average_depth
    )?;
    let mut ranked: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.pagerank.is_some())
        .collect();
    ranked.sort_by(|a, b| {
        b.pagerank
            .unwrap_or(0.0)
            .total_cmp(&a.pagerank.unwrap_or(0.0))
            .then_with(|| a.url.as_str().cmp(b.url.as_str()))
    });
    for (i, n) in ranked.iter().take(10).enumerate() {
        writeln!(out, "{}. {} {:.6}", i + 1, n.url, n.pagerank.unwrap_or(0.0))?;
    }
    for warning in &graph.site.warnings {
        writeln!(out, "Warning: {warning}")?;
    }
    Ok(())
}
async fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Crawl(args) => {
            let graph = ahref::crawl(
                &args.url,
                CrawlOptions {
                    max_pages: args.max_pages,
                    max_depth: args.max_depth,
                    concurrency: args.concurrency,
                    timeout: Duration::from_secs(args.timeout),
                    user_agent: args.user_agent,
                    include_subdomains: args.include_subdomains,
                    respect_robots: args.respect_robots && !args.ignore_robots,
                    max_body_bytes: args.max_body_bytes,
                    max_sitemaps: args.max_sitemaps,
                    max_sitemap_urls: args.max_sitemap_urls,
                    damping: args.damping,
                },
            )
            .await?;
            if args.output.is_some() || args.format.is_some() {
                save(
                    &graph,
                    args.format.unwrap_or(Format::Json),
                    args.output.as_ref(),
                )?;
                report(&graph, io::stderr().lock())?;
            } else {
                report(&graph, io::stdout().lock())?;
            }
        }
        Command::Analyze {
            file,
            damping,
            output,
        } => {
            let mut graph = read(&file)?;
            if let Some(d) = damping {
                graph.site.damping = d;
            }
            graph.analyze()?;
            if output.is_some() {
                save(&graph, Format::Json, output.as_ref())?;
                report(&graph, io::stderr().lock())?;
            } else {
                report(&graph, io::stdout().lock())?;
            }
        }
        Command::Export {
            file,
            format,
            output,
        } => {
            let mut graph = read(&file)?;
            graph.analyze()?;
            save(&graph, format, output.as_ref())?;
        }
    }
    Ok(())
}
#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(ahref::Error::Io(e)) if e.kind() == io::ErrorKind::BrokenPipe => {
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ahref: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
