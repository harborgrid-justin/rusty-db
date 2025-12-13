import axios, { AxiosError, AxiosInstance, AxiosRequestConfig, AxiosResponse } from 'axios';
import type { ApiResponse, ApiError } from '@/types';

// ============================================================================
// API Client Configuration
// ============================================================================

const API_URL = import.meta.env.VITE_API_URL || '';
const API_VERSION = import.meta.env.VITE_API_VERSION || 'v1';
const REQUEST_TIMEOUT = 30000; // 30 seconds

// Storage key for auth token
const TOKEN_STORAGE_KEY = 'rustydb_auth';

// ============================================================================
// Create Axios Instance
// ============================================================================

const createApiClient = (): AxiosInstance => {
  const client = axios.create({
    baseURL: `${API_URL}/api/${API_VERSION}`,
    timeout: REQUEST_TIMEOUT,
    headers: {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    },
  });

  // Request interceptor
  client.interceptors.request.use(
    (config) => {
      // Add auth token if available
      const storedAuth = localStorage.getItem(TOKEN_STORAGE_KEY);
      if (storedAuth) {
        try {
          const { session } = JSON.parse(storedAuth);
          if (session?.token) {
            config.headers.Authorization = `Bearer ${session.token}`;
          }
        } catch {
          // Invalid stored auth, ignore
        }
      }

      // Add request ID for tracing
      config.headers['X-Request-ID'] = crypto.randomUUID();

      // Add timestamp
      config.headers['X-Request-Time'] = new Date().toISOString();

      return config;
    },
    (error) => Promise.reject(error)
  );

  // Response interceptor
  client.interceptors.response.use(
    (response) => response,
    async (error: AxiosError<ApiError>) => {
      const originalRequest = error.config as AxiosRequestConfig & { _retry?: boolean };

      // Handle 401 Unauthorized
      if (error.response?.status === 401 && !originalRequest._retry) {
        originalRequest._retry = true;

        // Try to refresh token
        try {
          const storedAuth = localStorage.getItem(TOKEN_STORAGE_KEY);
          if (storedAuth) {
            const { session } = JSON.parse(storedAuth);
            if (session?.refreshToken) {
              const refreshResponse = await axios.post(
                `${API_URL}/api/${API_VERSION}/auth/refresh`,
                { refreshToken: session.refreshToken }
              );

              const newSession = refreshResponse.data.data;
              const currentAuth = JSON.parse(storedAuth);
              currentAuth.session = newSession;
              localStorage.setItem(TOKEN_STORAGE_KEY, JSON.stringify(currentAuth));

              // Retry original request with new token
              if (originalRequest.headers) {
                originalRequest.headers.Authorization = `Bearer ${newSession.token}`;
              }
              return client(originalRequest);
            }
          }
        } catch {
          // Refresh failed, clear auth and redirect to login
          localStorage.removeItem(TOKEN_STORAGE_KEY);
          window.location.href = '/login';
        }
      }

      // Handle 429 Too Many Requests
      if (error.response?.status === 429) {
        const retryAfter = error.response.headers['retry-after'];
        console.warn(`Rate limited. Retry after: ${retryAfter}s`);
      }

      // Handle network errors
      if (!error.response) {
        console.error('Network error:', error.message);
      }

      return Promise.reject(error);
    }
  );

  return client;
};

// ============================================================================
// API Client Singleton
// ============================================================================

export const apiClient = createApiClient();

// ============================================================================
// Generic Request Functions
// ============================================================================

export async function get<T>(
  url: string,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  const response: AxiosResponse<ApiResponse<T>> = await apiClient.get(url, config);
  return response.data;
}

export async function post<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  const response: AxiosResponse<ApiResponse<T>> = await apiClient.post(url, data, config);
  return response.data;
}

export async function put<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  const response: AxiosResponse<ApiResponse<T>> = await apiClient.put(url, data, config);
  return response.data;
}

export async function patch<T>(
  url: string,
  data?: unknown,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  const response: AxiosResponse<ApiResponse<T>> = await apiClient.patch(url, data, config);
  return response.data;
}

export async function del<T>(
  url: string,
  config?: AxiosRequestConfig
): Promise<ApiResponse<T>> {
  const response: AxiosResponse<ApiResponse<T>> = await apiClient.delete(url, config);
  return response.data;
}

// ============================================================================
// Error Handling Utilities
// ============================================================================

export function isApiError(error: unknown): error is AxiosError<ApiError> {
  return axios.isAxiosError(error);
}

export function getErrorMessage(error: unknown): string {
  if (isApiError(error)) {
    if (error.response?.data?.error?.message) {
      return error.response.data.error.message;
    }
    if (error.response?.data?.message) {
      return error.response.data.message as string;
    }
    if (error.message) {
      return error.message;
    }
  }

  if (error instanceof Error) {
    return error.message;
  }

  return 'An unexpected error occurred';
}

export function getErrorCode(error: unknown): string | undefined {
  if (isApiError(error)) {
    return error.response?.data?.error?.code;
  }
  return undefined;
}

// ============================================================================
// Request Utilities
// ============================================================================

export function buildQueryParams(params: Record<string, unknown>): string {
  const searchParams = new URLSearchParams();

  Object.entries(params).forEach(([key, value]) => {
    if (value !== undefined && value !== null && value !== '') {
      if (Array.isArray(value)) {
        value.forEach((v) => searchParams.append(key, String(v)));
      } else {
        searchParams.append(key, String(value));
      }
    }
  });

  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

export function createCancelToken(): {
  token: AbortController['signal'];
  cancel: () => void;
} {
  const controller = new AbortController();
  return {
    token: controller.signal,
    cancel: () => controller.abort(),
  };
}

// ============================================================================
// Export API URL for WebSocket connections
// ============================================================================

export const API_BASE_URL = API_URL;
export const WS_URL = `${API_URL.replace('http', 'ws')}/ws`;

// Export apiClient as default for backward compatibility
export default apiClient;
