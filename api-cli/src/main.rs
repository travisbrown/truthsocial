//! `truthsocial-api`: command-line access to the Truth Social API.
//!
//! Subcommands:
//!   - `setup`: open a browser to log in and clear Cloudflare, saving the session (User-Agent +
//!     Cloudflare cookies + token) to the config file.
//!   - `lookup-user-name <handle>`: fetch an account by handle and print the raw JSON.
//!   - `lookup-user-id <id>`: fetch an account by numeric id and print the raw JSON.
//!   - `lookup-status <status>`: fetch a status by id and print the raw JSON.
//!   - `lookup-group-id <id>`: fetch a group by numeric id and print the raw JSON.
//!   - `lookup-group-name <slug>`: fetch a group by its URL slug and print the raw JSON.
//!   - `group-memberships <id> --role <owner|admin|member> [--q <query>]`: list a group's members
//!     for one role and print the raw JSON.
//!   - `search <query> [--type <profiles|truths|groups|topics>] [--resolve]`: search and print the
//!     raw JSON results.
//!
//! ```text
//! cargo run -p truthsocial-api-cli -- setup
//! cargo run -p truthsocial-api-cli -- lookup-user-name realDonaldTrump
//! cargo run -p truthsocial-api-cli -- lookup-user-id 107780257626128497
//! cargo run -p truthsocial-api-cli -- lookup-status 116803704330293684
//! cargo run -p truthsocial-api-cli -- lookup-group-id 110419665997191580
//! cargo run -p truthsocial-api-cli -- lookup-group-name rapesaw-gang
//! cargo run -p truthsocial-api-cli -- group-memberships 112969595140221305 --role member
//! cargo run -p truthsocial-api-cli -- search trump --type groups
//! ```
//!
//! `--config <path>` overrides the config location (default `creds.toml`). `setup` also honors
//! `TRUTH_SOCIAL_LOGIN_PROFILE` (persistent browser profile directory) and `CHROME` (browser
//! binary).

use std::io::Write;
use std::path::{Path, PathBuf};

use chromiumoxide::{Browser, BrowserConfig};
use cli_helpers::prelude::*;
use futures::StreamExt;
use truthsocial_api::{client::Client, config::Config, types::SearchKind};

const START_URL: &str = "https://truthsocial.com/";

#[derive(Debug, Parser)]
#[command(name = "truthsocial-api", version, author)]
struct Opts {
    #[clap(flatten)]
    verbose: Verbosity,
    /// Path to the session config file (written by `setup`)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Open a browser to log in and clear Cloudflare, saving the session to the config file
    Setup,
    /// Look up an account by handle and print the raw JSON
    LookupUserName {
        /// Account handle (without a leading `@`)
        handle: String,
    },
    /// Look up an account by numeric id and print the raw JSON
    LookupUserId {
        /// Numeric account id
        id: String,
    },
    /// Look up a status by id and print the raw JSON
    LookupStatus {
        /// Status id
        status: String,
    },
    /// Look up a group by numeric id and print the raw JSON
    LookupGroupId {
        /// Numeric group id
        id: String,
    },
    /// Look up a group by its URL slug and print the raw JSON
    LookupGroupName {
        /// Group URL slug (e.g. `rapesaw-gang`)
        slug: String,
    },
    /// List a group's members for one role and print the raw JSON
    GroupMemberships {
        /// Numeric group id
        id: String,
        /// Membership role to list
        #[arg(long, value_enum)]
        role: GroupRole,
        /// Optional handle/display-name filter (sent as the `q` parameter)
        #[arg(long, default_value = "")]
        q: String,
    },
    /// Search and print the raw JSON results
    Search {
        /// The search query
        query: String,
        /// Restrict the search to one kind of entity (default: all kinds)
        #[arg(long = "type", value_enum)]
        kind: Option<SearchType>,
        /// Ask the server to resolve a remote handle or URL (requires an authenticated session)
        #[arg(long)]
        resolve: bool,
    },
}

/// The kind of entity to search for, named with Truth Social's user-facing vocabulary.
///
/// These map onto the API's `type` values via [`SearchKind`]: `profiles` → accounts, `truths` →
/// statuses, `topics` → hashtags, and `groups` unchanged.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum SearchType {
    /// Accounts (users).
    Profiles,
    /// Posts.
    Truths,
    /// Groups (Truth Social specific).
    Groups,
    /// Hashtags.
    Topics,
}

impl From<SearchType> for SearchKind {
    fn from(value: SearchType) -> Self {
        match value {
            SearchType::Profiles => Self::Accounts,
            SearchType::Truths => Self::Statuses,
            SearchType::Groups => Self::Groups,
            SearchType::Topics => Self::Hashtags,
        }
    }
}

/// A group membership role, as accepted by `GET /api/v1/groups/:id/memberships?role=`.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum GroupRole {
    /// The group's owner.
    Owner,
    /// A group administrator.
    Admin,
    /// A regular member (the API names this role `user`).
    Member,
}

impl GroupRole {
    /// The `role` query-parameter value the API expects.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Member => "user",
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Opts::parse();
    opts.verbose.init_logging()?;
    let config_path = opts.config.unwrap_or_else(Config::default_path);

    match opts.command {
        Command::Setup => setup(&config_path).await?,
        Command::LookupUserName { handle } => {
            let json = client(&config_path)?
                .get_text("/api/v1/accounts/lookup", &[("acct", handle)])
                .await?;
            println!("{json}");
        }
        Command::LookupUserId { id } => {
            let json = client(&config_path)?
                .get_text(&format!("/api/v1/accounts/{id}"), &[])
                .await?;
            println!("{json}");
        }
        Command::LookupStatus { status } => {
            let json = client(&config_path)?
                .get_text(&format!("/api/v1/statuses/{status}"), &[])
                .await?;
            println!("{json}");
        }
        Command::LookupGroupId { id } => {
            let json = client(&config_path)?
                .get_text(&format!("/api/v1/groups/{id}"), &[])
                .await?;
            println!("{json}");
        }
        Command::LookupGroupName { slug } => {
            let json = client(&config_path)?
                .get_text("/api/v1/groups/lookup", &[("slug", slug)])
                .await?;
            println!("{json}");
        }
        Command::GroupMemberships { id, role, q } => {
            let json = client(&config_path)?
                .get_text(
                    &format!("/api/v1/groups/{id}/memberships"),
                    &[("role", role.as_str().to_owned()), ("q", q)],
                )
                .await?;
            println!("{json}");
        }
        Command::Search {
            query,
            kind,
            resolve,
        } => {
            let json = client(&config_path)?
                .search_raw(&query, kind.map(SearchKind::from), resolve)
                .await?;
            println!("{json}");
        }
    }

    Ok(())
}

/// Build an API client from the saved session config.
fn client(config_path: &Path) -> Result<Client, Error> {
    Ok(Client::builder()
        .config(&Config::load(config_path)?)
        .build()?)
}

/// Open a headed browser so the user can log in and clear Cloudflare, then capture the User-Agent
/// and Cloudflare cookies (and, best-effort, the bearer token) into the config file at `output`.
async fn setup(output: &Path) -> Result<(), Error> {
    // A persistent profile: any Cloudflare clearance and login survive between runs, and the
    // browser looks like a real, used one rather than a throwaway. Override the location with
    // `TRUTH_SOCIAL_LOGIN_PROFILE`.
    let profile = std::env::var_os("TRUTH_SOCIAL_LOGIN_PROFILE")
        .map_or_else(|| PathBuf::from("login-profile"), PathBuf::from);
    std::fs::create_dir_all(&profile)?;

    // Launch a headed Chromium that does NOT look automated. By default `chromiumoxide` passes
    // `--enable-automation` and leaves `navigator.webdriver` set, which Cloudflare flags, making
    // its "security verification" loop forever. So drop the default args (which include
    // `--enable-automation`) and disable the `AutomationControlled` blink feature.
    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .with_head()
            .disable_default_args()
            .arg("disable-blink-features=AutomationControlled")
            .args([
                "no-first-run",
                "no-default-browser-check",
                "disable-background-networking",
                "disable-renderer-backgrounding",
                "disable-backgrounding-occluded-windows",
                "disable-hang-monitor",
                "password-store=basic",
                "lang=en-US",
            ])
            .user_data_dir(&profile)
            .window_size(1280, 1024)
            .build()
            .map_err(Error::BrowserConfig)?,
    )
    .await?;

    // `chromiumoxide` requires its event handler to be driven continuously.
    let handler_task = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            if event.is_err() {
                break;
            }
        }
    });

    // Open blank first so the last automation tell (`navigator.webdriver`) can be hidden before any
    // of Truth Social's (or Cloudflare's) scripts run, then navigate.
    let page = browser.new_page("about:blank").await?;
    page.evaluate_on_new_document(
        "Object.defineProperty(navigator, 'webdriver', { get: () => undefined });",
    )
    .await?;
    page.goto(START_URL).await?;

    eprintln!("\nA browser window has opened at {START_URL}");
    eprintln!("  1. Complete any Cloudflare check and log in.");
    eprintln!("  2. Come back here and press Enter to capture your session.");
    eprint!("\nPress Enter when ready... ");
    std::io::stderr().flush()?;
    // Block for Enter on a blocking thread so the async runtime keeps driving the browser.
    tokio::task::spawn_blocking(|| {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)
    })
    .await??;

    let user_agent: String = page.evaluate("navigator.userAgent").await?.into_value()?;

    let cookies = page.get_cookies().await?;
    let cookie = |name: &str| {
        cookies
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.value.clone())
    };
    let cf_clearance = cookie("cf_clearance").ok_or(Error::MissingClearance)?;

    // Best-effort: Truth Social's frontend stashes the OAuth token in localStorage; scan for it.
    let token: Option<String> = page
        .evaluate(
            r#"(() => {
                for (let i = 0; i < localStorage.length; i++) {
                    const value = localStorage.getItem(localStorage.key(i));
                    const match = value && value.match(/"access_token":"([^"]+)"/);
                    if (match) return match[1];
                }
                return null;
            })()"#,
        )
        .await?
        .into_value()
        .ok()
        .flatten();

    let config = Config {
        user_agent,
        cf_clearance,
        cf_bm: cookie("__cf_bm"),
        token,
    };
    config.save(output)?;

    eprintln!("\nSaved session to {}", output.display());
    eprintln!("  user_agent: {}", config.user_agent);
    eprintln!(
        "  cf_clearance: {}… ({} chars)",
        &config.cf_clearance[..config.cf_clearance.len().min(16)],
        config.cf_clearance.len()
    );
    eprintln!(
        "  __cf_bm: {}",
        if config.cf_bm.is_some() {
            "captured"
        } else {
            "absent"
        }
    );
    if config.token.is_some() {
        eprintln!("  token: captured");
    } else {
        eprintln!(
            "  token: not found (the API may still work with cf_clearance + a separate token)"
        );
    }

    browser.close().await?;
    let _ = handler_task.await;
    Ok(())
}

/// Errors from the `truthsocial-api` commands.
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("CLI argument error")]
    Args(#[from] cli_helpers::Error),
    #[error("API client error")]
    Api(#[from] truthsocial_api::error::Error),
    #[error("config error")]
    Config(#[from] truthsocial_api::config::ConfigError),
    #[error("browser error")]
    Browser(#[from] chromiumoxide::error::CdpError),
    #[error("browser configuration error: {0}")]
    BrowserConfig(String),
    #[error("no `cf_clearance` cookie found; complete the Cloudflare check before pressing Enter")]
    MissingClearance,
    #[error("background task error")]
    Join(#[from] tokio::task::JoinError),
    #[error("JSON error")]
    Json(#[from] serde_json::Error),
    #[error("I/O error")]
    Io(#[from] std::io::Error),
}
