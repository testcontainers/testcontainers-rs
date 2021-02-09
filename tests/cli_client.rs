use spectral::prelude::*;
use std::time::{Duration, Instant};
use testcontainers::{images::hello_world::HelloWorld, *};

#[test]
fn should_wait_for_at_least_one_second_before_fetching_logs() {
    let _ = pretty_env_logger::try_init();

    let docker = clients::Cli::default();

    let before_run = Instant::now();

    let container = docker.run(HelloWorld);

    let after_run = Instant::now();

    let before_logs = Instant::now();

    container.logs();

    let after_logs = Instant::now();

    assert_that(&(after_run - before_run)).is_greater_than(Duration::from_secs(1));
    assert_that(&(after_logs - before_logs)).is_less_than(Duration::from_secs(1));
}
