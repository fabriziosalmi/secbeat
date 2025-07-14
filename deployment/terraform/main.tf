# SecBeat Proxmox Infrastructure with Terraform
# This configuration deploys a complete SecBeat platform on Proxmox VE

terraform {
  required_version = ">= 1.0"
  required_providers {
    proxmox = {
      source  = "telmate/proxmox"
      version = "3.0.1-rc9"
    }
    local = {
      source  = "hashicorp/local"
      version = "~> 2.1"
    }
    tls = {
      source  = "hashicorp/tls"
      version = "~> 4.0"
    }
  }
}

# Variables
variable "proxmox_host" {
  description = "Proxmox host address"
  type        = string
  default     = "192.168.100.23"
}

variable "proxmox_user" {
  description = "Proxmox username"
  type        = string
  default     = "root"
}

variable "proxmox_password" {
  description = "Proxmox password"
  type        = string
  sensitive   = true
}

variable "ssh_public_key" {
  description = "SSH public key for VM access"
  type        = string
}

variable "vm_template" {
  description = "VM template name"
  type        = string
  default     = "ubuntu-24.04-server"
}

variable "target_node" {
  description = "Proxmox target node name"
  type        = string
  default     = "proxmox-lab"
}

variable "storage_pool" {
  description = "Proxmox storage pool"
  type        = string
  default     = "local-lvm"
}

# Provider configuration
# Using environment variables: PM_API_URL, PM_USER, PM_PASS
provider "proxmox" {
  pm_tls_insecure = true
  pm_timeout      = 600
}

# Local variables for VM configuration
locals {
  cloud_image = "ubuntu-24.04-server-cloudimg-amd64.img"
  storage     = "local"
  bridge      = "vmbr0"

  # VM configurations matching your original setup
  vm_configs = {
    mitigation_nodes = {
      count   = 3
      cores   = 1
      memory  = 768
      disk    = 8
      base_ip = 200
    }
    orchestrator = {
      cores  = 1
      memory = 768
      disk   = 8
      ip     = 203
    }
    nats_cluster = {
      count   = 3
      cores   = 1
      memory  = 512
      disk    = 6
      base_ip = 204
    }
    load_balancers = {
      count   = 2
      cores   = 1
      memory  = 512
      disk    = 6
      base_ip = 207
    }
    monitoring = {
      cores  = 1
      memory = 512
      disk   = 12
      ip     = 209
    }
  }
}

# Generate TLS certificates for SecBeat
resource "tls_private_key" "secbeat_key" {
  algorithm = "RSA"
  rsa_bits  = 4096
}

resource "tls_self_signed_cert" "secbeat_cert" {
  private_key_pem = tls_private_key.secbeat_key.private_key_pem

  subject {
    common_name  = "secbeat.local"
    organization = "SecBeat Security Platform"
  }

  validity_period_hours = 8760 # 1 year

  allowed_uses = [
    "key_encipherment",
    "digital_signature",
    "server_auth",
  ]

  dns_names = [
    "secbeat.local",
    "*.secbeat.local",
    "mitigation-1.secbeat.local",
    "mitigation-2.secbeat.local",
    "mitigation-3.secbeat.local",
    "orchestrator.secbeat.local"
  ]

  ip_addresses = [
    "192.168.100.200",
    "192.168.100.201",
    "192.168.100.202",
    "192.168.100.203"
  ]
}

# Cloud-init configuration template
locals {
  cloud_init_config = templatefile("${path.module}/cloud-init.yml.tpl", {
    hostname       = "secbeat-vm"
    ssh_public_key = var.ssh_public_key
    secbeat_cert   = base64encode(tls_self_signed_cert.secbeat_cert.cert_pem)
    secbeat_key    = base64encode(tls_private_key.secbeat_key.private_key_pem)
  })
}

# Mitigation Node VMs
resource "proxmox_vm_qemu" "mitigation_nodes" {
  count       = local.vm_configs.mitigation_nodes.count
  name        = "secbeat-mitigation-${count.index + 1}"
  target_node = var.target_node
  vmid        = 200 + count.index

  # Clone from template
  clone = var.vm_template
  kvm   = false

  # VM Resources
  cpu {
    cores = local.vm_configs.mitigation_nodes.cores
    type  = "qemu64"
  }
  memory   = local.vm_configs.mitigation_nodes.memory
  scsihw   = "virtio-scsi-pci"
  bootdisk = "scsi0"

  # Disk configuration
  disk {
    slot    = "scsi0"
    type    = "disk"
    storage = var.storage_pool
    size    = "${local.vm_configs.mitigation_nodes.disk}G"
    format  = "raw"
  }

  # Network configuration
  network {
    id     = 0
    model  = "virtio"
    bridge = local.bridge
  }

  # Cloud-init configuration
  os_type    = "cloud-init"
  ipconfig0  = "ip=192.168.100.${local.vm_configs.mitigation_nodes.base_ip + count.index}/24,gw=192.168.100.1"
  nameserver = "8.8.8.8,8.8.4.4"

  # SSH configuration
  sshkeys = var.ssh_public_key

  # Start VM automatically
  onboot = false

  tags = "secbeat,mitigation,production"
}

# Orchestrator Node VM
resource "proxmox_vm_qemu" "orchestrator" {
  name        = "secbeat-orchestrator"
  target_node = var.target_node
  vmid        = 210

  # Clone from template
  clone = var.vm_template
  kvm   = false

  cpu {
    cores = local.vm_configs.orchestrator.cores
    type  = "qemu64"
  }
  memory   = local.vm_configs.orchestrator.memory
  scsihw   = "virtio-scsi-pci"
  bootdisk = "scsi0"

  disk {
    slot    = "scsi0"
    type    = "disk"
    storage = var.storage_pool
    size    = "${local.vm_configs.orchestrator.disk}G"
    format  = "raw"
  }

  network {
    id     = 0
    model  = "virtio"
    bridge = local.bridge
  }

  os_type    = "cloud-init"
  ipconfig0  = "ip=192.168.100.${local.vm_configs.orchestrator.ip}/24,gw=192.168.100.1"
  nameserver = "8.8.8.8 8.8.4.4"

  sshkeys = var.ssh_public_key
  onboot  = false

  tags = "secbeat,orchestrator,production"
}

# NATS Cluster VMs
resource "proxmox_vm_qemu" "nats_cluster" {
  count       = local.vm_configs.nats_cluster.count
  name        = "secbeat-nats-${count.index + 1}"
  target_node = var.target_node
  vmid        = 220 + count.index

  # Clone from template
  clone = var.vm_template
  kvm   = false

  cpu {
    cores = local.vm_configs.nats_cluster.cores
    type  = "qemu64"
  }
  memory   = local.vm_configs.nats_cluster.memory
  scsihw   = "virtio-scsi-pci"
  bootdisk = "scsi0"

  disk {
    slot    = "scsi0"
    type    = "disk"
    storage = var.storage_pool
    size    = "${local.vm_configs.nats_cluster.disk}G"
    format  = "raw"
  }

  network {
    id     = 0
    model  = "virtio"
    bridge = local.bridge
  }

  os_type    = "cloud-init"
  ipconfig0  = "ip=192.168.100.${local.vm_configs.nats_cluster.base_ip + count.index}/24,gw=192.168.100.1"
  nameserver = "8.8.8.8 8.8.4.4"

  sshkeys = var.ssh_public_key
  onboot  = false

  tags = "secbeat,nats,messaging"
}

# Load Balancer VMs (HAProxy)
resource "proxmox_vm_qemu" "load_balancers" {
  count       = local.vm_configs.load_balancers.count
  name        = "secbeat-lb-${count.index + 1}"
  target_node = var.target_node
  vmid        = 230 + count.index

  # Clone from template
  clone = var.vm_template
  kvm   = false

  disk {
    slot    = "scsi0"
    type    = "disk"
    storage = var.storage_pool
    size    = "${local.vm_configs.load_balancers.disk}G"
    format  = "raw"
  }

  cpu {
    cores = local.vm_configs.load_balancers.cores
    type  = "qemu64"
  }
  memory   = local.vm_configs.load_balancers.memory
  scsihw   = "virtio-scsi-pci"
  bootdisk = "scsi0"

  network {
    id     = 0
    model  = "virtio"
    bridge = local.bridge
  }

  os_type    = "cloud-init"
  ipconfig0  = "ip=192.168.100.${local.vm_configs.load_balancers.base_ip + count.index}/24,gw=192.168.100.1"
  nameserver = "8.8.8.8 8.8.4.4"

  sshkeys = var.ssh_public_key
  onboot  = false

  tags = "secbeat,loadbalancer,production"
}

# Monitoring VM (Prometheus + Grafana)
resource "proxmox_vm_qemu" "monitoring" {
  name        = "secbeat-monitoring"
  target_node = var.target_node
  vmid        = 240

  # Clone from template
  clone = var.vm_template
  kvm   = false

  cpu {
    cores = local.vm_configs.monitoring.cores
    type  = "qemu64"
  }
  memory   = local.vm_configs.monitoring.memory
  scsihw   = "virtio-scsi-pci"
  bootdisk = "scsi0"

  disk {
    slot    = "scsi0"
    type    = "disk"
    storage = var.storage_pool
    size    = "${local.vm_configs.monitoring.disk}G"
    format  = "raw"
  }

  network {
    id     = 0
    model  = "virtio"
    bridge = local.bridge
  }

  os_type    = "cloud-init"
  ipconfig0  = "ip=192.168.100.${local.vm_configs.monitoring.ip}/24,gw=192.168.100.1"
  nameserver = "8.8.8.8 8.8.4.4"

  sshkeys = var.ssh_public_key
  onboot  = false

  tags = "secbeat,monitoring,prometheus,grafana"
}

# Generate inventory file for Ansible
resource "local_file" "ansible_inventory" {
  filename = "${path.module}/../ansible/inventory.ini"
  content = templatefile("${path.module}/inventory.ini.tpl", {
    mitigation_nodes = proxmox_vm_qemu.mitigation_nodes
    orchestrator     = proxmox_vm_qemu.orchestrator
    nats_cluster     = proxmox_vm_qemu.nats_cluster
    load_balancers   = proxmox_vm_qemu.load_balancers
    monitoring       = proxmox_vm_qemu.monitoring
  })
}

# Generate TLS certificates for deployment
resource "local_file" "secbeat_cert" {
  filename = "${path.module}/../certificates/secbeat.pem"
  content  = tls_self_signed_cert.secbeat_cert.cert_pem
}

resource "local_file" "secbeat_key" {
  filename = "${path.module}/../certificates/secbeat.key"
  content  = tls_private_key.secbeat_key.private_key_pem
}

# Outputs
output "mitigation_node_ips" {
  value = proxmox_vm_qemu.mitigation_nodes[*].default_ipv4_address
}

output "orchestrator_ip" {
  value = proxmox_vm_qemu.orchestrator.default_ipv4_address
}

output "nats_cluster_ips" {
  value = proxmox_vm_qemu.nats_cluster[*].default_ipv4_address
}

output "load_balancer_ips" {
  value = proxmox_vm_qemu.load_balancers[*].default_ipv4_address
}

output "monitoring_ip" {
  value = proxmox_vm_qemu.monitoring.default_ipv4_address
}

output "deployment_summary" {
  value = {
    total_vms           = length(proxmox_vm_qemu.mitigation_nodes) + 1 + length(proxmox_vm_qemu.nats_cluster) + length(proxmox_vm_qemu.load_balancers) + 1
    mitigation_nodes    = length(proxmox_vm_qemu.mitigation_nodes)
    orchestrator_nodes  = 1
    nats_nodes          = length(proxmox_vm_qemu.nats_cluster)
    load_balancer_nodes = length(proxmox_vm_qemu.load_balancers)
    monitoring_nodes    = 1
  }
}
