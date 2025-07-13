# Ansible Inventory for SecBeat Deployment
# Static template for hybrid deployment

[mitigation_nodes]
%{ for idx, ip in mitigation_ips ~}
secbeat-mitigation-${idx + 1} ansible_host=${ip} ansible_user=secbeat
%{ endfor ~}

[orchestrator]
secbeat-orchestrator ansible_host=${orchestrator_ip} ansible_user=secbeat

[nats_cluster]
%{ for idx, ip in nats_ips ~}
secbeat-nats-${idx + 1} ansible_host=${ip} ansible_user=secbeat
%{ endfor ~}

[load_balancers]
%{ for idx, ip in lb_ips ~}
secbeat-lb-${idx + 1} ansible_host=${ip} ansible_user=secbeat
%{ endfor ~}

[monitoring]
secbeat-monitoring ansible_host=${monitoring_ip} ansible_user=secbeat

[secbeat:children]
mitigation_nodes
orchestrator
nats_cluster
load_balancers
monitoring

[secbeat:vars]
ansible_ssh_private_key_file=~/.ssh/id_rsa
ansible_ssh_common_args='-o StrictHostKeyChecking=no'
ansible_python_interpreter=/usr/bin/python3
