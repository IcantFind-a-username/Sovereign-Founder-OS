use super::{Cli, Commands};
use clap::{error::ErrorKind, CommandFactory, Parser};

#[test]
fn playground_clap_port_and_option_contract() {
    let cases = [
        (&["sovereign", "playground"][..], Some(7788)),
        (&["sovereign", "playground", "--port", "0"][..], Some(0)),
        (
            &["sovereign", "playground", "--port", "7788"][..],
            Some(7788),
        ),
        (
            &["sovereign", "playground", "--port", "65535"][..],
            Some(65535),
        ),
        (&["sovereign", "playground", "--port", "65536"][..], None),
        (&["sovereign", "playground", "--port", "-1"][..], None),
        (&["sovereign", "playground", "--port", "abc"][..], None),
        (&["sovereign", "playground", "--port"][..], None),
        (&["sovereign", "playground", "--unknown"][..], None),
        (&["sovereign", "playground", "--root", "/tmp"][..], None),
        (&["sovereign", "playground", "--no-open"][..], None),
        (&["sovereign", "playground", "extra"][..], None),
    ];

    for (argv, expected_port) in cases {
        let parsed = Cli::try_parse_from(argv);
        match expected_port {
            Some(port) => match parsed.expect("valid Playground arguments").command {
                Commands::Playground { port: actual } => assert_eq!(actual, port),
                _ => panic!("parsed a different command"),
            },
            None => assert!(parsed.is_err(), "expected rejection for {argv:?}"),
        }
    }
}

#[test]
fn playground_help_lists_only_port_and_builtin_help() {
    let top_level = Cli::command().render_help().to_string();
    assert!(top_level.contains("playground"));

    let command = Cli::command();
    let playground = command
        .find_subcommand("playground")
        .expect("Playground subcommand metadata");
    let option_ids: Vec<_> = playground
        .get_arguments()
        .map(|argument| argument.get_id().as_str())
        .collect();
    // Clap supplies --help as a built-in flag; get_arguments exposes only
    // application-defined options, so the closure must contain just port.
    assert_eq!(option_ids, vec!["port"]);
    assert_eq!(
        playground
            .get_arguments()
            .find(|a| a.get_id() == "port")
            .and_then(|a| a.get_default_values().first())
            .map(|v| v.to_string_lossy()),
        Some("7788".into())
    );

    let help = Cli::try_parse_from(["sovereign", "playground", "--help"]);
    match help {
        Err(error) => assert_eq!(error.kind(), ErrorKind::DisplayHelp),
        Ok(_) => panic!("--help unexpectedly parsed as a command"),
    }
}

#[test]
fn ui_clap_and_dispatch_remain_compatible() {
    let source = include_str!("main.rs");
    assert!(source.contains(
        "Ui {\n        /// Port to bind on loopback\n        #[arg(long, default_value_t = 7787)]\n        port: u16,\n        /// Do not open the browser automatically\n        #[arg(long)]\n        no_open: bool,\n    }"
    ));
    assert!(
        source.contains("Commands::Ui { port, no_open } => ui::run(port, data_dir(), !no_open)?,")
    );
    assert!(source
        .contains("Commands::Playground { port } => sovereign_consultant_playground::run(port)?,"));
    assert!(!source.contains("Playground { port, "));
    match Cli::try_parse_from(["sovereign", "ui"]).unwrap().command {
        Commands::Ui { port, no_open } => assert_eq!((port, no_open), (7787, false)),
        _ => panic!("parsed a different command"),
    }
    match Cli::try_parse_from(["sovereign", "ui", "--port", "0", "--no-open"])
        .unwrap()
        .command
    {
        Commands::Ui { port, no_open } => assert_eq!((port, no_open), (0, true)),
        _ => panic!("parsed a different command"),
    }
}
