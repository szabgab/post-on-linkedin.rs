use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "post-on-linkedin",
    version,
    about = "CLI tool to post text and images to your LinkedIn profile or managed organization pages"
)]
pub struct Cli {
    /// Path to the YAML configuration file
    #[arg(short, long, global = true, default_value = "config.yaml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Publish a post (text and optional image) to your profile or an organization page
    Post {
        /// Text content of the post (conflicts with --text-file)
        #[arg(short, long, conflicts_with = "text_file")]
        text: Option<String>,

        /// Read post text content from a file
        #[arg(long)]
        text_file: Option<PathBuf>,

        /// Optional path to an image file (PNG, JPG, GIF) to attach to the post
        #[arg(short, long)]
        image: Option<PathBuf>,

        /// Optional alt-text description for the attached image
        #[arg(long)]
        alt_text: Option<String>,

        /// Post on behalf of an organization (alias from config.yaml or numeric Organization ID).
        /// If omitted, posts to your personal profile (`person_id`).
        #[arg(short, long)]
        org: Option<String>,

        /// Post visibility on LinkedIn
        #[arg(long, value_enum, default_value_t = Visibility::Public)]
        visibility: Visibility,

        /// Print the resolved author URN and post JSON payload without calling the LinkedIn API
        #[arg(long)]
        dry_run: bool,
    },

    /// Query LinkedIn's `/v2/userinfo` endpoint to display your Person ID and profile info
    Whoami,

    /// List the target profile and organization aliases configured in config.yaml
    Targets,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Visibility {
    Public,
    Connections,
}

impl Visibility {
    pub fn as_linkedin_str(self) -> &'static str {
        match self {
            Visibility::Public => "PUBLIC",
            Visibility::Connections => "CONNECTIONS",
        }
    }
}
