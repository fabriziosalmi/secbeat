# SecBeat Proxmox Infrastructure with Terraform (Simplified)
# This is a simplified version that can be enhanced once basic deployment works

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

# Provider configuration
provider "proxmox" {
  pm_api_url      = "https://${var.proxmox_host}:8006/api2/json"
  pm_user         = var.proxmox_user
  pm_password     = var.proxmox_password
  pm_tls_insecure = true
  pm_timeout      = 600
}

# Generate Ansible inventory file
resource "local_file" "ansible_inventory" {
  filename = "${path.module}/../ansible/inventory.ini"
  content = templatefile("${path.module}/inventory.ini.tpl", {
    # Static configuration for now - can be enhanced with actual VM resources later
    mitigation_ips = ["192.168.100.200", "192.168.100.201", "192.168.100.202"]
    orchestrator_ip = "192.168.100.203"
    nats_ips = ["192.168.100.204", "192.168.100.205", "192.168.100.206"]
    lb_ips = ["192.168.100.207", "192.168.100.208"]
    monitoring_ip = "192.168.100.209"
  })
}

# Outputs for reference
output "deployment_info" {
  value = {
    mitigation_ips = ["192.168.100.200", "192.168.100.201", "192.168.100.202"]
    orchestrator_ip = "192.168.100.203"
    nats_ips = ["192.168.100.204", "192.168.100.205", "192.168.100.206"]
    load_balancer_ips = ["192.168.100.207", "192.168.100.208"]
    monitoring_ip = "192.168.100.209"
    grafana_url = "http://192.168.100.209:3000"
    prometheus_url = "http://192.168.100.209:9090"
    haproxy_stats = "http://192.168.100.207:8404/stats"
  }
}
