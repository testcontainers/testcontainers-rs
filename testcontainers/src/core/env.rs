mod config;

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
    #[should_panic(
        expected = "unknown command 'foobar' provided via TESTCONTAINERS_COMMAND env variable"
    )]
    fn panics_on_unknown_command() {
        let _ = "foobar".parse::<Command>();
    }

    #[test]
    fn command_looks_up_testcontainers_env_variables() {
        let cmd = FakeEnvAlwaysKeep::get_env_value("TESTCONTAINERS_COMMAND").unwrap();

        assert_eq!(cmd.parse::<Command>(), Ok(Command::Keep))
    }

    #[test]
    fn default_command_is_remove() {
        let cmd = Command::default();

        assert_eq!(cmd, Command::Remove)
    }
}
