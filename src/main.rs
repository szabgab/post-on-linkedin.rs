mod cli;
mod config;
mod linkedin;

use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{Cli, Commands};
use config::{Config, normalize_urn};
use linkedin::{LinkedInClient, build_post_payload};
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load(&cli.config)?;

    match cli.command {
        Commands::Post {
            text,
            text_file,
            image,
            alt_text,
            org,
            visibility,
            dry_run,
        } => {
            let commentary = match (text, text_file) {
                (Some(t), None) => t,
                (None, Some(path)) => fs::read_to_string(&path).with_context(|| {
                    format!("Failed to read post text from '{}'", path.display())
                })?,
                (None, None) => {
                    bail!("Please provide post content via `--text <TEXT>` or `--text-file <PATH>`")
                }
                (Some(_), Some(_)) => unreachable!("clap conflicts_with prevents both"),
            };

            if commentary.trim().is_empty() {
                bail!("Post text commentary cannot be empty");
            }

            let author_urn = cfg.resolve_author_urn(org.as_deref())?;

            // Organization posts on LinkedIn must use PUBLIC visibility
            let visibility_str = if author_urn.starts_with("urn:li:organization:") {
                "PUBLIC"
            } else {
                visibility.as_linkedin_str()
            };

            if let Some(ref img_path) = image
                && !img_path.exists()
            {
                bail!("Image file '{}' does not exist", img_path.display());
            }

            if dry_run {
                let simulated_image_urn = image
                    .as_ref()
                    .map(|p| format!("urn:li:image:<DRY_RUN:{}>", p.display()));
                let payload = build_post_payload(
                    &author_urn,
                    &commentary,
                    simulated_image_urn,
                    alt_text,
                    visibility_str,
                );
                println!("--- DRY RUN (no request sent to LinkedIn) ---");
                println!("Target Author URN: {}", author_urn);
                if let Some(img_path) = image {
                    println!("Image to upload:   {}", img_path.display());
                }
                println!("POST https://api.linkedin.com/rest/posts");
                println!("LinkedIn-Version:  {}", cfg.api_version);
                println!("Payload:\n{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            let client = LinkedInClient::new(cfg.access_token.clone(), cfg.api_version.clone())?;

            let image_urn = if let Some(ref img_path) = image {
                println!(
                    "Uploading image '{}' for owner {}...",
                    img_path.display(),
                    author_urn
                );
                let urn = client.upload_image(&author_urn, img_path).await?;
                println!("Image uploaded successfully: {}", urn);
                Some(urn)
            } else {
                None
            };

            let payload = build_post_payload(
                &author_urn,
                &commentary,
                image_urn,
                alt_text,
                visibility_str,
            );

            println!("Publishing post to {}...", author_urn);
            let post_urn = client.create_post(&payload).await?;
            println!("Successfully published LinkedIn post! Post ID: {}", post_urn);
        }

        Commands::Whoami => {
            let client = LinkedInClient::new(cfg.access_token, cfg.api_version)?;
            let info = client.get_userinfo().await?;
            println!("LinkedIn Profile Information:");
            println!("  Person ID (sub): {}", info.sub);
            println!("  Author URN:      urn:li:person:{}", info.sub);
            if let Some(name) = info.name {
                println!("  Name:            {}", name);
            }
            if let Some(email) = info.email {
                println!("  Email:           {}", email);
            }
            println!(
                "\nTip: Copy `{}` into `person_id` in your config.yaml.",
                info.sub
            );
        }

        Commands::Targets => {
            println!("Configured Targets in '{}':", cli.config.display());
            match cfg.person_id.as_deref() {
                Some(id) if !id.trim().is_empty() => {
                    println!("  Personal Profile (default): {}", normalize_urn(id, "person"));
                }
                _ => {
                    println!("  Personal Profile (default): <not set>");
                }
            }
            if cfg.organizations.is_empty() {
                println!("  Organizations:              <none configured>");
            } else {
                println!("  Organizations (use with `--org <alias>`):");
                for (alias, id) in &cfg.organizations {
                    println!(
                        "    - {:<18} -> {}",
                        alias,
                        normalize_urn(id, "organization")
                    );
                }
            }
        }
    }

    Ok(())
}
