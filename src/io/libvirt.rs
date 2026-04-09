use crate::core::boundaries::VerificationLibvirtBoundary;
use crate::core::errors::CoreError;
use crate::core::types::FailureClass;
use crate::core::verification_eval::{
    accept_first_valid_readiness, evaluate_readiness_line, parse_timeout_literal,
};
use crate::core::verification_model::{
    LibvirtGuestHandle, VerificationGuestReadinessPayload, VerificationReadinessAcquisition,
    VerificationReadinessEvidence, VerificationReadinessExpectation, VerificationReadinessRecord,
    VerificationScenarioDefinition, VERIFICATION_READINESS_MARKER,
    VERIFICATION_READINESS_SCRIPT_PATH, VERIFICATION_READINESS_SERVICE_NAME,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_LIBVIRT_URI: &str = "qemu:///system";
const DEFAULT_SSH_USER: &str = "core";
const DEFAULT_BRIDGE: &str = "br0";
const DEFAULT_POOL: &str = "default";
const DEFAULT_BASE_IMAGE: &str = "/var/lib/libvirt/images/fcos-base.qcow2";
const DEFAULT_IGNITION_DIR: &str = "/var/lib/libvirt/ignition";
const DEFAULT_CONSOLE_LOG_DIR: &str = "/var/lib/libvirt/console";
const DEFAULT_DISK_SIZE: &str = "10G";
const DEFAULT_IP_LEASE_ROOT: &str = "/tmp/core-ops-verify-ip-leases";
const DEFAULT_DHCP_IGNITION_TEMPLATE: &str = "infra/ignition/dhcp.bu.tpl";
const DEFAULT_STATIC_IGNITION_TEMPLATE: &str = "infra/ignition/static-ip.bu.tpl";
const DEFAULT_NETWORK_MODE: &str = "dhcp";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibvirtCommandRunner {
    pub connection_uri: String,
    pub env_backed: bool,
    pub vm_host: Option<String>,
    pub ssh_user: String,
    pub bridge: String,
    pub pool: String,
    pub base_image: String,
    pub ignition_dir: String,
    pub console_log_dir: String,
    pub disk_size: String,
    pub network_mode: String,
    pub ignition_template: String,
    pub network_interface: Option<String>,
    pub subnet_cidr: Option<String>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub ip_pool: Option<String>,
    pub lease_root: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VerificationNetworkConfig {
    interface: Option<String>,
    prefix: Option<u8>,
    gateway: Option<String>,
    dns_servers: Vec<String>,
    allocated_ip: Option<String>,
    lease_path: Option<String>,
    rendered_nmconnection: Option<String>,
}

impl Default for LibvirtCommandRunner {
    fn default() -> Self {
        Self::from_env(false)
    }
}

impl LibvirtCommandRunner {
    pub fn from_env(env_backed: bool) -> Self {
        let vm_host = std::env::var("CORE_OPS_VERIFY_VM_HOST").ok();
        let connection_uri = std::env::var("CORE_OPS_VERIFY_LIBVIRT_URI").unwrap_or_else(|_| {
            vm_host
                .as_ref()
                .map(|host| format!("qemu+ssh://{DEFAULT_SSH_USER}@{host}/system"))
                .unwrap_or_else(|| DEFAULT_LIBVIRT_URI.to_string())
        });
        let network_mode =
            std::env::var("CORE_OPS_VERIFY_NETWORK_MODE").unwrap_or_else(|_| DEFAULT_NETWORK_MODE.to_string());
        let default_template = if network_mode == "static" {
            DEFAULT_STATIC_IGNITION_TEMPLATE
        } else {
            DEFAULT_DHCP_IGNITION_TEMPLATE
        };
        Self {
            connection_uri,
            env_backed,
            vm_host,
            ssh_user: std::env::var("CORE_OPS_VERIFY_SSH_USER")
                .unwrap_or_else(|_| DEFAULT_SSH_USER.to_string()),
            bridge: std::env::var("CORE_OPS_VERIFY_BRIDGE")
                .unwrap_or_else(|_| DEFAULT_BRIDGE.to_string()),
            pool: std::env::var("CORE_OPS_VERIFY_POOL").unwrap_or_else(|_| DEFAULT_POOL.to_string()),
            base_image: std::env::var("CORE_OPS_VERIFY_BASE_IMAGE")
                .unwrap_or_else(|_| DEFAULT_BASE_IMAGE.to_string()),
            ignition_dir: std::env::var("CORE_OPS_VERIFY_IGNITION_DIR")
                .unwrap_or_else(|_| DEFAULT_IGNITION_DIR.to_string()),
            console_log_dir: std::env::var("CORE_OPS_VERIFY_CONSOLE_LOG_DIR")
                .unwrap_or_else(|_| DEFAULT_CONSOLE_LOG_DIR.to_string()),
            disk_size: std::env::var("CORE_OPS_VERIFY_DISK_SIZE")
                .unwrap_or_else(|_| DEFAULT_DISK_SIZE.to_string()),
            network_mode,
            ignition_template: std::env::var("CORE_OPS_VERIFY_IGNITION_TEMPLATE")
                .unwrap_or_else(|_| default_template.to_string()),
            network_interface: std::env::var("CORE_OPS_VERIFY_INTERFACE").ok(),
            subnet_cidr: std::env::var("CORE_OPS_VERIFY_SUBNET_CIDR").ok(),
            gateway: std::env::var("CORE_OPS_VERIFY_GATEWAY").ok(),
            dns_servers: std::env::var("CORE_OPS_VERIFY_DNS")
                .ok()
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|entry| !entry.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            ip_pool: std::env::var("CORE_OPS_VERIFY_IP_POOL").ok(),
            lease_root: std::env::var("CORE_OPS_VERIFY_LEASE_ROOT")
                .unwrap_or_else(|_| DEFAULT_IP_LEASE_ROOT.to_string()),
        }
    }

    pub fn virsh_command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("virsh");
        command.arg("-c").arg(&self.connection_uri);
        command.args(args);
        command
    }

    pub fn qemu_img_command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("qemu-img");
        command.args(args);
        command
    }

    fn run_command(&self, command: &mut Command, context: &str) -> Result<String, CoreError> {
        let rendered = render_command(command);
        let output = command.output().map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format_launch_error(command, context, &rendered, &err),
            )
        })?;
        if !output.status.success() {
            return Err(CoreError::new(
                FailureClass::Apply,
                format!(
                    "{context} failed: {rendered}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn ssh_host(&self) -> String {
        format!(
            "{}@{}",
            self.ssh_user,
            self.vm_host.as_deref().unwrap_or("localhost")
        )
    }

    fn render_ignition(
        &self,
        workspace_root: &Path,
        domain_name: &str,
        network: &VerificationNetworkConfig,
        readiness: &VerificationGuestReadinessPayload,
    ) -> Result<(String, String), CoreError> {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(&self.ignition_template);
        if !source.exists() {
            return Err(CoreError::new(
                FailureClass::Validation,
                format!("missing ignition template {}", source.display()),
            ));
        }

        let template = fs::read_to_string(&source).map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("failed to read ignition template {}: {err}", source.display()),
            )
        })?;
        let rendered = template
            .replace("${SSH_PUBLIC_KEY}", &self.read_ssh_public_key()?)
            .replace("${VM_HOSTNAME}", domain_name);
        let rendered = if let Some(interface) = &network.interface {
            rendered.replace("${NETWORK_INTERFACE}", interface)
        } else {
            rendered
        };
        let rendered = if let Some(allocated_ip) = &network.allocated_ip {
            rendered.replace("${STATIC_IPV4_ADDRESS}", allocated_ip)
        } else {
            rendered
        };
        let rendered = if let Some(prefix) = &network.prefix {
            rendered.replace("${STATIC_IPV4_PREFIX}", &prefix.to_string())
        } else {
            rendered
        };
        let rendered = if let Some(gateway) = &network.gateway {
            rendered.replace("${STATIC_IPV4_GATEWAY}", gateway)
        } else {
            rendered
        };
        let rendered = rendered.replace("${STATIC_IPV4_DNS}", &network.dns_servers.join(";"));
        let rendered = rendered.replace(
            "${READINESS_SCRIPT}",
            &indent_block(&render_readiness_script(readiness), 10),
        );
        let rendered = rendered.replace(
            "${READINESS_SERVICE}",
            &indent_block(&render_readiness_service(readiness), 8),
        );
        let butane_path = workspace_root.join(format!("{domain_name}.bu"));
        let ignition_path = workspace_root.join(format!("{domain_name}.ign"));
        fs::write(&butane_path, rendered).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write rendered Butane {}: {err}", butane_path.display()),
            )
        })?;

        let mut butane = Command::new("butane");
        butane.arg(&butane_path).arg("-o").arg(&ignition_path);
        self.run_command(&mut butane, "render Butane ignition")?;
        let ignition_metadata = fs::metadata(&ignition_path).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!(
                    "failed to stat rendered ignition {}: {err}",
                    ignition_path.display()
                ),
            )
        })?;
        if ignition_metadata.len() == 0 {
            return Err(CoreError::new(
                FailureClass::Apply,
                format!(
                    "rendered ignition {} is empty",
                    ignition_path.display()
                ),
            ));
        }
        Ok((
            butane_path.display().to_string(),
            ignition_path.display().to_string(),
        ))
    }

    fn read_ssh_public_key(&self) -> Result<String, CoreError> {
        let explicit = std::env::var("CORE_OPS_VERIFY_SSH_PUBLIC_KEY_FILE")
            .ok()
            .map(|path| shellexpand(&path))
            .map(std::path::PathBuf::from);
        let default = std::env::var("HOME")
            .ok()
            .map(|home| Path::new(&home).join(".ssh/id_ed25519.pub"));
        let key_path = explicit.or(default).ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "unable to locate SSH public key; set CORE_OPS_VERIFY_SSH_PUBLIC_KEY_FILE",
            )
        })?;
        let key = fs::read_to_string(&key_path).map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("failed to read SSH public key {}: {err}", key_path.display()),
            )
        })?;
        Ok(key.trim().to_string())
    }

    fn install_ignition(
        &self,
        workspace_root: &Path,
        domain_name: &str,
        network: &VerificationNetworkConfig,
        readiness: &VerificationGuestReadinessPayload,
    ) -> Result<(String, String, String), CoreError> {
        let (butane_path, local_ignition_path) =
            self.render_ignition(workspace_root, domain_name, network, readiness)?;
        let source = Path::new(&local_ignition_path).to_path_buf();

        if self.vm_host.is_none() {
            return Ok((
                source.display().to_string(),
                butane_path,
                local_ignition_path,
            ));
        }

        let remote_name = format!("{domain_name}.ign");
        let remote_path = format!("{}/{}", self.ignition_dir, remote_name);

        let mut mkdir = Command::new("ssh");
        mkdir.arg(self.ssh_host()).arg(format!(
            "sudo install -d -m 0755 {}",
            shell_escape(&self.ignition_dir)
        ));
        self.run_command(&mut mkdir, "prepare remote ignition directory")?;

        let mut scp = Command::new("scp");
        scp.arg(&source)
            .arg(format!("{}:/tmp/{}", self.ssh_host(), remote_name));
        self.run_command(&mut scp, "copy ignition to vm host")?;

        let mut install = Command::new("ssh");
        install.arg(self.ssh_host()).arg(format!(
            "tmp_size=$(wc -c < /tmp/{name} 2>/dev/null || echo missing); sudo install -m 0644 /tmp/{name} {dest}; dest_size=$(sudo wc -c < {dest} 2>/dev/null || echo missing); rm -f /tmp/{name}; if [ \"$dest_size\" = missing ] || [ \"$dest_size\" = 0 ]; then echo \"tmp_size=$tmp_size dest_size=$dest_size dest={dest}\"; exit 1; fi",
            name = shell_escape(&remote_name),
            dest = shell_escape(&remote_path)
        ));
        self.run_command(&mut install, "install/verify ignition on vm host").map_err(|err| {
            let local_size = fs::metadata(&source)
                .map(|meta| meta.len().to_string())
                .unwrap_or_else(|_| "missing".to_string());
            CoreError::new(
                err.class,
                format!(
                    "{} (local_ignition_size={} local_ignition_path={})",
                    err.message,
                    local_size,
                    source.display()
                ),
            )
        })?;
        Ok((remote_path, butane_path, local_ignition_path))
    }

    fn create_overlay_volume(&self, volume_name: &str) -> Result<(), CoreError> {
        let mut command = self.virsh_command(&[
                "vol-create-as",
                &self.pool,
                volume_name,
                &self.disk_size,
                "--format",
                "qcow2",
                "--backing-vol",
                &self.base_image,
                "--backing-vol-format",
                "qcow2",
            ]);
        self.run_command(&mut command, "create libvirt overlay volume")?;
        Ok(())
    }

    fn install_domain(
        &self,
        domain_name: &str,
        volume_name: &str,
        ignition_path: &str,
        serial_log_path: &str,
        memory_mb: &str,
        vcpus: &str,
    ) -> Result<(), CoreError> {
        let mut install = Command::new("virt-install");
        install
            .arg("--connect")
            .arg(&self.connection_uri)
            .arg("--name")
            .arg(domain_name)
            .arg("--osinfo")
            .arg("fedora-coreos-stable")
            .arg("--memory")
            .arg(memory_mb)
            .arg("--vcpus")
            .arg(vcpus)
            .arg("--import")
            .arg("--disk")
            .arg(format!("vol={}/{},format=qcow2", self.pool, volume_name))
            .arg("--network")
            .arg(format!("bridge={},model=virtio", self.bridge))
            .arg("--graphics")
            .arg("none")
            .arg("--noautoconsole")
            .arg("--serial")
            .arg(format!("file,path={serial_log_path}"))
            .arg(format!(
                "--qemu-commandline=-fw_cfg name=opt/com.coreos/config,file={}",
                ignition_path
            ));
        self.run_command(&mut install, "virt-install guest")?;
        Ok(())
    }

    fn install_log_targets(&self, domain_name: &str) -> Result<String, CoreError> {
        let serial_log_path = format!("{}/{}.serial.log", self.console_log_dir, domain_name);
        if self.vm_host.is_none() {
            fs::create_dir_all(&self.console_log_dir).map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!(
                        "failed to create local console log dir {}: {err}",
                        self.console_log_dir
                    ),
                )
            })?;
            return Ok(serial_log_path);
        }

        let mut mkdir = Command::new("ssh");
        mkdir.arg(self.ssh_host()).arg(format!(
            "sudo install -d -m 0755 {}",
            shell_escape(&self.console_log_dir)
        ));
        self.run_command(&mut mkdir, "prepare remote console log directory")?;
        Ok(serial_log_path)
    }

    fn wait_for_guest_ip(&self, domain_name: &str) -> Result<String, CoreError> {
        for _ in 0..120 {
            for source in ["lease", "agent", "arp"] {
                let mut command = self.virsh_command(&["domifaddr", "--source", source, domain_name]);
                if let Ok(output) = self.run_command(&mut command, "discover guest IP") {
                    if let Some(ip) = parse_domifaddr_ip(&output) {
                        return Ok(ip);
                    }
                }
            }
            sleep(Duration::from_secs(2));
        }
        Err(CoreError::new(
            FailureClass::Transient,
            format!("timed out waiting for guest IP via virsh domifaddr for {domain_name}"),
        ))
    }

    fn cleanup_partial_guest(
        &self,
        domain_name: &str,
        volume_name: &str,
        ignition_path: Option<&str>,
        lease_path: Option<&str>,
    ) {
        let _ = self.virsh_command(&["destroy", domain_name]).output();
        let _ = self.virsh_command(&["undefine", domain_name]).output();
        let _ = self
            .virsh_command(&["vol-delete", "--pool", &self.pool, volume_name])
            .output();
        if let (Some(vm_host), Some(ignition_path)) = (&self.vm_host, ignition_path) {
            let _ = Command::new("ssh")
                .arg(format!("{}@{}", self.ssh_user, vm_host))
                .arg(format!("sudo rm -f {}", shell_escape(ignition_path)))
                .output();
        }
        if let Some(lease_path) = lease_path {
            let _ = fs::remove_file(lease_path);
        }
    }

    fn load_network_config(&self) -> Result<VerificationNetworkConfig, CoreError> {
        if self.network_mode != "static" {
            return Ok(VerificationNetworkConfig {
                interface: None,
                prefix: None,
                gateway: None,
                dns_servers: Vec::new(),
                allocated_ip: None,
                lease_path: None,
                rendered_nmconnection: None,
            });
        }
        let interface = self.network_interface.clone().ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "CORE_OPS_VERIFY_INTERFACE is required for static verification network mode",
            )
        })?;
        let subnet_cidr = self.subnet_cidr.clone().ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "CORE_OPS_VERIFY_SUBNET_CIDR is required for static verification network mode",
            )
        })?;
        let prefix = parse_prefix(&subnet_cidr)?;
        let gateway = self.gateway.clone().ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "CORE_OPS_VERIFY_GATEWAY is required for static verification network mode",
            )
        })?;
        let ip_pool = self.ip_pool.clone().ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "CORE_OPS_VERIFY_IP_POOL is required for static verification network mode",
            )
        })?;
        let (start, end) = parse_ip_pool(&ip_pool)?;
        let lease_dir = Path::new(&self.lease_root).join(sanitize_identifier(
            self.vm_host.as_deref().unwrap_or("local"),
        ));
        fs::create_dir_all(&lease_dir).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to create IP lease dir {}: {err}", lease_dir.display()),
            )
        })?;
        for value in start..=end {
            let ip = std::net::Ipv4Addr::from(value).to_string();
            let lease_path = lease_dir.join(format!("{ip}.lease"));
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lease_path)
            {
                Ok(mut file) => {
                    use std::io::Write as _;
                    let _ = writeln!(file, "{ip}");
                    let dns_servers = if self.dns_servers.is_empty() {
                        vec![gateway.clone()]
                    } else {
                        self.dns_servers.clone()
                    };
                    let rendered_nmconnection = format!(
                        "[connection]\nid=static\ntype=ethernet\ninterface-name={}\nautoconnect=true\n\n[ipv4]\nmethod=manual\naddress1={}/{},{}\ndns={};\n\n[ipv6]\nmethod=disabled\n",
                        interface,
                        ip,
                        prefix,
                        gateway,
                        dns_servers.join(";")
                    );
                    return Ok(VerificationNetworkConfig {
                        interface: Some(interface.clone()),
                        prefix: Some(prefix),
                        gateway: Some(gateway.clone()),
                        dns_servers: dns_servers.clone(),
                        allocated_ip: Some(ip.clone()),
                        lease_path: Some(lease_path.display().to_string()),
                        rendered_nmconnection: Some(rendered_nmconnection),
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => {
                    return Err(CoreError::new(
                        FailureClass::Apply,
                        format!("failed to allocate IP lease {}: {err}", lease_path.display()),
                    ))
                }
            }
        }
        Err(CoreError::new(
            FailureClass::Apply,
            format!("no free verification IPs available in pool {ip_pool}"),
        ))
    }

    fn allow_arp_fallback(&self) -> bool {
        std::env::var("CORE_OPS_VERIFY_ALLOW_ARP_FALLBACK")
            .ok()
            .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    }

    fn fetch_console_log(&self, guest: &LibvirtGuestHandle) -> Result<String, CoreError> {
        let log_path = guest
            .serial_log_path
            .as_deref()
            .ok_or_else(|| CoreError::new(FailureClass::Apply, "guest missing serial console log path"))?;
        if let Some(vm_host) = &guest.vm_host {
            let mut command = Command::new("ssh");
            command
                .arg(format!("{}@{}", self.ssh_user, vm_host))
                .arg(format!("sudo cat {}", shell_escape(log_path)));
            let output = command.output().map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format_launch_error(
                        &command,
                        "fetch guest serial console log over ssh",
                        &render_command(&command),
                        &err,
                    ),
                )
            })?;
            if !output.status.success() {
                return Err(CoreError::new(
                    FailureClass::Apply,
                    format!(
                        "failed to read guest serial console log: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                ));
            }
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        fs::read_to_string(log_path).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to read local guest serial console log {log_path}: {err}"),
            )
        })
    }
}

impl VerificationLibvirtBoundary for LibvirtCommandRunner {
    fn create_guest(
        &self,
        scenario: &VerificationScenarioDefinition,
        workspace_root: &Path,
    ) -> Result<LibvirtGuestHandle, CoreError> {
        let environment = scenario.effective_environment()?;
        let guest_root = workspace_root.join(&environment.guest.guest_name);
        fs::create_dir_all(&guest_root).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!(
                    "failed to create guest workspace {}: {err}",
                    guest_root.display()
                ),
            )
        })?;

        if !self.env_backed {
            fs::write(
                guest_root.join("domain.json"),
                serde_json::json!({
                    "scenario_id": scenario.scenario_id,
                    "guest_name": environment.guest.guest_name,
                    "connection_uri": self.connection_uri,
                })
                .to_string(),
            )
            .map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!("failed to write guest domain metadata: {err}"),
                )
            })?;
            return Ok(LibvirtGuestHandle {
                guest_name: format!("{}-{}", environment.guest.guest_name, scenario.scenario_id),
                domain_name: format!("{}-{}", environment.guest.guest_name, scenario.scenario_id),
                ssh_target: format!("{}@{}", DEFAULT_SSH_USER, environment.guest.guest_name),
                connection_uri: self.connection_uri.clone(),
                workspace_root: guest_root.display().to_string(),
                env_backed: false,
                network_mode: Some(self.network_mode.clone()),
                vm_host: self.vm_host.clone(),
                ssh_user: Some(self.ssh_user.clone()),
                ignition_path: None,
                local_butane_path: None,
                local_ignition_path: None,
                volume_name: None,
                assigned_ip: None,
                lease_path: None,
                rendered_network_config: None,
                serial_log_path: None,
                qemu_launch_log_path: None,
                readiness_payload: Some(VerificationGuestReadinessPayload {
                    run_id: workspace_root
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("synthetic-run")
                        .to_string(),
                    token: "synthetic-token".to_string(),
                    console_marker: VERIFICATION_READINESS_MARKER.to_string(),
                    service_name: VERIFICATION_READINESS_SERVICE_NAME.to_string(),
                    script_path: VERIFICATION_READINESS_SCRIPT_PATH.to_string(),
                }),
                readiness_evidence: None,
            });
        }

        let network = self.load_network_config()?;
        let readiness_payload = VerificationGuestReadinessPayload {
            run_id: workspace_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("run")
                .to_string(),
            token: generate_readiness_token(),
            console_marker: VERIFICATION_READINESS_MARKER.to_string(),
            service_name: VERIFICATION_READINESS_SERVICE_NAME.to_string(),
            script_path: VERIFICATION_READINESS_SCRIPT_PATH.to_string(),
        };
        let run_suffix = workspace_root
            .file_name()
            .and_then(|name| name.to_str())
            .map(sanitize_identifier)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "run".to_string());
        let domain_name = sanitize_identifier(&format!(
            "{}-{}-{run_suffix}",
            environment.guest.guest_name, scenario.scenario_id
        ));
        let volume_name = format!("{domain_name}.qcow2");
        let serial_log_path = self.install_log_targets(&domain_name)?;
        let (ignition_path, local_butane_path, local_ignition_path) =
            self.install_ignition(workspace_root, &domain_name, &network, &readiness_payload)?;
        if let Err(err) = self.create_overlay_volume(&volume_name) {
            self.cleanup_partial_guest(
                &domain_name,
                &volume_name,
                Some(&ignition_path),
                network.lease_path.as_deref(),
            );
            return Err(err);
        }
        let (memory_mb, vcpus) = match environment.guest.cpu_profile.as_str() {
            "small" => ("2048", "2"),
            "medium" => ("4096", "2"),
            _ => ("2048", "2"),
        };
        if let Err(err) =
            self.install_domain(
                &domain_name,
                &volume_name,
                &ignition_path,
                &serial_log_path,
                memory_mb,
                vcpus,
            )
        {
            self.cleanup_partial_guest(
                &domain_name,
                &volume_name,
                Some(&ignition_path),
                network.lease_path.as_deref(),
            );
            return Err(err);
        }
        sleep(Duration::from_secs(5));

        fs::write(
            guest_root.join("domain.json"),
            serde_json::json!({
                "scenario_id": scenario.scenario_id,
                "guest_name": environment.guest.guest_name,
                "domain_name": domain_name,
                "connection_uri": self.connection_uri,
                "ignition_path": ignition_path,
                "lease_path": network.lease_path,
                "readiness_run_id": readiness_payload.run_id,
                "readiness_token": readiness_payload.token,
            })
            .to_string(),
        )
        .map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write guest domain metadata: {err}"),
            )
        })?;
        let qemu_launch_log_path = format!("/var/log/libvirt/qemu/{}.log", domain_name);

        Ok(LibvirtGuestHandle {
            guest_name: environment.guest.guest_name,
            domain_name: domain_name.clone(),
            ssh_target: format!("{}@0.0.0.0", self.ssh_user),
            connection_uri: self.connection_uri.clone(),
            workspace_root: guest_root.display().to_string(),
            env_backed: true,
            network_mode: Some(self.network_mode.clone()),
            vm_host: self.vm_host.clone(),
            ssh_user: Some(self.ssh_user.clone()),
            ignition_path: Some(ignition_path),
            local_butane_path: Some(local_butane_path),
            local_ignition_path: Some(local_ignition_path),
            volume_name: Some(volume_name),
            assigned_ip: None,
            lease_path: network.lease_path,
            rendered_network_config: network.rendered_nmconnection,
            serial_log_path: Some(serial_log_path),
            qemu_launch_log_path: Some(qemu_launch_log_path),
            readiness_payload: Some(readiness_payload),
            readiness_evidence: None,
        })
    }

    fn acquire_guest_readiness(
        &self,
        scenario: &VerificationScenarioDefinition,
        guest: &LibvirtGuestHandle,
    ) -> Result<VerificationReadinessAcquisition, CoreError> {
        if !guest.env_backed {
            let mut ready_guest = guest.clone();
            let accepted_record = VerificationReadinessRecord {
                run_id: guest
                    .readiness_payload
                    .as_ref()
                    .map(|payload| payload.run_id.clone())
                    .unwrap_or_else(|| "synthetic-run".to_string()),
                token: guest
                    .readiness_payload
                    .as_ref()
                    .map(|payload| payload.token.clone())
                    .unwrap_or_else(|| "synthetic-token".to_string()),
                ip: "192.0.2.10".to_string(),
                hostname: Some(guest.guest_name.clone()),
                ts: None,
            };
            ready_guest.assigned_ip = Some(accepted_record.ip.clone());
            ready_guest.ssh_target = format!("{}@{}", self.ssh_user, accepted_record.ip);
            let evidence = VerificationReadinessEvidence {
                source: "synthetic".to_string(),
                accepted_record: Some(accepted_record),
                rejected_records: Vec::new(),
                final_status: "accepted".to_string(),
                failure_summary: None,
            };
            ready_guest.readiness_evidence = Some(evidence.clone());
            return Ok(VerificationReadinessAcquisition {
                guest: ready_guest,
                evidence,
            });
        }

        let payload = guest.readiness_payload.as_ref().ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "guest readiness payload is missing from the libvirt guest handle",
            )
        })?;
        let expectation = VerificationReadinessExpectation {
            run_id: payload.run_id.clone(),
            token: payload.token.clone(),
            marker: payload.console_marker.clone(),
        };
        let timeout = parse_timeout_literal(&scenario.effective_timeouts()?.readiness_timeout)?;
        let deadline = std::time::Instant::now() + timeout;
        let mut rejected_records = Vec::new();
        let mut seen_lines = std::collections::BTreeSet::new();
        let mut accepted_record = None;
        let mut last_fetch_error = None;

        while std::time::Instant::now() < deadline && accepted_record.is_none() {
            let console_log = match self.fetch_console_log(guest) {
                Ok(console_log) => {
                    last_fetch_error = None;
                    console_log
                }
                Err(err) => {
                    last_fetch_error = Some(err.message);
                    sleep(Duration::from_secs(2));
                    continue;
                }
            };
            for line in console_log.lines().filter(|line| line.contains(&payload.console_marker)) {
                let line = line.trim();
                if line.is_empty() || !seen_lines.insert(line.to_string()) {
                    continue;
                }
                match evaluate_readiness_line(line, &expectation) {
                    Ok(candidate) => match accept_first_valid_readiness(&accepted_record, candidate, line) {
                        Ok(record) => accepted_record = Some(record),
                        Err(rejection) => rejected_records.push(rejection),
                    },
                    Err(rejection) => rejected_records.push(rejection),
                }
            }
            if accepted_record.is_none() {
                sleep(Duration::from_secs(2));
            }
        }

        if let Some(record) = accepted_record {
            let mut ready_guest = guest.clone();
            ready_guest.assigned_ip = Some(record.ip.clone());
            ready_guest.ssh_target = format!("{}@{}", self.ssh_user, record.ip);
            let evidence = VerificationReadinessEvidence {
                source: "serial-console".to_string(),
                accepted_record: Some(record),
                rejected_records,
                final_status: "accepted".to_string(),
                failure_summary: None,
            };
            ready_guest.readiness_evidence = Some(evidence.clone());
            return Ok(VerificationReadinessAcquisition {
                guest: ready_guest,
                evidence,
            });
        }

        if self.allow_arp_fallback() {
            let ip = self.wait_for_guest_ip(&guest.domain_name)?;
            let mut fallback_guest = guest.clone();
            fallback_guest.assigned_ip = Some(ip.clone());
            fallback_guest.ssh_target = format!("{}@{}", self.ssh_user, ip);
            let evidence = VerificationReadinessEvidence {
                source: "arp-fallback".to_string(),
                accepted_record: None,
                rejected_records,
                final_status: "fallback_used".to_string(),
                failure_summary: Some(match last_fetch_error {
                    Some(error) => format!(
                        "serial-console readiness was not accepted before fallback was used; last console-read error: {error}"
                    ),
                    None => {
                        "serial-console readiness was not accepted before fallback was used"
                            .to_string()
                    }
                }),
            };
            fallback_guest.readiness_evidence = Some(evidence.clone());
            return Ok(VerificationReadinessAcquisition {
                guest: fallback_guest,
                evidence,
            });
        }

        Ok(VerificationReadinessAcquisition {
            guest: guest.clone(),
            evidence: VerificationReadinessEvidence {
                source: "serial-console".to_string(),
                accepted_record: None,
                rejected_records,
                final_status: "timed_out".to_string(),
                failure_summary: Some(match last_fetch_error {
                    Some(error) => format!(
                        "no valid serial-console readiness record was accepted before the readiness deadline; last console-read error: {error}"
                    ),
                    None => {
                        "no valid serial-console readiness record was accepted before the readiness deadline"
                            .to_string()
                    }
                }),
            },
        })
    }

    fn destroy_guest(&self, guest: &LibvirtGuestHandle) -> Result<(), CoreError> {
        if guest.domain_name.trim().is_empty() {
            return Err(CoreError::new(
                FailureClass::Apply,
                "guest handle must contain a domain_name",
            ));
        }

        if guest.env_backed {
            let _ = self.virsh_command(&["destroy", &guest.domain_name]).output();
            let _ = self.virsh_command(&["undefine", &guest.domain_name]).output();
            if let Some(volume_name) = &guest.volume_name {
                let _ = self
                    .virsh_command(&["vol-delete", "--pool", &self.pool, volume_name])
                    .output();
            }
            if let Some(lease_path) = &guest.lease_path {
                let _ = fs::remove_file(lease_path);
            }
            if let (Some(vm_host), Some(ignition_path)) = (&self.vm_host, &guest.ignition_path) {
                let _ = Command::new("ssh")
                    .arg(format!("{}@{}", self.ssh_user, vm_host))
                    .arg(format!("sudo rm -f {}", shell_escape(ignition_path)))
                    .output();
            }
        }

        let guest_root = Path::new(&guest.workspace_root);
        if guest_root.exists() {
            fs::remove_dir_all(guest_root).map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!(
                        "failed to tear down guest workspace {}: {err}",
                        guest_root.display()
                    ),
                )
            })?;
        }
        Ok(())
    }
}

fn parse_domifaddr_ip(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        line.split_whitespace().find_map(|field| {
            field.split_once('/').and_then(|(ip, _)| {
                if ip.parse::<std::net::IpAddr>().is_ok() {
                    Some(ip.to_string())
                } else {
                    None
                }
            })
        })
    })
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sanitize_identifier(value: &str) -> String {
    value.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn parse_prefix(cidr: &str) -> Result<u8, CoreError> {
    cidr.split_once('/')
        .and_then(|(_, prefix)| prefix.parse::<u8>().ok())
        .filter(|prefix| *prefix <= 32)
        .ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                format!("invalid CORE_OPS_VERIFY_SUBNET_CIDR {cidr}"),
            )
        })
}

fn parse_ip_pool(pool: &str) -> Result<(u32, u32), CoreError> {
    let (start, end) = pool.split_once('-').ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            format!("invalid CORE_OPS_VERIFY_IP_POOL {pool}; expected start-end"),
        )
    })?;
    let start = parse_ipv4(start)?;
    let end = parse_ipv4(end)?;
    if start > end {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!("invalid CORE_OPS_VERIFY_IP_POOL {pool}; start exceeds end"),
        ));
    }
    Ok((start, end))
}

fn parse_ipv4(value: &str) -> Result<u32, CoreError> {
    value
        .parse::<std::net::Ipv4Addr>()
        .map(u32::from)
        .map_err(|_| {
            CoreError::new(
                FailureClass::Validation,
                format!("invalid IPv4 address {value}"),
            )
        })
}

fn shellexpand(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{stripped}");
        }
    }
    path.to_string()
}

fn generate_readiness_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("r{:x}", nanos)
}

fn render_readiness_script(payload: &VerificationGuestReadinessPayload) -> String {
    format!(
        "#!/usr/bin/bash\nset -euo pipefail\nip=\"$(ip -o -4 addr show scope global | awk '$2 != \"lo\" {{ split($4, a, \"/\"); print a[1]; exit }}')\"\nif [ -z \"$ip\" ]; then\n  exit 1\nfi\nhostname_value=\"$(hostname)\"\nts_value=\"$(date -u +%FT%TZ)\"\nprintf '%s {{\"run_id\":\"%s\",\"token\":\"%s\",\"ip\":\"%s\",\"hostname\":\"%s\",\"ts\":\"%s\"}}\\n' \\\n  '{marker}' \\\n  '{run_id}' \\\n  '{token}' \\\n  \"$ip\" \\\n  \"$hostname_value\" \\\n  \"$ts_value\"\n",
        marker = payload.console_marker,
        run_id = payload.run_id,
        token = payload.token,
    )
}

fn render_readiness_service(payload: &VerificationGuestReadinessPayload) -> String {
    format!(
        "[Unit]\nDescription=CoreOps verification guest readiness emitter\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=oneshot\nExecStart={}\nStandardOutput=journal+console\nStandardError=journal+console\nRemainAfterExit=yes\n\n[Install]\nWantedBy=multi-user.target\n",
        payload.script_path
    )
}

fn indent_block(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_command(command: &Command) -> String {
    let mut rendered = command.get_program().to_string_lossy().to_string();
    for arg in command.get_args() {
        rendered.push(' ');
        rendered.push_str(&shell_escape(&arg.to_string_lossy()));
    }
    rendered
}

fn format_launch_error(
    command: &Command,
    context: &str,
    rendered: &str,
    err: &std::io::Error,
) -> String {
    let executable = command.get_program().to_string_lossy();
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "failed to launch {context}: executable `{executable}` not found while running {rendered}: {err}"
        )
    } else {
        format!("failed to launch {context}: {rendered}: {err}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_launch_error;
    use std::process::Command;

    #[test]
    fn launch_error_identifies_missing_executable() {
        let mut command = Command::new("virt-install");
        command.arg("--name").arg("demo");
        let rendered = "virt-install '--name' 'demo'";
        let err = std::io::Error::from(std::io::ErrorKind::NotFound);

        let message = format_launch_error(&command, "launch guest", rendered, &err);

        assert!(message.contains("executable `virt-install` not found"));
        assert!(message.contains(rendered));
    }
}
