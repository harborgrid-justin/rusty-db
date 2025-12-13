/**
 * API Gateway Service
 * Route management, rate limiting, and service discovery
 */

import api from './api';
import type { ApiResponse } from '../types/api';

export interface Route {
  id: string;
  path: string;
  service: string;
  methods: string[];
  strip_prefix: boolean;
  timeout_ms: number;
  retry_policy: {
    max_attempts: number;
    backoff_ms: number;
  };
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateRouteRequest {
  path: string;
  service: string;
  methods?: string[];
  strip_prefix?: boolean;
  timeout_ms?: number;
}

export interface RateLimit {
  id: string;
  name: string;
  routes: string[];
  config: {
    requests_per_minute: number;
    burst_size: number;
    key_type: 'ip' | 'user' | 'api_key';
  };
  enabled: boolean;
  created_at: string;
}

export interface ServiceEndpoint {
  url: string;
  weight: number;
  health_check?: {
    enabled: boolean;
    path: string;
    interval_seconds: number;
  };
}

export interface Service {
  id: string;
  name: string;
  description?: string;
  endpoints: ServiceEndpoint[];
  load_balancer_policy: 'round_robin' | 'least_connections' | 'weighted';
  circuit_breaker?: {
    enabled: boolean;
    failure_threshold: number;
    timeout_seconds: number;
  };
  enabled: boolean;
}

export interface GatewayMetrics {
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  avg_response_time_ms: number;
  p99_response_time_ms: number;
  rate_limited_requests: number;
  circuit_breaker_opens: number;
  routes: Record<string, {
    requests: number;
    avg_latency_ms: number;
    error_rate: number;
  }>;
}

export interface IpFilter {
  id: string;
  name: string;
  ip_ranges: string[];
  action: 'allow' | 'deny';
  priority: number;
  enabled: boolean;
}

class GatewayService {
  /**
   * List all routes
   */
  async listRoutes(params?: { page?: number; page_size?: number }): Promise<ApiResponse<{ routes: Route[]; total: number }>> {
    return api.get('/api/gateway/routes', { params });
  }

  /**
   * Get route by ID
   */
  async getRoute(id: string): Promise<ApiResponse<Route>> {
    return api.get(`/api/gateway/routes/${id}`);
  }

  /**
   * Create new route
   */
  async createRoute(data: CreateRouteRequest): Promise<ApiResponse<Route>> {
    return api.post('/api/gateway/routes', data);
  }

  /**
   * Update route
   */
  async updateRoute(id: string, data: Partial<CreateRouteRequest>): Promise<ApiResponse<Route>> {
    return api.put(`/api/gateway/routes/${id}`, data);
  }

  /**
   * Delete route
   */
  async deleteRoute(id: string): Promise<ApiResponse<{ message: string }>> {
    return api.delete(`/api/gateway/routes/${id}`);
  }

  /**
   * List rate limits
   */
  async listRateLimits(): Promise<ApiResponse<RateLimit[]>> {
    return api.get('/api/gateway/rate-limits');
  }

  /**
   * Create rate limit
   */
  async createRateLimit(data: Partial<RateLimit>): Promise<ApiResponse<RateLimit>> {
    return api.post('/api/gateway/rate-limits', data);
  }

  /**
   * Update rate limit
   */
  async updateRateLimit(id: string, data: Partial<RateLimit>): Promise<ApiResponse<RateLimit>> {
    return api.put(`/api/gateway/rate-limits/${id}`, data);
  }

  /**
   * Delete rate limit
   */
  async deleteRateLimit(id: string): Promise<ApiResponse<{ message: string }>> {
    return api.delete(`/api/gateway/rate-limits/${id}`);
  }

  /**
   * List services
   */
  async listServices(): Promise<ApiResponse<Service[]>> {
    return api.get('/api/gateway/services');
  }

  /**
   * Register service
   */
  async registerService(data: Partial<Service>): Promise<ApiResponse<Service>> {
    return api.post('/api/gateway/services', data);
  }

  /**
   * Update service
   */
  async updateService(id: string, data: Partial<Service>): Promise<ApiResponse<Service>> {
    return api.put(`/api/gateway/services/${id}`, data);
  }

  /**
   * Deregister service
   */
  async deregisterService(id: string): Promise<ApiResponse<{ message: string }>> {
    return api.delete(`/api/gateway/services/${id}`);
  }

  /**
   * Get service health
   */
  async getServiceHealth(id: string): Promise<ApiResponse<any>> {
    return api.get(`/api/gateway/services/${id}/health`);
  }

  /**
   * Get gateway metrics
   */
  async getMetrics(params?: { from?: string; to?: string }): Promise<ApiResponse<GatewayMetrics>> {
    return api.get('/api/gateway/metrics', { params });
  }

  /**
   * Get audit log
   */
  async getAuditLog(params?: { page?: number; page_size?: number; from?: string; to?: string }): Promise<ApiResponse<any>> {
    return api.get('/api/gateway/audit-log', { params });
  }

  /**
   * List IP filters
   */
  async listIpFilters(): Promise<ApiResponse<IpFilter[]>> {
    return api.get('/api/gateway/ip-filters');
  }

  /**
   * Add IP filter
   */
  async addIpFilter(data: Partial<IpFilter>): Promise<ApiResponse<IpFilter>> {
    return api.post('/api/gateway/ip-filters', data);
  }

  /**
   * Remove IP filter
   */
  async removeIpFilter(id: string): Promise<ApiResponse<{ message: string }>> {
    return api.delete(`/api/gateway/ip-filters/${id}`);
  }
}

export default new GatewayService();
