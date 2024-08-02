use std::{
    collections::HashMap,
    str::FromStr,
};

use anyhow::Result;
use clap::{
    Arg,
    ArgAction,
};

use crate::error::Error;

#[derive(Debug, Eq, PartialEq)]
pub enum Privilege {
    Normal,
    Experimental,
}

#[derive(Debug)]
pub struct CallArgs {
    pub privileges: Privilege,
    pub command: Command,
}

impl CallArgs {
    pub fn validate(&self) -> Result<()> {
        if self.privileges == Privilege::Experimental {
            return Ok(());
        }

        match &self.command {
            // | Command::Experimental { .. } => Err(Error::ExperimentalCommand("watch".to_owned()))?,
            | _ => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ManualFormat {
    Manpages,
    Markdown,
}

#[derive(Debug)]
pub enum Command {
    Manual { path: String, format: ManualFormat },
    Autocomplete { path: String, shell: clap_complete::Shell },

    Generate(GenerateCommand),
}

#[derive(Debug)]
pub enum GenerateCommand {
    Local {
        out: String,
        folder: String,
        vars: HashMap<String, String>,
        interactive: bool,
        force: bool,
    },
    Git {
        out: String,
        repo: String,
        branch: String,
        folder: String,
        vars: HashMap<String, String>,
        interactive: bool,
        force: bool,
    },
}

pub struct ClapArgumentLoader {}

impl ClapArgumentLoader {
    pub fn root_command() -> clap::Command {
        clap::Command::new("ranger")
            .version(env!("CARGO_PKG_VERSION"))
            .about("ranger - local development on steroids")
            .author("replicadse <aw@voidpointergroup.com>")
            .propagate_version(true)
            .subcommand_required(true)
            .args([Arg::new("experimental")
                .short('e')
                .long("experimental")
                .help("Enables experimental features.")
                .num_args(0)])
            .subcommand(
                clap::Command::new("man")
                    .about("Renders the manual.")
                    .arg(clap::Arg::new("out").short('o').long("out").required(true))
                    .arg(
                        clap::Arg::new("format")
                            .short('f')
                            .long("format")
                            .value_parser(["manpages", "markdown"])
                            .required(true),
                    ),
            )
            .subcommand(
                clap::Command::new("autocomplete")
                    .about("Renders shell completion scripts.")
                    .arg(clap::Arg::new("out").short('o').long("out").required(true))
                    .arg(
                        clap::Arg::new("shell")
                            .short('s')
                            .long("shell")
                            .value_parser(["bash", "zsh", "fish", "elvish", "powershell"])
                            .required(true),
                    ),
            )
            .subcommand(
                clap::Command::new("generate")
                    .subcommand_required(true)
                    .about("Generate command.")
                    .subcommand(
                        clap::Command::new("git")
                            .about("Generate from git repo.")
                            .arg(clap::Arg::new("out").short('o').long("out").required(true))
                            .arg(
                                clap::Arg::new("repo")
                                    .short('r')
                                    .long("repo")
                                    .default_value("https://github.com/replicadse/ranger.git"),
                            )
                            .arg(clap::Arg::new("branch").short('b').long("branch").default_value("master"))
                            .arg(clap::Arg::new("folder").short('f').long("folder").default_value("./"))
                            .arg(
                                clap::Arg::new("var").short('v').long("var").action(ArgAction::Append).help(
                                    "A variable in the template (placeholder). This takes precendence over varfile.",
                                ),
                            )
                            .arg(
                                clap::Arg::new("varfile")
                                    .long("varfile")
                                    .help("A file path containing variables in the template (placeholder)."),
                            )
                            .arg(
                                clap::Arg::new("interactive").long("interactive").short('i').action(ArgAction::SetTrue),
                            )
                            .arg(clap::Arg::new("force").long("force").action(ArgAction::SetTrue)),
                    )
                    .subcommand(
                        clap::Command::new("local")
                            .about("Generate from a local source folder.")
                            .arg(clap::Arg::new("out").short('o').long("out").required(true))
                            .arg(clap::Arg::new("folder").short('f').long("folder").required(true))
                            .arg(
                                clap::Arg::new("var").short('v').long("var").action(ArgAction::Append).help(
                                    "A variable in the template (placeholder). This takes precendence over varfile.",
                                ),
                            )
                            .arg(
                                clap::Arg::new("varfile")
                                    .long("varfile")
                                    .help("A file path containing variables in the template (placeholder)."),
                            )
                            .arg(
                                clap::Arg::new("interactive").long("interactive").short('i').action(ArgAction::SetTrue),
                            )
                            .arg(clap::Arg::new("force").long("force").action(ArgAction::SetTrue)),
                    ),
            )
    }

    pub fn load() -> Result<CallArgs> {
        let command = Self::root_command().get_matches();

        let privileges = if command.get_flag("experimental") {
            Privilege::Experimental
        } else {
            Privilege::Normal
        };

        let cmd = if let Some(subc) = command.subcommand_matches("man") {
            Command::Manual {
                path: subc.get_one::<String>("out").unwrap().into(),
                format: match subc.get_one::<String>("format").unwrap().as_str() {
                    | "manpages" => ManualFormat::Manpages,
                    | "markdown" => ManualFormat::Markdown,
                    | _ => return Err(Error::Argument("unknown format".into()).into()),
                },
            }
        } else if let Some(subc) = command.subcommand_matches("autocomplete") {
            Command::Autocomplete {
                path: subc.get_one::<String>("out").unwrap().into(),
                shell: clap_complete::Shell::from_str(subc.get_one::<String>("shell").unwrap().as_str()).unwrap(),
            }
        } else if let Some(subc) = command.subcommand_matches("generate") {
            if let Some(subc) = subc.subcommand_matches("git") {
                let mut vars = HashMap::<String, String>::new();
                if let Some(v_argfile) = subc.get_one::<String>("varfile") {
                    let file = std::fs::read_to_string(v_argfile)?;
                    for vo in file.lines() {
                        let spl = vo.splitn(2, "=").collect::<Vec<_>>();
                        vars.insert(spl[0].into(), spl[1].into());
                    }
                }
                if let Some(v_arg) = subc.get_many::<String>("var") {
                    for vo in v_arg {
                        let spl = vo.splitn(2, "=").collect::<Vec<_>>();
                        vars.insert(spl[0].into(), spl[1].into());
                    }
                }
                Command::Generate(GenerateCommand::Git {
                    out: subc.get_one::<String>("out").unwrap().into(),
                    repo: subc.get_one::<String>("repo").unwrap().into(),
                    branch: subc.get_one::<String>("branch").unwrap().into(),
                    folder: subc.get_one::<String>("folder").unwrap().into(),
                    vars,
                    interactive: subc.get_flag("interactive"),
                    force: subc.get_flag("force"),
                })
            } else if let Some(subc) = subc.subcommand_matches("local") {
                let mut vars = HashMap::<String, String>::new();
                if let Some(v_argfile) = subc.get_one::<String>("varfile") {
                    let file = std::fs::read_to_string(v_argfile)?;
                    for vo in file.lines() {
                        let spl = vo.splitn(2, "=").collect::<Vec<_>>();
                        vars.insert(spl[0].into(), spl[1].into());
                    }
                }
                if let Some(v_arg) = subc.get_many::<String>("var") {
                    for vo in v_arg {
                        let spl = vo.splitn(2, "=").collect::<Vec<_>>();
                        vars.insert(spl[0].into(), spl[1].into());
                    }
                }
                Command::Generate(GenerateCommand::Local {
                    out: subc.get_one::<String>("out").unwrap().into(),
                    folder: subc.get_one::<String>("folder").unwrap().into(),
                    vars,
                    interactive: subc.get_flag("interactive"),
                    force: subc.get_flag("force"),
                })
            } else {
                return Err(Error::UnknownCommand.into());
            }
        } else {
            return Err(Error::UnknownCommand.into());
        };

        let callargs = CallArgs {
            privileges,
            command: cmd,
        };

        callargs.validate()?;
        Ok(callargs)
    }
}
