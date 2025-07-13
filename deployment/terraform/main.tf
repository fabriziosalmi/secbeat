# SecBeat Proxmox Infrastructure with Terraform
# This configuration deploys a complete SecBeat platform on Proxmox VE

terraform {
  required_version = ">= 1.0"
  required_providers {
    proxmox = {
      source  = "telmate/proxmox"
      version = "~> 2.9"
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
  default     = "root@pam"
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
  default     = "ubuntu-22.04-server"
}

variable "storage_pool" {
  description = "Proxmox storage pool"
  type        = string
  default     = "local-lvm"
}

# Provider configuration
provider "proxmox" {
  pm_api_url      = "https://${var.proxmox_host}:8006/api2/json"
  pm_user         = var.proxmox_user
  pm_password     = var.proxmox_password
  pm_tls_insecure = true
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

  validity_period_hours = 8760  # 1 year

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
    "192.168.200.10",
    "192.168.200.11", 
    "192.168.200.12",
    "192.168.200.20"
  ]
}

# Cloud-init configuration template
locals {
  cloud_init_config = templatefile("${path.module}/cloud-init.yml.tpl", {
    ssh_public_key = var.ssh_public_key
    secbeat_cert   = base64encode(tls_self_signed_cert.secbeat_cert.cert_pem)
    secbeat_key    = base64encode(tls_private_key.secbeat_key.private_key_pem)
  })
}

# Mitigation Node VMs
resource "proxmox_vm_qemu" "mitigation_nodes" {
  count       = 3
  name        = "secbeat-mitigation-${count.index + 1}"
  target_node = "pve"  # Adjust to your Proxmox node name
  clone       = var.vm_template
  
  # VM Resources
  cores  = 4
  memory = 8192
  
  # Disk configuration
  disk {
    storage = var.storage_pool
    type    = "scsi"
    size    = "40G"
    format  = "qcow2"
  }
  
  # Network configuration
  network {
    model  = "virtio"
    bridge = "vmbr0"
  }
  
  # Cloud-init
  os_type = "cloud-init"
  ipconfig0 = "ip=192.168.200.${10 + count.index}/24,gw=192.168.200.1"
  
  # SSH configuration
  sshkeys = var.ssh_public_key
  
  # Start VM automatically
  onboot = true
  
  # Wait for cloud-init to complete
  define_connection_info = false
  
  tags = "secbeat,mitigation,production"
}

# Orchestrator Node VM
resource "proxmox_vm_qemu" "orchestrator" {
  name        = "secbeat-orchestrator"
  target_node = "pve"
  clone       = var.vm_template
  
  cores  = 2
  memory = 4096
  
  disk {
    storage = var.storage_pool
    type    = "scsi"
    size    = "20G"
    format  = "qcow2"
  }
  
  network {
    model  = "virtio"
    bridge = "vmbr0"
  }
  
  os_type = "cloud-init"
  ipconfig0 = "ip=192.168.200.20/24,gw=192.168.200.1"
  
  sshkeys = var.ssh_public_key
  onboot = true
  
  tags = "secbeat,orchestrator,production"
}

# NATS Cluster VMs
resource "proxmox_vm_qemu" "nats_cluster" {
  count       = 3
  name        = "secbeat-nats-${count.index + 1}"
  target_node = "pve"
  clone       = var.vm_template
  
  cores  = 2
  memory = 2048
  
  disk {
    storage = var.storage_pool
    type    = "scsi"
    size    = "10G"
    format  = "qcow2"
  }
  
  network {
    model  = "virtio"
    bridge = "vmbr0"
  }
  
  os_type = "cloud-init"
  ipconfig0 = "ip=192.168.200.${30 + count.index}/24,gw=192.168.200.1"
  
  sshkeys = var.ssh_public_key
  onboot = true
  
  tags = "secbeat,nats,messaging"
}

# Load Balancer VMs (HAProxy)
resource "proxmox_vm_qemu" "load_balancers" {
  count       = 2
  name        = "secbeat-lb-${count.index + 1}"
  target_node = "pve"
  clone       = var.vm_template
  
  cores  = 2
  memory = 2048
  
  disk {
    storage = var.storage_pool
    type    = "scsi"
    size    = "10G"
    format  = "qcow2"
  }
  
  network {
    model  = "virtio"
    bridge = "vmbr0"
  }
  
  os_type = "cloud-init"
  ipconfig0 = "ip=192.168.200.${40 + count.index}/24,gw=192.168.200.1"
  
  sshkeys = var.ssh_public_key
  onboot = true
  
  tags = "secbeat,loadbalancer,production"
}

# Monitoring VM (Prometheus + Grafana)
resource "proxmox_vm_qemu" "monitoring" {
  name        = "secbeat-monitoring"
  target_node = "pve"
  clone       = var.vm_template
  
  cores  = 4
  memory = 8192
  
  disk {
    storage = var.storage_pool
    type    = "scsi"
    size    = "60G"
    format  = "qcow2"
  }
  
  network {
    model  = "virtio"
    bridge = "vmbr0"
  }
  
  os_type = "cloud-init"
  ipconfig0 = "ip=192.168.300.10/24,gw=192.168.300.1"
  
  sshkeys = var.ssh_public_key
  onboot = true
  
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
    total_vms = length(proxmox_vm_qemu.mitigation_nodes) + 1 + length(proxmox_vm_qemu.nats_cluster) + length(proxmox_vm_qemu.load_balancers) + 1
    mitigation_nodes = length(proxmox_vm_qemu.mitigation_nodes)
    orchestrator_nodes = 1
    nats_nodes = length(proxmox_vm_qemu.nats_cluster)
    load_balancer_nodes = length(proxmox_vm_qemu.load_balancers)
    monitoring_nodes = 1
  }
}
