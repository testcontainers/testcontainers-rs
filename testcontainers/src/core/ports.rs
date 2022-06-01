use bollard_stubs::models::{PortBinding, PortMap};
use std::{collections::HashMap, net::IpAddr};

/// The exposed ports of a running container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ports {
    ipv4_mapping: HashMap<u16, u16>,
    ipv6_mapping: HashMap<u16, u16>,
}

impl Ports {
    pub fn new(ports: HashMap<String, Option<Vec<HashMap<String, String>>>>) -> Self {
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

        Self::from(port_binding)
    }

    /// Returns the host port for the given internal port, on the host's IPv4 interfaces.
    pub fn map_to_host_port_ipv4(&self, internal_port: u16) -> Option<u16> {
        self.ipv4_mapping.get(&internal_port).cloned()
    }

    /// Returns the host port for the given internal port, on the host's IPv6 interfaces.
    pub fn map_to_host_port_ipv6(&self, internal_port: u16) -> Option<u16> {
        self.ipv6_mapping.get(&internal_port).cloned()
    }
}

impl From<PortMap> for Ports {
    fn from(ports: PortMap) -> Self {
        let mut ipv4_mapping = HashMap::new();
        let mut ipv6_mapping = HashMap::new();
        for (internal, external) in ports {
            // internal is of the form '8332/tcp', split off the protocol ...
            let internal_port = if let Some(internal) = internal.split('/').next() {
                parse_port(internal)
            } else {
                continue;
            };

            // get the `HostPort` of each external port binding
            for binding in external.into_iter().flatten() {
                if let Some(external_port) = binding.host_port.as_ref() {
                    let external_port = parse_port(external_port);

                    // switch on the IP version of the `HostIp`
                    let mapping = match binding.host_ip.map(|ip| ip.parse()) {
                        Some(Ok(IpAddr::V4(_))) => {
                            log::debug!(
                                "Registering IPv4 port mapping: {} -> {}",
                                internal_port,
                                external_port
                            );
                            &mut ipv4_mapping
                        }
                        Some(Ok(IpAddr::V6(_))) => {
                            log::debug!(
                                "Registering IPv6 port mapping: {} -> {}",
                                internal_port,
                                external_port
                            );
                            &mut ipv6_mapping
                        }
                        Some(Err(_)) | None => continue,
                    };

                    mapping.insert(internal_port, external_port);
                } else {
                    continue;
                }
            }
        }

        Self {
            ipv4_mapping,
            ipv6_mapping,
        }
    }
}

fn parse_port(port: &str) -> u16 {
    port.parse()
        .unwrap_or_else(|e| panic!("Failed to parse {} as u16 because {}", port, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bollard_stubs::models::ContainerInspectResponse;

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
      "18332/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33076"
        }
      ],
      "18333/tcp": [
        {
          "HostIp": "0.0.0.0",
          "HostPort": "33075"
        }
      ],
      "18443/tcp": null,
      "18444/tcp": null,
      "8332/tcp": [
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
        .unwrap();

        let parsed_ports = container_details
            .network_settings
            .unwrap_or_default()
            .ports
            .map(Ports::from)
            .unwrap_or_default();

        let mut expected_ports = Ports::default();
        expected_ports.ipv4_mapping.insert(18332, 33076);
        expected_ports.ipv4_mapping.insert(18333, 33075);
        expected_ports.ipv4_mapping.insert(8332, 33078);
        expected_ports.ipv4_mapping.insert(8333, 33077);
        expected_ports.ipv6_mapping.insert(8333, 49718);

        assert_eq!(parsed_ports, expected_ports)
    }
}
