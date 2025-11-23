import { useEffect, useState } from 'react';
import { Server, Activity, Cpu, HardDrive, AlertCircle } from 'lucide-react';
import { apiClient } from '../api/client';
import type { NodeInfo } from '../types/api';

export function Nodes() {
  const [nodes, setNodes] = useState<NodeInfo[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchNodes = async () => {
      try {
        const data = await apiClient.getNodes();
        setNodes(data);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch nodes');
      } finally {
        setLoading(false);
      }
    };

    fetchNodes();
    const interval = setInterval(fetchNodes, 3000);
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4">
        <div className="flex items-center gap-2">
          <AlertCircle className="w-5 h-5 text-red-600" />
          <p className="text-red-700">{error}</p>
        </div>
      </div>
    );
  }

  const getStatusColor = (status: string): string => {
    switch (status) {
      case 'Active':
        return 'bg-green-100 text-green-800 border-green-200';
      case 'Registered':
        return 'bg-blue-100 text-blue-800 border-blue-200';
      case 'Draining':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      default:
        return 'bg-red-100 text-red-800 border-red-200';
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">Mitigation Nodes</h2>
          <p className="text-gray-600 mt-1">Active fleet members and their status</p>
        </div>
        <div className="text-sm text-gray-600">
          Total: <span className="font-semibold">{nodes.length}</span> nodes
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {nodes.map((node) => (
          <div
            key={node.node_id}
            className="bg-white rounded-lg shadow border border-gray-200 p-6"
          >
            <div className="flex items-start justify-between mb-4">
              <div className="flex items-center gap-3">
                <Server className="w-8 h-8 text-blue-600" />
                <div>
                  <h3 className="font-semibold text-gray-900">
                    {node.config.node_type}
                  </h3>
                  <p className="text-sm text-gray-500">{node.public_ip}</p>
                </div>
              </div>
              <span
                className={`px-2 py-1 text-xs font-medium rounded border ${getStatusColor(
                  node.status
                )}`}
              >
                {node.status}
              </span>
            </div>

            {node.metrics && (
              <div className="space-y-3 mt-4">
                <div className="flex items-center justify-between text-sm">
                  <div className="flex items-center gap-2 text-gray-600">
                    <Cpu className="w-4 h-4" />
                    <span>CPU</span>
                  </div>
                  <span className="font-semibold text-gray-900">
                    {node.metrics.cpu_usage.toFixed(1)}%
                  </span>
                </div>

                <div className="flex items-center justify-between text-sm">
                  <div className="flex items-center gap-2 text-gray-600">
                    <HardDrive className="w-4 h-4" />
                    <span>Memory</span>
                  </div>
                  <span className="font-semibold text-gray-900">
                    {node.metrics.memory_usage.toFixed(1)}%
                  </span>
                </div>

                <div className="flex items-center justify-between text-sm">
                  <div className="flex items-center gap-2 text-gray-600">
                    <Activity className="w-4 h-4" />
                    <span>PPS</span>
                  </div>
                  <span className="font-semibold text-gray-900">
                    {node.metrics.packets_per_second.toLocaleString()}
                  </span>
                </div>

                <div className="pt-3 border-t border-gray-200">
                  <div className="flex justify-between text-sm">
                    <span className="text-gray-600">Total Requests</span>
                    <span className="font-semibold text-gray-900">
                      {node.metrics.total_requests.toLocaleString()}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm mt-1">
                    <span className="text-gray-600">Blocked</span>
                    <span className="font-semibold text-red-600">
                      {(node.metrics.ddos_blocks + node.metrics.waf_blocks).toLocaleString()}
                    </span>
                  </div>
                </div>
              </div>
            )}

            <div className="mt-4 pt-4 border-t border-gray-200">
              <p className="text-xs text-gray-500">
                Last heartbeat:{' '}
                {new Date(node.last_heartbeat).toLocaleTimeString()}
              </p>
              {node.config.region && (
                <p className="text-xs text-gray-500 mt-1">
                  Region: {node.config.region}
                </p>
              )}
            </div>
          </div>
        ))}
      </div>

      {nodes.length === 0 && (
        <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-8 text-center">
          <Server className="w-12 h-12 text-yellow-600 mx-auto mb-3" />
          <h3 className="text-lg font-semibold text-yellow-900 mb-2">
            No Nodes Registered
          </h3>
          <p className="text-yellow-700">
            Start a mitigation node to see it appear here
          </p>
        </div>
      )}
    </div>
  );
}
