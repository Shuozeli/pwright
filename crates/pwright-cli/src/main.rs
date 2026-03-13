//! pwright CLI — browser automation from the command line.
//!
//! Usage:
//!   pwright open <https://example.com>
//!   pwright snapshot
//!   pwright click e1
//!   pwright screenshot
//!   pwright close

mod commands;
mod output;
mod state;

use clap::{Parser, Subcommand};

/// pwright — browser automation CLI powered by Chrome DevTools Protocol.
#[derive(Parser)]
#[command(name = "pwright", version, about = "Browser automation CLI via CDP")]
struct Cli {
    /// Chrome CDP HTTP endpoint
    #[arg(
        long,
        env = "PWRIGHT_CDP",
        default_value = "http://localhost:9222",
        global = true
    )]
    cdp: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Open browser and navigate to URL
    Open {
        /// URL to navigate to
        url: Option<String>,
    },

    /// Navigate active tab to URL
    Goto {
        /// Target URL
        url: String,
    },

    /// Close active tab
    Close,

    /// Click element by ref (from snapshot)
    Click {
        /// Element ref (e.g. "e1")
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Type text into focused element
    Type {
        /// Text to type
        text: String,
    },

    /// Fill input by ref
    Fill {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
        /// Text to fill
        text: String,
    },

    /// Press keyboard key
    Press {
        /// Key name (e.g. "Enter", "Tab", "a")
        key: String,
    },

    /// Hover over element by ref
    Hover {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Select dropdown option by ref
    Select {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
        /// Value to select
        value: String,
    },

    /// Trigger and wait for a download by clicking a ref
    Download {
        /// Element ref to click
        #[arg(name = "ref")]
        ref_str: String,
        /// Optional destination path to move the downloaded file to
        #[arg(long)]
        dest: Option<String>,
    },

    /// Focus an element by ref
    Focus {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Drag element by ref with dx/dy offset
    Drag {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
        /// Horizontal offset
        #[arg(long, default_value = "0")]
        dx: i32,
        /// Vertical offset
        #[arg(long, default_value = "0")]
        dy: i32,
    },

    /// Upload file(s) to a file input by ref
    Upload {
        /// Element ref for file input
        #[arg(name = "ref")]
        ref_str: String,
        /// File path(s) to upload
        files: Vec<String>,
    },

    /// Print page accessibility snapshot
    Snapshot,

    /// Take screenshot of current page
    Screenshot {
        /// Output filename
        #[arg(long)]
        filename: Option<String>,
    },

    /// Evaluate JavaScript expression
    Eval {
        /// JS expression
        expression: String,
    },

    /// Reload current page
    Reload,

    /// Navigate back
    GoBack,

    /// Navigate forward
    GoForward,

    /// List open tabs
    TabList,

    /// Create new tab
    TabNew {
        /// URL for new tab
        url: Option<String>,
    },

    /// Close a tab
    TabClose {
        /// Tab ID (defaults to active)
        tab_id: Option<String>,
    },

    /// Switch active tab
    TabSelect {
        /// Tab ID to activate
        tab_id: String,
    },

    /// List cookies for active tab
    CookieList,

    /// Set cookie on active tab
    CookieSet {
        /// Cookie name
        #[arg(long)]
        name: String,
        /// Cookie value
        #[arg(long)]
        value: String,
        /// Cookie domain
        #[arg(long)]
        domain: String,
        /// Cookie path (default: /)
        #[arg(long, default_value = "/")]
        path: String,
    },

    /// Save page as PDF
    Pdf {
        /// Output filename
        #[arg(long)]
        filename: Option<String>,
    },

    /// Check Chrome connectivity
    Health,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("pwright=info".parse().unwrap()),
        )
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();
    let mut state = state::CliState::load();

    // For `open` and `health`, use --cdp flag
    // For other commands, reuse stored cdp_url
    let cdp = cli.cdp.clone();

    let result = match cli.command {
        Command::Open { url } => commands::open(&mut state, &cdp, url.as_deref()).await,
        Command::Goto { url } => commands::goto(&mut state, &url).await,
        Command::Close => commands::close(&mut state).await,
        Command::Click { ref_str } => commands::click(&mut state, &ref_str).await,
        Command::Type { text } => commands::type_text(&mut state, &text).await,
        Command::Fill { ref_str, text } => commands::fill(&mut state, &ref_str, &text).await,
        Command::Press { key } => commands::press(&mut state, &key).await,
        Command::Hover { ref_str } => commands::hover(&mut state, &ref_str).await,
        Command::Select { ref_str, value } => commands::select(&mut state, &ref_str, &value).await,
        Command::Download { ref_str, dest } => {
            commands::download(&mut state, &ref_str, dest.as_deref()).await
        }
        Command::Focus { ref_str } => commands::focus(&mut state, &ref_str).await,
        Command::Drag { ref_str, dx, dy } => commands::drag(&mut state, &ref_str, dx, dy).await,
        Command::Upload { ref_str, files } => commands::upload(&mut state, &ref_str, &files).await,
        Command::Snapshot => commands::snapshot(&mut state).await,
        Command::Screenshot { filename } => {
            commands::screenshot(&mut state, filename.as_deref()).await
        }
        Command::Eval { expression } => commands::eval(&mut state, &expression).await,
        Command::Reload => commands::reload(&mut state).await,
        Command::GoBack => commands::go_back(&mut state).await,
        Command::GoForward => commands::go_forward(&mut state).await,
        Command::TabList => commands::tab_list(&mut state).await,
        Command::TabNew { url } => commands::tab_new(&mut state, url.as_deref()).await,
        Command::TabClose { tab_id } => commands::tab_close(&mut state, tab_id.as_deref()).await,
        Command::TabSelect { tab_id } => commands::tab_select(&mut state, &tab_id).await,
        Command::CookieList => commands::cookie_list(&mut state).await,
        Command::CookieSet {
            name,
            value,
            domain,
            path,
        } => commands::cookie_set(&mut state, &name, &value, &domain, &path).await,
        Command::Pdf { filename } => commands::pdf(&mut state, filename.as_deref()).await,
        Command::Health => commands::health(&state).await,
    };

    if let Err(e) = result {
        output::error(&format!("{:#}", e));
        std::process::exit(1);
    }
}
