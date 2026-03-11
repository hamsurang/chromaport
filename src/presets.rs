use crate::ir::ThemeIR;
use crate::store;
use anyhow::{Context, Result};
use inquire::InquireError;
use serde::Deserialize;
use std::collections::HashSet;
use std::time::Duration;

const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets/index.json";
const THEME_BASE_URL: &str =
    "https://raw.githubusercontent.com/hamsurang/chromaport/main/assets/presets";
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct Manifest {
    #[allow(dead_code)]
    version: u32,
    themes: Vec<PresetEntry>,
}

#[derive(Deserialize, Clone)]
struct PresetEntry {
    slug: String,
    name: String,
    author: String,
    license: String,
    #[allow(dead_code)]
    source_url: String,
}

fn make_agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .build();
    config.into()
}

fn fetch_manifest(agent: &ureq::Agent) -> Result<Vec<PresetEntry>> {
    let mut response = agent
        .get(MANIFEST_URL)
        .call()
        .context("Failed to fetch preset manifest. Check your internet connection.")?;
    let manifest: Manifest = response
        .body_mut()
        .read_json()
        .context("Failed to parse preset manifest")?;
    Ok(manifest.themes)
}

fn fetch_theme_ir(agent: &ureq::Agent, slug: &str) -> Result<ThemeIR> {
    let url = format!("{THEME_BASE_URL}/{slug}.json");
    let mut response = agent
        .get(&url)
        .call()
        .with_context(|| format!("Failed to download theme: {slug}"))?;
    let ir: ThemeIR = response
        .body_mut()
        .read_json()
        .with_context(|| format!("Invalid theme data: {slug}"))?;
    Ok(ir)
}

fn installed_slugs() -> HashSet<String> {
    store::list_ir_files()
        .unwrap_or_default()
        .iter()
        .filter_map(|p| p.file_stem()?.to_str().map(|s| s.to_string()))
        .collect()
}

pub fn run_list() -> Result<()> {
    println!("Fetching preset themes...");
    let agent = make_agent();
    let presets = fetch_manifest(&agent)?;
    let installed = installed_slugs();

    if presets.is_empty() {
        println!("No preset themes available.");
        return Ok(());
    }

    println!("\nAvailable preset themes:\n");
    for p in &presets {
        let marker = if installed.contains(&p.slug) {
            " (installed)"
        } else {
            ""
        };
        println!("  {} by {} [{}]{}", p.name, p.author, p.license, marker);
    }
    println!("\nRun `chromaport presets install` to install themes.");
    Ok(())
}

pub fn run_install() -> Result<()> {
    println!("Fetching preset themes...");
    let agent = make_agent();
    let presets = fetch_manifest(&agent)?;
    let installed = installed_slugs();

    let available: Vec<&PresetEntry> = presets
        .iter()
        .filter(|p| !installed.contains(&p.slug))
        .collect();

    if available.is_empty() {
        println!("All preset themes are already installed!");
        return Ok(());
    }

    let options: Vec<String> = available
        .iter()
        .map(|p| format!("{} by {} [{}]", p.name, p.author, p.license))
        .collect();

    let selected =
        match inquire::MultiSelect::new("Select themes to install:", options.clone()).prompt() {
            Ok(s) => s,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

    if selected.is_empty() {
        println!("No themes selected.");
        return Ok(());
    }

    let to_install: Vec<&PresetEntry> = selected
        .iter()
        .filter_map(|s| {
            let idx = options.iter().position(|o| o == s)?;
            Some(available[idx])
        })
        .collect();

    println!();
    let mut success_count = 0;
    for entry in &to_install {
        eprint!("  Downloading {}...", entry.name);
        match fetch_theme_ir(&agent, &entry.slug) {
            Ok(ir) => match store::save_ir(&ir) {
                Ok(path) => {
                    eprintln!(" \u{2714} {}", path.display());
                    success_count += 1;
                }
                Err(e) => eprintln!(" \u{2718} save failed: {e}"),
            },
            Err(e) => eprintln!(" \u{2718} {e}"),
        }
    }

    println!("\nInstalled {success_count}/{} themes.", to_install.len());
    if success_count > 0 {
        println!("Run `chromaport apply` to apply a theme to your targets.");
    }
    Ok(())
}
