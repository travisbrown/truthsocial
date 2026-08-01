//! Fetch an account and its most recent statuses, authenticating from a session config file (the
//! one written by the `truthsocial-api-cli` `ts-login` utility).
//!
//! ```text
//! # 1. Capture a session (opens a browser to log in / clear Cloudflare); writes creds.toml:
//! cargo run -p truthsocial-api-cli -- setup
//!
//! # 2. Fetch (config defaults to creds.toml; override the path with --config):
//! cargo run -p truthsocial-api --example fetch -- realDonaldTrump
//! cargo run -p truthsocial-api --example fetch -- realDonaldTrump --config path/to/creds.toml
//! ```
//!
//! If the config has no bearer token, the example falls back to an OAuth password-grant login from
//! the `TRUTHSOCIAL_CLIENT_ID` / `_CLIENT_SECRET` / `_USERNAME` / `_PASSWORD` environment
//! variables.

use std::path::PathBuf;

use truthsocial_api::{auth::Credentials, client::Client, config::Config, types::TimelineParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse an optional account handle (positional) and an optional `--config <path>` (default
    // `creds.toml`).
    let mut config_path = Config::default_path();
    let mut handle: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" | "-c" => {
                config_path = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or("--config requires a path")?;
            }
            _ if handle.is_none() => handle = Some(arg),
            _ => {}
        }
    }
    let handle = handle.unwrap_or_else(|| "realDonaldTrump".to_owned());

    // Authenticate from the captured session (User-Agent + Cloudflare cookies + token).
    let mut builder = Client::builder();
    match Config::load(&config_path) {
        Ok(config) => {
            builder = builder.config(&config);
            eprintln!("Loaded session from {}", config_path.display());
        }
        Err(error) => eprintln!(
            "Could not load a session from {} ({error}); run `truthsocial-api-cli` to create one",
            config_path.display()
        ),
    }
    let mut client = builder.build()?;

    // If the config had no bearer token, fall back to an OAuth login from environment credentials.
    if !client.is_authenticated() {
        if let Ok(credentials) = Credentials::from_env() {
            client.login(&credentials).await?;
            eprintln!("Logged in via OAuth password grant");
        } else {
            eprintln!(
                "Not authenticated (no token in config and no TRUTHSOCIAL_* env credentials set); \
                 requests may be rejected"
            );
        }
    }

    let account = client.lookup_account(&handle).await?;
    println!(
        "@{} ({}): id {}, {} followers, {} statuses, joined {}",
        account.acct,
        account.display_name,
        account.id,
        account.followers_count,
        account.statuses_count,
        account.created_at.date_naive(),
    );

    let id = account.id.to_string();
    let statuses = client
        .account_statuses(
            &id,
            &TimelineParams::default().limit(5).exclude_replies(true),
        )
        .await?;

    println!("\nMost recent {} statuses:", statuses.len());
    for status in &statuses {
        // `content` is HTML; strip tags for a readable one-line preview.
        let text: String = strip_html(&status.content);
        let preview: String = text.chars().take(100).collect();
        println!(
            "  {} [{}] {preview}",
            status.id,
            status.created_at.date_naive()
        );
    }

    Ok(())
}

/// Very small HTML-to-text helper for previews: drops anything between `<` and `>`.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
