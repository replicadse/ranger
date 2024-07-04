include!("check_features.rs");

pub mod args;
pub mod error;
pub mod reference;
mod blueprint;

use {anyhow::Result, args::ManualFormat, blueprint::Blueprint, git2::FetchOptions, handlebars::JsonRender, std::{collections::HashMap, path::{Path, PathBuf}}};

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

                    let render_result = render(&vars, &root_dir, out_path_root);
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

                    match render(&vars, &folder, out_path_root) {
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

struct Helper {
    cmd: String,
}
impl handlebars::HelperDef for Helper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        h: &handlebars::Helper<'rc>,
        _: &handlebars::Handlebars<'reg>,
        _: &handlebars::Context,
        _: &mut handlebars::RenderContext<'reg, 'rc>
    ) -> std::result::Result<handlebars::ScopedJson<'rc>, handlebars::RenderError>{
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.cmd)
            .env("VALUE", h.param(0).unwrap().value().render())
            .output()?;
        assert!(output.status.success());
        let v = serde_json::Value::String(String::from_utf8(output.stdout).unwrap());
        Ok(handlebars::ScopedJson::Derived(v))
    }
}

fn render(vars: &HashMap<String, String>, root_dir: &Path, out_path_root: &Path) -> Result<(), anyhow::Error> {
    let mut hb = handlebars::Handlebars::new();
    hb.register_escape_fn(|s| s.into());
    hb.set_strict_mode(true);

    let bp_vars = if let Ok(v) = std::fs::read_to_string(Path::join(root_dir, ".ranger.yaml")) {
        let blueprint: blueprint::Blueprint = serde_yaml::from_str(&v)?;
        let v = build_vars(Some(&blueprint), vars);

        if let Some(helpers) = &blueprint.helpers {
            for (name, cmd) in helpers {
                let h = Helper { cmd: cmd.clone() };
                hb.register_helper(&name, Box::new(h));
            }
        }

        (Some(blueprint), v)
    } else {
        (None, build_vars(None, vars))
    };

    for w in walkdir::WalkDir::new(root_dir) {
        let entry = w?;
        let path = entry.path();

        let rel_path = hb.render_template(path.strip_prefix(&root_dir)?.to_str().unwrap(), &bp_vars.1)?;
        if rel_path == ".ranger.yaml" {
            continue;
        }
        let out_path = Path::join(out_path_root, rel_path);

        if path.is_dir() {
            std::fs::create_dir_all(out_path)?;
        } else {
            let content = std::fs::read_to_string(path)?;
            let rendered = hb.render_template(&content, &bp_vars.1)?;
            std::fs::write(out_path, rendered)?;
        }
    }

    Ok(())
}

fn build_vars(blueprint: Option<&Blueprint>, vars: &HashMap<String, String>) -> serde_json::Value {
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

    let mut vars_map = serde_json::Value::Object(serde_json::Map::new());
    if let Some(blueprint) = blueprint {
        for v in &blueprint.vars {
            if let Some(default_value) = &v.1.default {
                let namespaces_vec: Vec<String> = v.0.split('.').map(|s| s.to_string()).collect();
                let mut namespaces = std::collections::VecDeque::from(namespaces_vec);
                recursive_add(&mut namespaces, &mut vars_map, &default_value);
            }
        }
    }
    for v in vars {
        let namespaces_vec: Vec<String> = v.0.split('.').map(|s| s.to_string()).collect();
        let mut namespaces = std::collections::VecDeque::from(namespaces_vec);
        recursive_add(&mut namespaces, &mut vars_map, v.1);
    }
    let mut values_json = HashMap::<String, serde_json::Value>::new();
    values_json.insert("vars".to_owned(), vars_map);

    serde_json::to_value(&values_json).unwrap()
}
