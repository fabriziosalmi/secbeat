import { useEffect, useState } from 'react';
import { Shield, AlertOctagon, Clock, MapPin } from 'lucide-react';
import { apiClient } from '../api/client';
import type { AttackEvent } from '../types/api';

export function Attacks() {
  const [attacks, setAttacks] = useState<AttackEvent[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchAttacks = async () => {
      try {
        const data = await apiClient.getDashboardAttacks();
        setAttacks(data.attacks);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch attacks');
      }
    };

    fetchAttacks();
    const interval = setInterval(fetchAttacks, 2000);
    return () => clearInterval(interval);
  }, []);

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4">
        <p className="text-red-700">{error}</p>
      </div>
    );
  }

  const getActionColor = (action: string): string => {
    switch (action.toLowerCase()) {
      case 'block':
        return 'bg-red-100 text-red-800 border-red-200';
      case 'log':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      case 'ratelimit':
        return 'bg-orange-100 text-orange-800 border-orange-200';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const getAttackTypeColor = (type: string): string => {
    if (type.includes('Flood')) return 'text-red-600';
    if (type.includes('Injection')) return 'text-purple-600';
    if (type.includes('Traversal')) return 'text-orange-600';
    return 'text-gray-600';
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900">Recent Attacks</h2>
          <p className="text-gray-600 mt-1">Real-time security events from all nodes</p>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-red-500 animate-pulse"></div>
          <span className="text-sm text-gray-600">Live Feed</span>
        </div>
      </div>

      <div className="bg-white rounded-lg shadow border border-gray-200 overflow-hidden">
        <div className="overflow-x-auto">
          <table className="min-w-full divide-y divide-gray-200">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Time
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Source IP
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Attack Type
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  Action
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                  URI
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-gray-200">
              {attacks.map((attack, index) => (
                <tr key={`${attack.timestamp}-${index}`} className="hover:bg-gray-50">
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-2 text-sm text-gray-900">
                      <Clock className="w-4 h-4 text-gray-400" />
                      {new Date(attack.timestamp).toLocaleTimeString()}
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-2 text-sm font-mono text-gray-900">
                      <MapPin className="w-4 h-4 text-gray-400" />
                      {attack.source_ip}
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="flex items-center gap-2">
                      <AlertOctagon className={`w-4 h-4 ${getAttackTypeColor(attack.attack_type)}`} />
                      <span className={`text-sm font-medium ${getAttackTypeColor(attack.attack_type)}`}>
                        {attack.attack_type}
                      </span>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs font-medium rounded border ${getActionColor(
                        attack.action
                      )}`}
                    >
                      {attack.action}
                    </span>
                  </td>
                  <td className="px-6 py-4 text-sm text-gray-900 max-w-md truncate">
                    {attack.uri || '-'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {attacks.length === 0 && (
          <div className="text-center py-12">
            <Shield className="w-12 h-12 text-green-500 mx-auto mb-3" />
            <h3 className="text-lg font-semibold text-gray-900 mb-2">
              All Clear
            </h3>
            <p className="text-gray-600">
              No recent attacks detected
            </p>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="bg-white rounded-lg shadow border border-gray-200 p-6">
          <div className="flex items-center gap-3 mb-2">
            <div className="bg-red-100 p-2 rounded-lg">
              <Shield className="w-5 h-5 text-red-600" />
            </div>
            <h3 className="font-semibold text-gray-900">Total Blocked</h3>
          </div>
          <p className="text-3xl font-bold text-red-600">
            {attacks.filter((a) => a.action === 'Block').length}
          </p>
        </div>

        <div className="bg-white rounded-lg shadow border border-gray-200 p-6">
          <div className="flex items-center gap-3 mb-2">
            <div className="bg-yellow-100 p-2 rounded-lg">
              <AlertOctagon className="w-5 h-5 text-yellow-600" />
            </div>
            <h3 className="font-semibold text-gray-900">Logged</h3>
          </div>
          <p className="text-3xl font-bold text-yellow-600">
            {attacks.filter((a) => a.action === 'Log').length}
          </p>
        </div>

        <div className="bg-white rounded-lg shadow border border-gray-200 p-6">
          <div className="flex items-center gap-3 mb-2">
            <div className="bg-blue-100 p-2 rounded-lg">
              <Clock className="w-5 h-5 text-blue-600" />
            </div>
            <h3 className="font-semibold text-gray-900">Last Hour</h3>
          </div>
          <p className="text-3xl font-bold text-blue-600">{attacks.length}</p>
        </div>
      </div>
    </div>
  );
}
