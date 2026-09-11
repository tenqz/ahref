use ahref::{
    crawl,
    export::{self, Format},
    graph::FetchState,
    CrawlOptions, Graph,
};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Server {
    root: String,
    hits: Arc<Mutex<BTreeMap<String, usize>>>,
    peak: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start(robots_fail: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let root = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let (task_hits, task_peak) = (hits.clone(), peak.clone());
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let (hits, active, peak) = (task_hits.clone(), active.clone(), task_peak.clone());
                tokio::spawn(async move {
                    let mut data = vec![0u8; 8192];
                    let n = socket.read(&mut data).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&data[..n]);
                    let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                    *hits.lock().unwrap().entry(path.clone()).or_default() += 1;
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let (status, mime, headers, body) = match path.as_str() {
                        "/robots.txt" if robots_fail => (503,"text/plain","","try later".to_owned()),
                        "/robots.txt" => (200,"text/plain","","User-agent: *\nDisallow: /private\nAllow: /private/open\nDisallow: /*secret$".to_owned()),
                        "/" => (200,"text/html","",include_str!("fixtures/index.html").to_owned()),
                        "/a" => (200,"text/html","","<a href='./b?q=1#fragment'>B</a><a href='/'>Home</a>".to_owned()),
                        "/b?q=1" => (200,"text/html","","<a href='../a'>A</a>".to_owned()),
                        "/redirect" => (302,"text/html","Location: /target\r\n","".into()),
                        "/broken-redirect-page" => (200,"text/html","","<a href='/broken-redirect'>Broken through redirect</a>".into()),
                        "/broken-redirect" => (302,"text/html","Location: /missing\r\n","".into()),
                        "/blocked-redirect" => (302,"text/html","Location: /private\r\n","".into()),
                        "/target" => (200,"text/html","","<p>dead end</p>".into()),
                        "/loop1" => (301,"text/html","Location: /loop2\r\n","".into()),
                        "/loop2" => (301,"text/html","Location: /loop1\r\n","".into()),
                        "/escape" => (302,"text/html","Location: https://outside.invalid/redirect\r\n","".into()),
                        "/private" | "/private/open" => (200,"text/html","","<a href='/'>Home</a>".into()),
                        "/orphan" => (200,"text/html","","<a href='/orphan'>Self-link is not an incoming path</a>".into()),
                        "/binary" | "/asset.pdf" => (200,"application/pdf","","%PDF-1.5".into()),
                        "/sitemap.xml" => (200,"application/xml","","<sitemapindex xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><sitemap><loc>/pages.xml</loc></sitemap><sitemap><loc>/sitemap.xml</loc></sitemap></sitemapindex>".into()),
                        "/pages.xml" | "/pages.xml.gz" => (200,"application/xml","","<urlset xmlns='http://www.sitemaps.org/schemas/sitemap/0.9'><url><loc>/</loc></url><url><loc>/orphan</loc></url><url><loc>/a</loc></url></urlset>".into()),
                        "/shortcuts" => (200,"text/html","","<a href='/shortcut-a'>A</a><a href='/shortcut-z'>Z</a>".into()),
                        "/shortcut-a" => (200,"text/html","","<a href='/shortcut-target'>long route</a>".into()),
                        "/shortcut-z" => (302,"text/html","Location: /shortcut-target\r\n","".into()),
                        "/shortcut-target" => (200,"text/html","","<a href='/shortcut-child'>Child</a>".into()),
                        "/shortcut-child" => (200,"text/html","","done".into()),
                        "/slow" => { tokio::time::sleep(Duration::from_millis(300)).await; (200,"text/html","","late".into()) },
                        "/large" => (200,"text/html","","x".repeat(4096)),
                        "/malformed.xml" => (200,"application/xml","","<urlset>".into()),
                        _ => (404,"text/html","","<p>missing</p>".into()),
                    };
                    let mut payload = body.into_bytes();
                    if path.ends_with(".gz") {
                        use std::io::Write;
                        let mut encoder = flate2::write::GzEncoder::new(
                            Vec::new(),
                            flate2::Compression::default(),
                        );
                        encoder.write_all(&payload).unwrap();
                        payload = encoder.finish().unwrap();
                    }
                    let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: {mime}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",payload.len());
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.write_all(&payload).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        Self {
            root,
            hits,
            peak,
            task,
        }
    }
    fn hits(&self, path: &str) -> usize {
        *self.hits.lock().unwrap().get(path).unwrap_or(&0)
    }
}
fn node<'a>(g: &'a Graph, suffix: &str) -> &'a ahref::PageNode {
    g.nodes
        .iter()
        .find(|n| n.url.as_str().ends_with(suffix))
        .unwrap()
}

#[tokio::test]
async fn full_graph_is_deterministic_and_bounded() {
    let s = Server::start(false).await;
    let opts = CrawlOptions {
        concurrency: 3,
        ..Default::default()
    };
    let g = crawl(&s.root, opts.clone()).await.unwrap();
    assert_eq!(node(&g, "/a").internal_incoming_links, 3);
    assert_eq!(node(&g, "/a").in_degree, 2);
    assert_eq!(node(&g, "/b?q=1").depth, Some(2));
    assert_eq!(node(&g, "/target").depth, Some(1));
    assert_eq!(node(&g, "/private").state, FetchState::BlockedRobots);
    assert_eq!(node(&g, "/binary").state, FetchState::SkippedResource);
    assert_eq!(node(&g, "/asset.pdf").state, FetchState::SkippedResource);
    assert_eq!(s.hits("/asset.pdf"), 0);
    assert_eq!(s.hits("/private"), 0);
    assert_eq!(s.hits("/robots.txt"), 1);
    assert_eq!(s.hits("/a"), 1);
    assert_eq!(s.hits("/loop1"), 1);
    assert_eq!(s.hits("/loop2"), 1);
    assert_eq!(g.site.summary.broken_links, 1);
    assert_eq!(g.site.summary.dead_end_pages, 1);
    assert_eq!(g.site.summary.external_domains, 1);
    assert!(g
        .nodes
        .iter()
        .filter(|n| n.kind == ahref::LinkType::External)
        .all(|n| n.status_code.is_none()));
    assert!((g.nodes.iter().filter_map(|n| n.pagerank).sum::<f64>() - 1.0).abs() < 1e-10);
    assert!(s.peak.load(Ordering::SeqCst) <= 3);
    assert!(s.peak.load(Ordering::SeqCst) >= 2);
    let serial = crawl(
        &s.root,
        CrawlOptions {
            concurrency: 1,
            ..opts
        },
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(&serial).unwrap()
    );
}

#[tokio::test]
async fn sitemap_orphans_are_not_zero_depth_pages() {
    let s = Server::start(false).await;
    let g = crawl(&format!("{}/sitemap.xml", s.root), CrawlOptions::default())
        .await
        .unwrap();
    assert_eq!(g.site.summary.orphan_pages, 1);
    assert!(node(&g, "/orphan").sitemap);
    assert_eq!(node(&g, "/orphan").depth, None);
    assert_eq!(s.hits("/sitemap.xml"), 1);
    assert_eq!(s.hits("/pages.xml"), 1);
    assert_eq!(node(&g, "/a").depth, Some(1));
}

#[tokio::test]
async fn limits_and_robots_opt_out() {
    let s = Server::start(false).await;
    let g = crawl(
        &s.root,
        CrawlOptions {
            max_pages: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(g.site.summary.fetched_pages, 1);
    assert!(g.site.summary.pending_pages > 0);
    assert_eq!(s.hits("/a"), 0);
    let g = crawl(
        &s.root,
        CrawlOptions {
            max_depth: 0,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(g.site.summary.fetched_pages, 1);
    assert_eq!(s.hits("/a"), 0);
    let g = crawl(
        &format!("{}/private", s.root),
        CrawlOptions {
            respect_robots: false,
            max_depth: 0,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(node(&g, "/private").status_code, Some(200));
    let g = crawl(
        &format!("{}/private/open", s.root),
        CrawlOptions {
            max_depth: 0,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(node(&g, "/private/open").status_code, Some(200));
}

#[tokio::test]
async fn unavailable_robots_and_external_redirects() {
    let s = Server::start(true).await;
    let g = crawl(&s.root, CrawlOptions::default()).await.unwrap();
    assert_eq!(s.hits("/"), 0);
    assert_eq!(g.site.summary.blocked_pages, 1);
    assert!(!g.site.warnings.is_empty());
    let s = Server::start(false).await;
    let g = crawl(&format!("{}/escape", s.root), CrawlOptions::default())
        .await
        .unwrap();
    assert_eq!(g.site.summary.redirects, 1);
    assert!(g
        .nodes
        .iter()
        .any(|n| n.kind == ahref::LinkType::External && n.status_code.is_none()));
}

#[tokio::test]
async fn network_and_document_failures_remain_in_report() {
    let s = Server::start(false).await;
    let g = crawl(
        &format!("{}/slow", s.root),
        CrawlOptions {
            respect_robots: false,
            timeout: Duration::from_millis(80),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(g.site.summary.failed_pages, 1);
    assert_eq!(g.site.summary.broken_links, 0);
    let g = crawl(
        &format!("{}/large", s.root),
        CrawlOptions {
            respect_robots: false,
            max_body_bytes: 256,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(g.site.summary.failed_pages, 1);
    let g = crawl(
        &format!("{}/malformed.xml", s.root),
        CrawlOptions::default(),
    )
    .await
    .unwrap();
    assert!(g
        .site
        .warnings
        .iter()
        .any(|w| w.contains("invalid sitemap")));
    assert!(crawl(
        &s.root,
        CrawlOptions {
            concurrency: 0,
            ..Default::default()
        }
    )
    .await
    .is_err());
}

#[tokio::test]
async fn exports_round_trip_and_reject_invalid_graphs() {
    let s = Server::start(false).await;
    let mut graph = crawl(&s.root, CrawlOptions::default()).await.unwrap();
    graph.edges[0].anchor = "quote \" & <tag> \\ newline\n日本語\u{1}".into();
    graph.analyze().unwrap();
    let mut json = Vec::new();
    export::write(&graph, Format::Json, &mut json).unwrap();
    let mut loaded: Graph = serde_json::from_slice(&json).unwrap();
    loaded.analyze().unwrap();
    assert_eq!(
        serde_json::to_value(&graph).unwrap(),
        serde_json::to_value(&loaded).unwrap()
    );
    let mut xml = Vec::new();
    export::write(&graph, Format::Graphml, &mut xml).unwrap();
    let xml = String::from_utf8(xml).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    assert_eq!(
        doc.descendants()
            .filter(|n| n.tag_name().name() == "edge")
            .count(),
        graph.edges.len()
    );
    let mut dot = Vec::new();
    export::write(&graph, Format::Dot, &mut dot).unwrap();
    let dot = String::from_utf8(dot).unwrap();
    assert!(dot.contains("\\\""));
    assert!(dot.contains("日本語"));
    loaded.schema_version = "2.0".into();
    assert!(loaded.analyze().is_err());
    loaded.schema_version = "1.0".into();
    loaded.nodes.pop();
    assert!(loaded.analyze().is_err());
}

#[tokio::test]
async fn redirects_preserve_shortest_depth_and_check_terminal_errors() {
    let s = Server::start(false).await;
    let graph = crawl(
        &format!("{}/shortcuts", s.root),
        CrawlOptions {
            max_depth: 2,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(node(&graph, "/shortcut-target").depth, Some(1));
    assert_eq!(node(&graph, "/shortcut-child").status_code, Some(200));
    assert_eq!(node(&graph, "/shortcut-child").depth, Some(2));
    assert_eq!(s.hits("/shortcut-target"), 1);
    let graph = crawl(
        &format!("{}/broken-redirect-page", s.root),
        CrawlOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(graph.site.summary.broken_links, 1);
    let graph = crawl(
        &format!("{}/blocked-redirect", s.root),
        CrawlOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(node(&graph, "/private").state, FetchState::BlockedRobots);
    assert_eq!(s.hits("/private"), 0);
}

#[tokio::test]
async fn compressed_sitemaps_and_sitemap_budgets() {
    let s = Server::start(false).await;
    let graph = crawl(&format!("{}/pages.xml.gz", s.root), CrawlOptions::default())
        .await
        .unwrap();
    assert!(node(&graph, "/orphan").sitemap);
    let graph = crawl(
        &format!("{}/pages.xml", s.root),
        CrawlOptions {
            max_sitemap_urls: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(graph.nodes.iter().filter(|n| n.sitemap).count(), 1);
    assert!(graph
        .site
        .warnings
        .iter()
        .any(|w| w.contains("sitemap URL limit")));
    let graph = crawl(
        &format!("{}/sitemap.xml", s.root),
        CrawlOptions {
            max_sitemaps: 1,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(graph
        .site
        .warnings
        .iter()
        .any(|w| w.contains("sitemap document limit")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_stdout_is_machine_readable_and_options_are_validated() {
    let server = Server::start(false).await;
    let binary = env!("CARGO_BIN_EXE_ahref");
    let output = std::process::Command::new(binary)
        .args([
            "crawl",
            &server.root,
            "--max-pages",
            "1",
            "--format",
            "json",
            "--ignore-robots",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let graph: Graph = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(graph.site.summary.fetched_pages, 1);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Pages (known)"));
    assert_eq!(server.hits("/robots.txt"), 0);
    let invalid = std::process::Command::new(binary)
        .args(["crawl", &server.root, "--concurrency", "0"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let no_args = std::process::Command::new(binary).output().unwrap();
    assert!(!no_args.status.success());
    let help = std::process::Command::new(binary)
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());

    // Exercise offline commands against a real graph file, without another crawl.
    let directory = std::env::temp_dir().join(format!(
        "ahref-cli-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let input = directory.join("input.json");
    let analyzed = directory.join("analyzed.json");
    let mut modified = graph.clone();
    modified.site.summary.pages = 99999;
    std::fs::write(&input, serde_json::to_vec(&modified).unwrap()).unwrap();
    let analysis = std::process::Command::new(binary)
        .arg("analyze")
        .arg(&input)
        .arg("--output")
        .arg(&analyzed)
        .output()
        .unwrap();
    assert!(
        analysis.status.success(),
        "{}",
        String::from_utf8_lossy(&analysis.stderr)
    );
    let recomputed: Graph = serde_json::from_slice(&std::fs::read(&analyzed).unwrap()).unwrap();
    assert_eq!(recomputed.site.summary.pages, graph.site.summary.pages);
    for format in ["json", "graphml", "dot"] {
        let exported = std::process::Command::new(binary)
            .arg("export")
            .arg(&input)
            .args(["--format", format])
            .output()
            .unwrap();
        assert!(exported.status.success());
        assert!(exported.stderr.is_empty());
        if format == "json" {
            serde_json::from_slice::<Graph>(&exported.stdout)
                .unwrap()
                .validate()
                .unwrap();
        }
        if format == "graphml" {
            roxmltree::Document::parse(std::str::from_utf8(&exported.stdout).unwrap()).unwrap();
        }
        if format == "dot" {
            assert!(exported.stdout.starts_with(b"digraph site"));
        }
    }
    modified.schema_version = "999".into();
    std::fs::write(&input, serde_json::to_vec(&modified).unwrap()).unwrap();
    let rejected = std::process::Command::new(binary)
        .arg("analyze")
        .arg(&input)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("unsupported schema_version"));
    std::fs::remove_file(&input).unwrap();
    std::fs::remove_file(&analyzed).unwrap();
    std::fs::remove_dir(&directory).unwrap();
}

#[tokio::test]
async fn sitemap_urls_start_together_before_following_discovered_links() {
    let server = Server::start(false).await;
    let graph = crawl(
        &format!("{}/pages.xml", server.root),
        CrawlOptions {
            max_pages: 3,
            concurrency: 3,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // The budget fits exactly the homepage and two sitemap URLs. Following the
    // homepage's component first would consume it before reaching the orphan.
    assert_eq!(node(&graph, "/orphan").status_code, Some(200));
    assert_eq!(node(&graph, "/a").status_code, Some(200));
    assert_eq!(server.hits("/"), 1);
    assert_eq!(server.hits("/a"), 1);
    assert_eq!(server.hits("/orphan"), 1);
    assert_eq!(server.hits("/b?q=1"), 0);
    assert_eq!(graph.site.summary.fetched_pages, 3);

    // Seed scheduling must not invent root reachability or zero-depth pages.
    assert_eq!(node(&graph, "/a").depth, Some(1));
    assert_eq!(node(&graph, "/orphan").depth, None);
    assert_eq!(graph.site.summary.orphan_pages, 1);
}
