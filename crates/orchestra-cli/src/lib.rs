//! User-facing command-line interface (`orchestra`).

pub mod cli;
pub mod client;
pub mod commands;
pub mod output;

#[cfg(test)]
mod tests {
    use super::cli::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing_cluster_create() {
        let args = [
            "orchestra",
            "cluster",
            "create",
            "test-clus",
            "--workers",
            "3",
        ];
        let parsed = Cli::try_parse_from(args).unwrap();
        match parsed.command {
            Commands::Cluster(ClusterArgs {
                command:
                    ClusterCommands::Create {
                        name,
                        workers,
                        control_plane,
                        ..
                    },
            }) => {
                assert_eq!(name, "test-clus");
                assert_eq!(workers, 3);
                assert_eq!(control_plane, 1);
            }
            _ => panic!("Expected Cluster create command"),
        }
    }

    #[test]
    fn test_cli_parsing_cluster_list() {
        let args = ["orchestra", "cluster", "list"];
        let parsed = Cli::try_parse_from(args).unwrap();
        assert!(matches!(
            parsed.command,
            Commands::Cluster(ClusterArgs {
                command: ClusterCommands::List
            })
        ));
    }

    #[test]
    fn test_cli_parsing_system_ping() {
        let args = ["orchestra", "system", "ping"];
        let parsed = Cli::try_parse_from(args).unwrap();
        assert!(matches!(
            parsed.command,
            Commands::System(SystemArgs {
                command: SystemCommands::Ping
            })
        ));
    }
}
