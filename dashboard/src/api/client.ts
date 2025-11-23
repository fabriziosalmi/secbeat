import type { DashboardSummary, DashboardAttacksResponse, NodeInfo } from '../types/api';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3030';

export class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  private async fetch<T>(endpoint: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${endpoint}`);
    if (!response.ok) {
      throw new Error(`API error: ${response.status} ${response.statusText}`);
    }
    return response.json();
  }

  async getDashboardSummary(): Promise<DashboardSummary> {
    return this.fetch<DashboardSummary>('/api/v1/dashboard/summary');
  }

  async getDashboardAttacks(): Promise<DashboardAttacksResponse> {
    return this.fetch<DashboardAttacksResponse>('/api/v1/dashboard/attacks');
  }

  async getNodes(): Promise<NodeInfo[]> {
    return this.fetch<NodeInfo[]>('/api/v1/nodes');
  }
}

export const apiClient = new ApiClient();
