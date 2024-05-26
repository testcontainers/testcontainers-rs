mod config;

pub use config::ConfigurationError;
pub(crate) use config::{Command, Config};

/// Abstracts over reading a value from the environment.
pub trait GetEnvValue {
    fn get_env_value(key: &str) -> Option<String>;
}

/// Represents the operating system environment for use within a production environment.
#[derive(Debug)]
pub struct Os;

impl GetEnvValue for Os {
    fn get_env_value(key: &str) -> Option<String> {
        ::std::env::var(key).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FakeEnvAlwaysKeep;

    impl GetEnvValue for FakeEnvAlwaysKeep {
        fn get_env_value(key: &str) -> Option<String> {
            match key {
                "TESTCONTAINERS_COMMAND" => Some("keep".to_owned()),
                _ => None,
            }
        }
    }

    #[test]
    fn errors_on_unknown_command() {
        let res = "foobar".parse::<Command>();
        assert!(res.is_err());
    }

    #[test]
    fn command_looks_up_testcontainers_env_variables() {
        let cmd = FakeEnvAlwaysKeep::get_env_value("TESTCONTAINERS_COMMAND").unwrap();

        assert!(matches!(cmd.parse::<Command>(), Ok(Command::Keep)),)
    }

    #[test]
    fn default_command_is_remove() {
        let cmd = Command::default();

        assert_eq!(cmd, Command::Remove)
    }
}
