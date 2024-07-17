include!("check_features.rs");

pub mod args;
pub mod error;
pub mod reference;
mod blueprint;

use {anyhow::Result, args::ManualFormat, blueprint::Blueprint, git2::FetchOptions, std::{collections::HashMap, path::{Path, PathBuf}}};

#[tokio::main]
async fn main() -> Result<()> {
    let cmd = crate::args::ClapArgumentLoader::load()?;

    match cmd.command {
        | crate::args::Command::Manual { path, format } => {
            let out_path = PathBuf::from(path);
            std::fs::create_dir_all(&out_path)?;
            match format {
                | ManualFormat::Manpages => {
                    reference::build_manpages(&out_path)?;
                },
                | ManualFormat::Markdown => {
                    reference::build_markdown(&out_path)?;
                },
            }
            Ok(())
        },
        | crate::args::Command::Autocomplete { path, shell } => {
            let out_path = PathBuf::from(path);
            std::fs::create_dir_all(&out_path)?;
            reference::build_shell_completion(&out_path, &shell)?;
            Ok(())
        },
        | crate::args::Command::Generate(c) => {
            match c {
                | crate::args::GenerateCommand::Git { out, repo, branch, folder, vars, force } => {
                    let out_path_root = Path::new(&out);
                    let temp_dir = Path::join(&std::env::temp_dir(), uuid::Uuid::new_v4().to_string());
                    let root_dir = Path::join(&temp_dir, &folder);

                    let mut fo = FetchOptions::new();
                    fo.depth(1);

                    git2::build::RepoBuilder::new()
                        .branch(&branch)
                        .fetch_options(fo)
                        .clone(&repo, &temp_dir)?;

                    if force {
                        let _ = std::fs::remove_dir_all(&out);
                    }
                    if std::fs::create_dir_all(&out).is_err() {
                        return Err(anyhow::anyhow!("failed to create output directory - might already exist"));
                    }

                    let blueprint = serde_yaml::from_str::<Blueprint>(&std::fs::read_to_string(Path::join(&temp_dir, ".ranger.yaml")).unwrap()).unwrap();
                    let render_result = render(&blueprint, &vars, &root_dir, out_path_root).await;
                    std::fs::remove_dir_all(temp_dir)?; // remove temp dir in any case

                    match render_result {
                        | Ok(_) => Ok(()),
                        | Err(e) => {
                            std::fs::remove_dir_all(out_path_root)?;
                            Err(e)
                        },
                    }
                },
                | crate::args::GenerateCommand::Local { out, folder, vars, force } => {
                    let out_path_root = Path::new(&out);
                    let folder = Path::new(&folder);

                    let mut fo = FetchOptions::new();
                    fo.depth(1);

                    if force {
                        let _ = std::fs::remove_dir_all(&out);
                    }
                    if std::fs::create_dir_all(&out).is_err() {
                        return Err(anyhow::anyhow!("failed to create output directory - might already exist"));
                    }

                    let blueprint = serde_yaml::from_str::<Blueprint>(&std::fs::read_to_string(Path::join(folder, ".ranger.yaml")).unwrap()).unwrap();

                    match render(&blueprint, &vars, &folder, out_path_root).await {
                        | Ok(_) => Ok(()),
                        | Err(e) => {
                            std::fs::remove_dir_all(out_path_root)?;
                            Err(e)
                        },
                    }
                },
            }
        }
    }
}

async fn render(bp: &Blueprint, value_overrides: &HashMap<String, String>, root_dir: &Path, out_path_root: &Path) -> Result<(), anyhow::Error> {
    let values = if let Some(variables) = &bp.template.variables {
        complate::render::populate_variables(
            variables,
            value_overrides,
            &complate::render::ShellTrust::Ultimate,
            &complate::render::Backend::CLI,
            Some("vars".to_owned()),
        )
        .await?
    } else {
        HashMap::<_, _>::new()
    };

    let hb = complate::render::make_handlebars(&values, &bp.template.helpers, &complate::render::ShellTrust::Ultimate, true).await?;

    for w in walkdir::WalkDir::new(root_dir) {
        let entry = w?;
        let path = entry.path();

        let rel_path = hb.0.render_template(&path.strip_prefix(&root_dir)?.to_str().unwrap(), &hb.1).map_err(|e| anyhow::anyhow!(e))?;
        if rel_path == ".ranger.yaml" {
            continue;
        }
        let out_path = Path::join(out_path_root, rel_path);

        if path.is_dir() {
            std::fs::create_dir_all(out_path)?;
        } else {
            let content = std::fs::read_to_string(path)?;
            let rendered = hb.0.render_template(&content, &hb.1).map_err(|e| anyhow::anyhow!(e))?;
            std::fs::write(out_path, rendered)?;
        }
    }

    Ok(())
}
