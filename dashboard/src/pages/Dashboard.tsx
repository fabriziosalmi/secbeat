import { useEffect, useState } from 'react';
import { Activity, Shield, Server, TrendingUp, AlertTriangle } from 'lucide-react';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { StatCard } from '../components/StatCard';
import { apiClient } from '../api/client';
import type { DashboardSummary } from '../types/api';

interface TrafficDataPoint {
  time: string;
  requests: number;
  blocked: number;
}

export function Dashboard() {
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [trafficHistory, setTrafficHistory] = useState<TrafficDataPoint[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const data = await apiClient.getDashboardSummary();
        setSummary(data);
        setError(null);

        // Add to traffic history (keep last 20 data points)
        const now = new Date().toLocaleTimeString();
        setTrafficHistory((prev) => {
          const newHistory = [
            ...prev,
            {
              time: now,
              requests: data.total_requests,
              blocked: data.blocked_requests,
            },
          ].slice(-20);
          return newHistory;
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch data');
      }
    };

    // Initial fetch
    fetchData();

    // Poll every 2 seconds
    const interval = setInterval(fetchData, 2000);

    return () => clearInterval(interval);
  }, []);

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="bg-red-50 border border-red-200 rounded-lg p-6 max-w-md">
          <div className="flex items-center gap-3 mb-2">
            <AlertTriangle className="w-5 h-5 text-red-600" />
            <h3 className="text-lg font-semibold text-red-900">Connection Error</h3>
          </div>
          <p className="text-red-700">{error}</p>
          <p className="text-sm text-red-600 mt-2">
            Make sure the orchestrator is running on http://localhost:3030
          </p>
        </div>
      </div>
    );
  }

  if (!summary) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="flex items-center gap-3">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <p className="text-gray-600">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  const formatNumber = (num: number): string => {
    if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
    if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
    return num.toString();
  };

  const getHealthColor = (health: string): 'green' | 'yellow' | 'red' => {
    switch (health) {
      case 'healthy':
        return 'green';
      case 'degraded':
        return 'yellow';
      default:
        return 'red';
    }
  };

  return (
    <div className="space-y-6">
      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatCard
          title="Active Nodes"
          value={summary.active_nodes}
          icon={Server}
          color="blue"
        />
        <StatCard
          title="Requests/Min"
          value={formatNumber(summary.requests_per_minute)}
          icon={TrendingUp}
          color="green"
        />
        <StatCard
          title="Total Blocks"
          value={formatNumber(summary.blocked_requests)}
          icon={Shield}
          color="red"
        />
        <StatCard
          title="Block Rate"
          value={`${summary.block_rate.toFixed(1)}%`}
          icon={Activity}
          color="purple"
        />
      </div>

      {/* Cluster Health Banner */}
      <div className={`rounded-lg p-4 border ${
        summary.cluster_health === 'healthy' 
          ? 'bg-green-50 border-green-200' 
          : summary.cluster_health === 'degraded'
          ? 'bg-yellow-50 border-yellow-200'
          : 'bg-red-50 border-red-200'
      }`}>
        <div className="flex items-center gap-3">
          <div className={`w-3 h-3 rounded-full ${
            summary.cluster_health === 'healthy' ? 'bg-green-500' :
            summary.cluster_health === 'degraded' ? 'bg-yellow-500' :
            'bg-red-500'
          } animate-pulse`}></div>
          <span className={`font-semibold ${
            summary.cluster_health === 'healthy' ? 'text-green-900' :
            summary.cluster_health === 'degraded' ? 'text-yellow-900' :
            'text-red-900'
          }`}>
            Cluster Status: {summary.cluster_health.toUpperCase()}
          </span>
          <span className={`text-sm ${
            summary.cluster_health === 'healthy' ? 'text-green-700' :
            summary.cluster_health === 'degraded' ? 'text-yellow-700' :
            'text-red-700'
          }`}>
            • Last updated {new Date(summary.timestamp).toLocaleTimeString()}
          </span>
        </div>
      </div>

      {/* Traffic Chart */}
      <div className="bg-white rounded-lg shadow p-6 border border-gray-200">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Traffic Overview</h2>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={trafficHistory}>
            <CartesianGrid strokeDasharray="3 3" />
            <XAxis
              dataKey="time"
              tick={{ fontSize: 12 }}
              interval="preserveStartEnd"
            />
            <YAxis tick={{ fontSize: 12 }} />
            <Tooltip />
            <Line
              type="monotone"
              dataKey="requests"
              stroke="#3b82f6"
              strokeWidth={2}
              name="Total Requests"
              dot={false}
            />
            <Line
              type="monotone"
              dataKey="blocked"
              stroke="#ef4444"
              strokeWidth={2}
              name="Blocked"
              dot={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* System Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white rounded-lg shadow p-6 border border-gray-200">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Fleet Status</h3>
          <dl className="space-y-3">
            <div className="flex justify-between">
              <dt className="text-gray-600">Total PPS</dt>
              <dd className="font-semibold text-gray-900">{formatNumber(summary.total_pps)}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-600">Total Requests</dt>
              <dd className="font-semibold text-gray-900">{formatNumber(summary.total_requests)}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-600">Active Nodes</dt>
              <dd className="font-semibold text-blue-600">{summary.active_nodes}</dd>
            </div>
          </dl>
        </div>

        <div className="bg-white rounded-lg shadow p-6 border border-gray-200">
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Protection Summary</h3>
          <dl className="space-y-3">
            <div className="flex justify-between">
              <dt className="text-gray-600">Blocked Requests</dt>
              <dd className="font-semibold text-red-600">{formatNumber(summary.blocked_requests)}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-600">Block Rate</dt>
              <dd className="font-semibold text-gray-900">{summary.block_rate.toFixed(2)}%</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-gray-600">Requests/Min</dt>
              <dd className="font-semibold text-green-600">{formatNumber(summary.requests_per_minute)}</dd>
            </div>
          </dl>
        </div>
      </div>
    </div>
  );
}
