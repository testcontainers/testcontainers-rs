# Custom configuration

You can override some default properties if your environment requires that.

## Configuration locations

The configuration may be loaded from multiple locations. Properties are considered in the following order:

1. Environment variables
2. `~/.testcontainers.properties` file (a Java properties file, enabled by the `properties-config` feature)
   Example locations:  
   **Linux:** `/home/myuser/.testcontainers.properties`  
   **Windows:** `C:/Users/myuser/.testcontainers.properties`  
   **macOS:** `/Users/myuser/.testcontainers.properties`

## Docker host resolution

The host is resolved in the following order:

1. Docker host from the `tc.host` property in the `~/.testcontainers.properties` file.
2. `DOCKER_HOST` environment variable.
3. Docker host from the "docker.host" property in the `~/.testcontainers.properties` file.
4. Else, the default Docker socket will be returned.

## Docker authentication

Sometimes the Docker images you use live in a private Docker registry.
For that reason, Testcontainers for Rust gives you the ability to read the Docker configuration and retrieve the authentication for a given registry.
Configuration is fetched in the following order:

1. `DOCKER_AUTH_CONFIG` environment variable, unmarshalling the string value from its JSON representation and using it as the Docker config.
2. `DOCKER_CONFIG` environment variable, as an alternative path to the Docker config file.
3. else it will load the default Docker config file, which lives in the user's home, e.g. `~/.docker/config.json`.

