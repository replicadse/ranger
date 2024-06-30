include!("check_features.rs");

pub mod args;
pub mod error;
pub mod reference;

use {anyhow::Result, args::ManualFormat, git2::FetchOptions, std::{collections::HashMap, path::{Path, PathBuf}}};

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
                | crate::args::GenerateCommand::Git { out, repo, branch, folder, vars } => {
                    let vars_map = build_vars(&vars);
                    let mut hb = handlebars::Handlebars::new();
                    hb.set_strict_mode(true);

                    let out_path_root = Path::new(&out);
                    let temp_dir = std::env::temp_dir();
                    let root_dir = Path::join(&temp_dir, &folder);

                    let mut fo = FetchOptions::new();
                    fo.depth(1);

                    git2::build::RepoBuilder::new()
                        .branch(&branch)
                        .fetch_options(fo)
                        .clone(&repo, &temp_dir)?;

                    if std::fs::create_dir_all(&out).is_err() {
                        return Err(anyhow::anyhow!("failed to create output directory"));
                    }

                    for w in walkdir::WalkDir::new(root_dir) {
                        let entry = w?;
                        let path = entry.path();

                        let rel_path = hb.render(path.strip_prefix(&temp_dir)?.to_str().unwrap(), &vars_map)?;
                        let out_path = Path::join(out_path_root, rel_path);

                        if out_path.is_dir() {
                            std::fs::create_dir_all(out_path)?;
                        } else {
                            let content = std::fs::read_to_string(path)?;
                            let rendered = hb.render(&content, &vars_map)?;
                            std::fs::write(out_path, rendered)?;
                        }
                    }

                    std::fs::remove_dir_all(temp_dir)?;

                    Ok(())
                },
            }
        }
    }
}

fn build_vars(vars: &HashMap<String, String>) {
    fn recursive_add(namespace: &mut std::collections::VecDeque<String>, parent: &mut serde_json::Value, value: &str) {
        let current_namespace = namespace.pop_front().unwrap();
        match namespace.len() {
            | 0 => {
                parent
                    .as_object_mut()
                    .unwrap()
                    .entry(&current_namespace)
                    .or_insert(serde_json::Value::String(value.into()));
            },
            | _ => {
                let p = parent
                    .as_object_mut()
                    .unwrap()
                    .entry(&current_namespace)
                    .or_insert(serde_json::Value::Object(serde_json::Map::new()));
                recursive_add(namespace, p, value);
            },
        }
    }

    let mut values_json = serde_json::Value::Object(serde_json::Map::new());
    for v in vars {
        let namespaces_vec: Vec<String> = v.0.split('.').map(|s| s.to_string()).collect();
        let mut namespaces = std::collections::VecDeque::from(namespaces_vec);
        recursive_add(&mut namespaces, &mut values_json, v.1);
    }
}
