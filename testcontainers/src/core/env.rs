use std::str::FromStr;

/// Lookup and parse the command specified through the `TESTCONTAINERS` env variable.
pub fn command<E>() -> Option<Command>
where
    E: GetEnvValue,
{
    // warn users that the old behaviour is gone
    if E::get_env_value("KEEP_CONTAINERS").is_some() {
        log::warn!("`KEEP_CONTAINERS` has been changed to `TESTCONTAINERS`");
    }

    let command = E::get_env_value("TESTCONTAINERS")?;
    let command = command.parse().ok()?;

    Some(command)
}

/// Abstracts over reading a value from the environment.
pub trait GetEnvValue {
    fn get_env_value(key: &str) -> Option<String>;
}

/// Represents the operation system environment for use within a production environment.
#[derive(Debug)]
pub struct Os;

impl GetEnvValue for Os {
    fn get_env_value(key: &str) -> Option<String> {
        ::std::env::var(key).ok()
    }
}

/// The commands available to the `TESTCONTAINERS` env variable.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Command {
    Keep,
    Remove,
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keep" => Ok(Command::Keep),
            "remove" => Ok(Command::Remove),
            other => panic!(
                "unknown command '{}' provided via TESTCONTAINERS env variable",
                other
            ),
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        Command::Remove
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
                "TESTCONTAINERS" => Some("keep".to_owned()),
                _ => None,
            }
        }
    }

    #[test]
    #[should_panic(expected = "unknown command 'foobar' provided via TESTCONTAINERS env variable")]
    fn panics_on_unknown_command() {
        let _ = "foobar".parse::<Command>();
    }

    #[test]
    fn command_looks_up_testcontainers_env_variables() {
        let cmd = command::<FakeEnvAlwaysKeep>().unwrap();

        assert_eq!(cmd, Command::Keep)
    }

    #[test]
    fn default_command_is_remove() {
        let cmd = Command::default();

        assert_eq!(cmd, Command::Remove)
    }
}
