// Socat can be useful if we would want to allow exposing not exposed ports of a container.
// But for now it's unused, so dead code is allowed.
#![allow(dead_code)]

use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;

use crate::{core::WaitFor, Image};

/// A socat container is used as a TCP proxy, enabling any port of another container to be exposed
/// publicly, even if that container does not make the port public itself.
pub(crate) struct Socat {
    targets: HashMap<u16, String>,
}

impl Socat {
    pub(crate) fn new() -> Self {
        Self {
            targets: HashMap::new(),
        }
    }

    pub(crate) fn add_target(&mut self, port: u16, value: String) {
        self.targets.insert(port, value);
    }
}

impl Image for Socat {
    fn name(&self) -> &str {
        "alpine/socat"
    }

    fn tag(&self) -> &str {
        "1.8.0.1"
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::healthcheck()]
    }

    fn entrypoint(&self) -> Option<&str> {
        Some("/bin/sh")
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        [
            "-c".to_string(),
            self.targets
                .iter()
                .map(|(port, value)| format!("socat TCP-LISTEN:{port},fork,reuseaddr TCP:{value}"))
                .join(" & "),
        ]
    }
}
