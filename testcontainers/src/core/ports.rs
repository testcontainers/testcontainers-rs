use std::{collections::HashMap, net::IpAddr, num::ParseIntError};

use bollard::models::{PortBinding, PortMap};

/// Represents a port that is exposed by a container.
///
/// There is a helper [`IntoContainerPort`] trait to convert a `u16` into a [`ContainerPort`].
/// Also, `u16` can be directly converted into a `ContainerPort` using `Into::into`, it will default to `ContainerPort::Tcp`.
#[derive(
    parse_display::Display, parse_display::FromStr, Debug, Clone, Copy, Eq, PartialEq, Hash,
)]
pub enum ContainerPort {
    #[display("{0}/tcp")]
    #[from_str(regex = r"^(?<0>\d+)(?:/tcp)?$")]
    Tcp(u16),
    #[display("{0}/udp")]
    Udp(u16),
    #[display("{0}/sctp")]
    Sctp(u16),
}

/// A trait to allow easy conversion of a `u16` into a `ContainerPort`.
/// For example, `123.udp()` is equivalent to `ContainerPort::Udp(123)`.
pub trait IntoContainerPort {
    fn tcp(self) -> ContainerPort;
    fn udp(self) -> ContainerPort;
    fn sctp(self) -> ContainerPort;
}

#[derive(Debug, thiserror::Error)]
pub enum PortMappingError {
    #[error("failed to parse container port: {0}")]
    FailedToParseContainerPort(parse_display::ParseError),
    #[error("failed to parse host port: {0}")]
    FailedToParseHostPort(ParseIntError),
}

/// The exposed ports of a running container.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ports {
    ipv4_mapping: HashMap<ContainerPort, u16>,
    ipv6_mapping: HashMap<ContainerPort, u16>,
}

impl Ports {
    pub fn new(
        ports: HashMap<String, Option<Vec<HashMap<String, String>>>>,
    ) -> Result<Self, PortMappingError> {
        let port_binding = ports
            .into_iter()
            .filter_map(|(internal, external)| {
                Some((
                    internal,
                    Some(
                        external?
                            .into_iter()
                            .map(|external| PortBinding {
                                host_ip: external.get("HostIp").cloned(),
                                host_port: external.get("HostPort").cloned(),
                            })
                            .collect(),
                    ),
                ))
            })
            .collect::<HashMap<_, _>>();

        Self::try_from(port_binding)
    }

    /// Returns the host port for the given internal container's port, on the host's IPv4 interfaces.
    pub fn map_to_host_port_ipv4(&self, container_port: impl Into<ContainerPort>) -> Option<u16> {
        self.ipv4_mapping.get(&container_port.into()).cloned()
    }

    /// Returns the host port for the given internal container's port, on the host's IPv6 interfaces.
    pub fn map_to_host_port_ipv6(&self, container_port: impl Into<ContainerPort>) -> Option<u16> {
        self.ipv6_mapping.get(&container_port.into()).cloned()
    }
}

impl TryFrom<PortMap> for Ports {
    type Error = PortMappingError;

    fn try_from(ports: PortMap) -> Result<Self, Self::Error> {
        let mut ipv4_mapping = HashMap::new();
        let mut ipv6_mapping = HashMap::new();
        for (internal, external) in ports {
            // internal is of the form '8332/tcp', split off the protocol ...
            let container_port = internal
                .parse::<ContainerPort>()
                .map_err(PortMappingError::FailedToParseContainerPort)?;

            // get the `HostPort` of each external port binding
            for binding in external.into_iter().flatten() {
                if let Some(host_port) = binding.host_port.as_ref() {
                    let host_port = host_port
                        .parse()
                        .map_err(PortMappingError::FailedToParseHostPort)?;

                    // switch on the IP version of the `HostIp`
                    let mapping = match binding.host_ip.map(|ip| ip.parse()) {
                        Some(Ok(IpAddr::V4(_))) => {
                            log::debug!(
                                "Registering IPv4 port mapping: {} -> {}",
                                container_port,
                                host_port
                            );
                            &mut ipv4_mapping
                        }
                        Some(Ok(IpAddr::V6(_))) => {
                            log::debug!(
                                "Registering IPv6 port mapping: {} -> {}",
                                container_port,
                                host_port
                            );
                            &mut ipv6_mapping
                        }
                        Some(Err(_)) | None => continue,
                    };

                    mapping.insert(container_port, host_port);
                } else {
                    continue;
                }
            }
        }

        Ok(Self {
            ipv4_mapping,
            ipv6_mapping,
        })
    }
}

impl ContainerPort {
    /// Returns the port number, regardless of the protocol.
    pub fn as_u16(self) -> u16 {
        match self {
            ContainerPort::Tcp(port) | ContainerPort::Udp(port) | ContainerPort::Sctp(port) => port,
        }
    }
}

impl IntoContainerPort for u16 {
    fn tcp(self) -> ContainerPort {
        ContainerPort::Tcp(self)
    }

    fn udp(self) -> ContainerPort {
        ContainerPort::Udp(self)
    }

    fn sctp(self) -> ContainerPort {
        ContainerPort::Sctp(self)
    }
}

impl From<u16> for ContainerPort {
    fn from(port: u16) -> Self {
        ContainerPort::Tcp(port)
    }
}

#[cfg(test)]
mod tests {
    use bollard::models::ContainerInspectResponse;

    use super::*;

    #[test]
    fn can_deserialize_docker_inspect_response_into_api_ports() {
        let container_details = serde_json::from_str::<ContainerInspectResponse>(
            r#"{
  "Id": "1233c36b54a5bac19efbf92728aa33b2faf67f3364f24db506d90fd46a5d0e8c",
  "Created": "2021-02-19T04:57:38.081442827Z",
  "Path": "/hello",
  "Args": [],
  "State": {
    "Status": "exited",
    "Running": false,
    "Paused": false,
    "Restarting": false,
    "OOMKilled": false,
    "Dead": false,
    "Pid": 0,
    "ExitCode": 0,
    "Error": "",
    "StartedAt": "2021-02-19T04:57:40.898633268Z",
    "FinishedAt": "2021-02-19T04:57:40.898476096Z"
  },
  "Image": "sha256:bf756fb1ae65adf866bd8c456593cd24beb6a0a061dedf42b26a993176745f6b",
  "ResolvConfPath": "/var/lib/docker/containers/1233c36b54a5bac19efbf92728aa33b2faf67f3364f24db506d90fd46a5d0e8c/resolv.conf",
  "HostnamePath": "/var/lib/docker/containers/1233c36b54a5bac19efbf92728aa33b2faf67f3364f24db506d90fd46a5d0e8c/hostname",
  "HostsPath": "/var/lib/docker/containers/1233c36b54a5bac19efbf92728aa33b2faf67f3364f24db506d90fd46a5d0e8c/hosts",
  "LogPath": "/var/lib/docker/containers/1233c36b54a5bac19efbf92728aa33b2faf67f3364f24db506d90fd46a5d0e8c/1233c36b54a5bac19efbf92728aa33b2faf67f3364f24db506d90fd46a5d0e8c-json.log",
  "Name": "/serene_mclaren",
  "RestartCount": 0,
  "Driver": "overlay2",
  "Platform": "linux",
  "MountLabel": "",
  "ProcessLabel": "",
  "AppArmorProfile": "",
  "ExecIDs": null,
  "HostConfig": {
    "Binds": null,
    "ContainerIDFile": "",
    "LogConfig": {
      "Type": "json-file",
      "Config": {}
    },
    "NetworkMode": "default",
    "PortBindings": {},
    "RestartPolicy": {
      "Name": "no",
      "MaximumRetryCount": 0
    },
    "AutoRemove": false,
    "VolumeDriver": "",
    "VolumesFrom": null,
    "CapAdd": null,
    "CapDrop": null,
    "CgroupnsMode": "host",
    "Dns": [],
    "DnsOptions": [],
    "DnsSearch": [],
    "ExtraHosts": null,
    "GroupAdd": null,
    "IpcMode": "private",
    "Cgroup": "",
    "Links": null,
    "OomScoreAdj": 0,
    "PidMode": "",
    "Privileged": false,
    "PublishAllPorts": false,
    "ReadonlyRootfs": false,
    "SecurityOpt": null,
    "UTSMode": "",
    "UsernsMode": "",
    "ShmSize": 67108864,
    "Runtime": "runc",
    "ConsoleSize": [
      0,
      0
    ],
    "Isolation": "",
    "CpuShares": 0,
    "Memory": 0,
    "NanoCpus": 0,
    "CgroupParent": "",
    "BlkioWeight": 0,
    "BlkioWeightDevice": [],
    "BlkioDeviceReadBps": null,
    "BlkioDeviceWriteBps": null,
    "BlkioDeviceReadIOps": null,
    "BlkioDeviceWriteIOps": null,
    "CpuPeriod": 0,
    "CpuQuota": 0,
    "CpuRealtimePeriod": 0,
    "CpuRealtimeRuntime": 0,
    "CpusetCpus": "",
    "CpusetMems": "",
    "Devices": [],
    "DeviceCgroupRules": null,
    "DeviceRequests": null,
    "KernelMemory": 0,
    "KernelMemoryTCP": 0,
    "MemoryReservation": 0,
    "MemorySwap": 0,
    "MemorySwappiness": null,
    "OomKillDisable": false,
    "PidsLimit": null,
    "Ulimits": null,
    "CpuCount": 0,
    "CpuPercent": 0,
    "IOMaximumIOps": 0,
    "IOMaximumBandwidth": 0,
    "MaskedPaths": [
      "/proc/asound",
      "/proc/acpi",
      "/proc/kcore",
      "/proc/keys",
      "/proc/latency_stats",
      "/proc/timer_list",
      "/proc/timer_stats",
      "/proc/sched_debug",
      "/proc/scsi",
      "/sys/firmware"
    ],
    "ReadonlyPaths": [
      "/proc/bus",
      "/proc/fs",
      "/proc/irq",
      "/proc/sys",
      "/proc/sysrq-trigger"
    ]
  },
  "GraphDriver": {
    "Data": {
      "LowerDir": "/var/lib/docker/overlay2/0ae22e77c278a63623a89bf85eafae0d7a8a4c2997078241e62c92e1452a97c6-init/diff:/var/lib/docker/overlay2/0aec19489d594957aed93558799b64242594b1e97769468e13fb7f4169214930/diff",
      "MergedDir": "/var/lib/docker/overlay2/0ae22e77c278a63623a89bf85eafae0d7a8a4c2997078241e62c92e1452a97c6/merged",
      "UpperDir": "/var/lib/docker/overlay2/0ae22e77c278a63623a89bf85eafae0d7a8a4c2997078241e62c92e1452a97c6/diff",
      "WorkDir": "/var/lib/docker/overlay2/0ae22e77c278a63623a89bf85eafae0d7a8a4c2997078241e62c92e1452a97c6/work"
    },
    "Name": "overlay2"
  },
  "Mounts": [],
  "Config": {
    "Hostname": "1233c36b54a5",
    "Domainname": "",
    "User": "",
    "AttachStdin": false,
    "AttachStdout": true,
    "AttachStderr": true,
    "Tty": false,
    "OpenStdin": false,
    "StdinOnce": false,
    "Env": [
      "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    ],
    "Cmd": [
      "/hello"
    ],
    "Image": "hello-world",
    "Volumes": null,
    "WorkingDir": "",
    "Entrypoint": null,
    "OnBuild": null,
    "Labels": {}
  },
  "NetworkSettings": {
    "Bridge": "",
    "SandboxID": "ea04d01ca4b48cf9b02710833859ed47f17cd39849384797c0316d97c5c73d6e",
    "HairpinMode": false,
    "LinkLocalIPv6Address": "",
    "LinkLocalIPv6PrefixLen": 0,
    "Ports": {
      "18332": [],
      "18332/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33076"
        }
      ],
      "18333/udp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33075"
        }
      ],
      "18443/tcp": null,
      "18444/tcp": null,
      "8332/sctp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33078"
        }
      ],
      "8333/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33077"
        },
        {
          "HostIp": "::",
          "HostPort": "49718"
        }
      ]
    }
  ,
    "SandboxKey": "/var/run/docker/netns/ea04d01ca4b4",
    "SecondaryIPAddresses": null,
    "SecondaryIPv6Addresses": null,
    "EndpointID": "",
    "Gateway": "",
    "GlobalIPv6Address": "",
    "GlobalIPv6PrefixLen": 0,
    "IPAddress": "",
    "IPPrefixLen": 0,
    "IPv6Gateway": "",
    "MacAddress": "",
    "Networks": {
      "bridge": {
        "IPAMConfig": null,
        "Links": null,
        "Aliases": null,
        "NetworkID": "24a52bac87c0adf5d8ab8a97c46ed75541a9cf8592fdf8af0e7b491fecadedb0",
        "EndpointID": "",
        "Gateway": "",
        "IPAddress": "",
        "IPPrefixLen": 0,
        "IPv6Gateway": "",
        "GlobalIPv6Address": "",
        "GlobalIPv6PrefixLen": 0,
        "MacAddress": "",
        "DriverOpts": null
      }
    }
  }
}"#,
        )
        .expect("JSON is valid");

        let parsed_ports = container_details
            .network_settings
            .unwrap_or_default()
            .ports
            .map(Ports::try_from)
            .expect("ports are mapped correctly")
            .unwrap_or_default();

        let mut expected_ports = Ports::default();
        expected_ports.ipv6_mapping.insert(8333.tcp(), 49718);
        expected_ports.ipv4_mapping.insert(8332.sctp(), 33078);
        expected_ports.ipv4_mapping.insert(18332.tcp(), 33076);
        expected_ports.ipv4_mapping.insert(8333.tcp(), 33077);
        expected_ports.ipv4_mapping.insert(18333.udp(), 33075);

        assert_eq!(parsed_ports, expected_ports)
    }
}
