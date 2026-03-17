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

    /// Click at viewport coordinates (real CDP input event)
    ClickAt {
        /// X coordinate (pixels)
        x: f64,
        /// Y coordinate (pixels)
        y: f64,
        /// Mouse button
        #[arg(long, default_value = "left")]
        button: String,
        /// Click count (2 for double-click)
        #[arg(long, default_value = "1")]
        click_count: i32,
    },

    /// Double-click element by ref
    Dblclick {
        /// Element ref (e.g. "e1")
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Hover at viewport coordinates
    HoverAt {
        /// X coordinate (pixels)
        x: f64,
        /// Y coordinate (pixels)
        y: f64,
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

    /// Check a checkbox by ref
    Check {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Uncheck a checkbox by ref
    Uncheck {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Scroll element into view by ref
    Scroll {
        /// Element ref
        #[arg(name = "ref")]
        ref_str: String,
    },

    /// Get visible text content of the page
    Text,

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

    /// Stream network traffic as JSONL (run in separate terminal while interacting)
    NetworkListen {
        /// Maximum seconds to listen (default: unlimited)
        #[arg(long)]
        duration: Option<u64>,
        /// Filter by URL substring
        #[arg(long)]
        filter: Option<String>,
        /// Filter by resource type (XHR, Fetch, Document, Script, etc.)
        #[arg(long, name = "type")]
        resource_type: Option<String>,
    },

    /// List resources loaded on current page (retroactive, no listener needed)
    NetworkList {
        /// Filter by URL substring
        #[arg(long)]
        filter: Option<String>,
    },

    /// Get response body for a request by ID (from network-listen output)
    NetworkGet {
        /// Request ID from network-listen output
        reqid: String,
        /// Save response body to file instead of printing
        #[arg(long)]
        output: Option<String>,
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

    /// Run or validate YAML automation scripts
    Script {
        #[command(subcommand)]
        action: ScriptAction,
    },
}

#[derive(Subcommand)]
enum ScriptAction {
    /// Execute a YAML automation script
    Run {
        /// Path to YAML script file
        script: std::path::PathBuf,

        /// Parameters as key=value pairs (repeatable)
        #[arg(long = "param", value_parser = parse_param)]
        params: Vec<(String, String)>,

        /// Load parameters from a YAML file
        #[arg(long = "param-file")]
        param_file: Option<std::path::PathBuf>,
    },

    /// Validate a YAML script without executing
    Validate {
        /// Path to YAML script file
        script: std::path::PathBuf,

        /// Parameters as key=value pairs (repeatable)
        #[arg(long = "param", value_parser = parse_param)]
        params: Vec<(String, String)>,

        /// Load parameters from a YAML file
        #[arg(long = "param-file")]
        param_file: Option<std::path::PathBuf>,
    },
}

fn parse_param(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid param format '{s}', expected key=value"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
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
        Command::ClickAt {
            x,
            y,
            button,
            click_count,
        } => commands::click_at(&mut state, x, y, &button, click_count).await,
        Command::Dblclick { ref_str } => commands::dblclick(&mut state, &ref_str).await,
        Command::HoverAt { x, y } => commands::hover_at(&mut state, x, y).await,
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
        Command::Check { ref_str } => commands::check(&mut state, &ref_str).await,
        Command::Uncheck { ref_str } => commands::uncheck(&mut state, &ref_str).await,
        Command::Scroll { ref_str } => commands::scroll(&mut state, &ref_str).await,
        Command::Text => commands::text(&mut state).await,
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
        Command::NetworkListen {
            duration,
            filter,
            resource_type,
        } => {
            commands::network_listen(
                &mut state,
                duration,
                filter.as_deref(),
                resource_type.as_deref(),
            )
            .await
        }
        Command::NetworkList { filter } => {
            commands::network_list(&mut state, filter.as_deref()).await
        }
        Command::NetworkGet { reqid, output } => {
            commands::network_get(&mut state, &reqid, output.as_deref()).await
        }
        Command::CookieList => commands::cookie_list(&mut state).await,
        Command::CookieSet {
            name,
            value,
            domain,
            path,
        } => commands::cookie_set(&mut state, &name, &value, &domain, &path).await,
        Command::Pdf { filename } => commands::pdf(&mut state, filename.as_deref()).await,
        Command::Health => commands::health(&state).await,
        Command::Script { action } => commands::script(&cdp, action).await,
    };

    if let Err(e) = result {
        output::error(&format!("{:#}", e));
        std::process::exit(1);
    }
}
