// API Response Types matching Orchestrator Rust structs

export interface DashboardSummary {
  total_requests: number;
  blocked_requests: number;
  active_nodes: number;
  cluster_health: 'healthy' | 'degraded' | 'critical';
  requests_per_minute: number;
  total_pps: number;
  block_rate: number;
  timestamp: string;
}

export interface AttackEvent {
  timestamp: string;
  source_ip: string;
  attack_type: string;
  node_id: string;
  action: string;
  uri?: string;
}

export interface DashboardAttacksResponse {
  attacks: AttackEvent[];
  total_count: number;
}

export interface NodeInfo {
  node_id: string;
  public_ip: string;
  last_heartbeat: string;
  status: 'Active' | 'Registered' | 'Dead' | 'Draining';
  metrics?: NodeMetrics;
  registered_at: string;
  config: NodeConfig;
}

export interface NodeMetrics {
  cpu_usage: number;
  memory_usage: number;
  packets_per_second: number;
  active_connections: number;
  total_requests: number;
  ddos_blocks: number;
  waf_blocks: number;
}

export interface NodeConfig {
  node_type: string;
  region?: string;
  tags: string[];
}
