use std::collections::HashMap;

#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
struct Sctp(u16);
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
struct Tcp(u16);
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
struct Udp(u16);

/// The exposed ports of a running container.
#[derive(Debug, PartialEq, Default)]
pub struct Ports {
    tcp: HashMap<Tcp, Tcp>,
    udp: HashMap<Udp, Udp>,
    sctp: HashMap<Sctp, Sctp>,
}

impl Ports {
    pub fn new(ports: HashMap<String, Option<Vec<HashMap<String, String>>>>) -> Self {
        let tcp = HashMap::new();
        let udp = HashMap::new();
        let sctp = HashMap::new();

        ports
            .into_iter()
            .filter_map(|(internal, external)| {
                // internal is ')8332/tcp', split off the protocol ...
                let mut iter = internal.split('/');
                let internal = iter.next()?;
                let protocol = iter.next()?.to_string();

                // external is a an optional list of maps: [ { "HostIp": "0.0.0.0", "HostPort": "33078" } ]
                // get the first entry and get the value of the `HostPort` field
                let external = external?.first()?.get("HostPort").cloned()?;

                let internal = parse_port(internal);
                let external = parse_port(&external);

                log::debug!(
                    "Registering port mapping: {} -> {} / {}",
                    internal,
                    external,
                    protocol
                );

                Some((protocol, internal, external))
            })
            .fold(Self { tcp, udp, sctp }, |mut mappings, val| {
                let (protocol, internal, external) = val;
                match protocol.as_str() {
                    "tcp" => {
                        mappings.tcp.insert(Tcp(internal), Tcp(external));
                    }
                    "udp" => {
                        mappings.udp.insert(Udp(internal), Udp(external));
                    }
                    "sctp" => {
                        mappings.sctp.insert(Sctp(internal), Sctp(external));
                    }
                    _ => panic!("Not a valid port mapping."),
                };
                mappings
            })
    }
}

pub trait MapToHostPort<T> {
    /// Returns the host port for the given internal port.
    fn map_to_host_port(&self, internal_port: &T) -> Option<T>;
}

impl MapToHostPort<u16> for Ports {
    fn map_to_host_port(&self, internal_port: &u16) -> Option<u16> {
        self.map_to_host_port(&Tcp(*internal_port)).map(|p| p.0)
    }
}

impl MapToHostPort<Tcp> for Ports {
    fn map_to_host_port(&self, internal_port: &Tcp) -> Option<Tcp> {
        self.tcp.get(internal_port).copied()
    }
}

impl MapToHostPort<Udp> for Ports {
    fn map_to_host_port(&self, internal_port: &Udp) -> Option<Udp> {
        self.udp.get(internal_port).copied()
    }
}

impl MapToHostPort<Sctp> for Ports {
    fn map_to_host_port(&self, internal_port: &Sctp) -> Option<Sctp> {
        self.sctp.get(internal_port).copied()
    }
}

fn parse_port(port: &str) -> u16 {
    port.parse()
        .unwrap_or_else(|e| panic!("Failed to parse {} as u16 because {}", port, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplift::rep::ContainerDetails;

    #[test]
    fn can_deserialize_docker_inspect_response_into_api_ports() {
        let container_details = serde_json::from_str::<ContainerDetails>(
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
            .ports
            .map(Ports::new)
            .unwrap_or_default();

        let mut expected_ports = Ports::default();
        expected_ports.tcp.insert(Tcp(18332), Tcp(33076));
        expected_ports.tcp.insert(Tcp(18333), Tcp(33075));
        expected_ports.tcp.insert(Tcp(8332), Tcp(33078));
        expected_ports.tcp.insert(Tcp(8333), Tcp(33077));

        assert_eq!(parsed_ports, expected_ports)
    }
}
