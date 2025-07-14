# SecBeat Terraform Configuration
# Configured for Proxmox deployment at 192.168.100.23

# Proxmox connection details
proxmox_host     = "192.168.100.23"
proxmox_user     = "root@pam"
proxmox_password = "invaders"

# SSH public key for VM access
ssh_public_key = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQDYJovSB5jgiPS6ErG3y03lL4Nlz4N0y8tgA+AUX6e1wUsxc3a2vo9lHryRb2cdHdlBFez92p/m3cn2crL/8P+nSBwYZnhgIBj/kt3wj5fb5XVXS9V6dCmeFrPb177WsLDqE8wmwjoWg/HY0SYZJqpdy/EGZ60Bz1VALPthctEdGO3Rpq/7/3d/VolE14Iy/42A99rac7tRrlUWL9u1a/Tlb1JgAWNYYyr7pxKIEMzsn8ecOhcn9iGQRdVwI1cns4D46dQXvFhmYmn9RwbJXwcKtaKg6qChktqkkSdqe5XwMQl3C/BvWTL+3Tvu/+pdDre3+flhREycUytiFWlBDwB2OFSKw81YoKqsZ5yUja4XUhiDJV7BpCurEYdQqrE8IeBbjuQLuYfQsd0EVXiFScdg4Ae475vR1Ge1KRrZxpdNHAKq6gtmJyDqeNMvIPYHRtBcwbGZJnEeZm08RGVCTIvWf522qnRK4Ch7M2OizMFYFh0/JAtM0BMFUriq3F1bRMk= fab@m4.local"

# VM template and storage (using defaults optimized for your setup)
vm_template  = "ubuntu-24.04-server"
storage_pool = "local-lvm"
target_node  = "proxmox-lab"
