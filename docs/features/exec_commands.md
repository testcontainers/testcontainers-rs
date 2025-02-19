
# Command Execution in Containers

Some test scenarios require running commands within a container.
In order to achieve this, the `testcontainers` library provides the ability
to execute commands in 3 different ways:

- `exec` method of [ContainerAsync] (or [Container])

Allows to run a command in an already running container.

- [Image::exec_after_start](https://docs.rs/testcontainers/latest/testcontainers/core/trait.Image.html#method.exec_after_start)

Only if you implement your own `Image`: Allows to define commands
to be executed after the container is started and ready.

- [Image::exec_before_ready](https://docs.rs/testcontainers/latest/testcontainers/core/trait.Image.html#method.exec_before_ready)

Only if you implement your own `Image`: Allows to define commands
to be executed executed after the container has started,
but before the `Image::ready_conditions` are awaited for.

Here we will focus on the first option, which is the most common one.
The method expects an [ExecCommand] struct,
and returns an [ExecResult](or [SyncExecResult]) struct.

## [ExecCommand]

The [ExecCommand] struct represents a command to be executed within a container.
It includes the command itself and conditions to be checked
on the command output and the container.

### ExecCommand Usage

To create a new [ExecCommand]:

```rust
let command = ExecCommand::new(vec!["echo", "Hello, World!"])
    .with_container_ready_conditions(vec![/* conditions */])
    .with_cmd_ready_condition(CmdWaitFor::message_on_stdout("Hello, World!"));
```

## [CmdWaitFor]

The [CmdWaitFor] enum defines conditions to be checked on the command's output.

## [ExecResult] / [SyncExecResult]

For async version, the [ExecResult] struct represents the result
of an executed command in a container.
For non-async (`blocking` feature), the [SyncExecResult] struct is used instead.

The structs represents the result of an executed command in a container.

### ExecResult Usage

To execute a command and handle the result:

```rust
let result = container.exec(command).await?;
let exit_code = result.exit_code().await?;
let stdout = result.stdout_to_vec().await?;
let stderr = result.stderr_to_vec().await?;
```

[Container]: https://docs.rs/testcontainers/latest/testcontainers/core/struct.Container.html
[ContainerAsync]: https://docs.rs/testcontainers/latest/testcontainers/core/struct.ContainerAsync.html
[ExecCommand]: https://docs.rs/testcontainers/latest/testcontainers/core/struct.ExecCommand.html
[CmdWaitFor]: https://docs.rs/testcontainers/latest/testcontainers/core/enum.CmdWaitFor.html
[ExecResult]: https://docs.rs/testcontainers/latest/testcontainers/core/struct.ExecResult.html
[SyncExecResult]: https://docs.rs/testcontainers/latest/testcontainers/core/struct.SyncExecResult.html
