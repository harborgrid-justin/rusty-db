/**
 * Configuration management for RustyDB adapter
 * @module config
 */

import { LogLevel, ServerConfig, BaseClientConfig, WebSocketConfig } from '../types';

// ============================================================================
// Default Configuration Values
// ============================================================================

/**
 * Default server configuration
 */
export const DEFAULT_SERVER_CONFIG: Required<ServerConfig> = {
  host: 'localhost',
  port: 5432,
  dataDir: './data',
  logLevel: LogLevel.INFO,
  maxConnections: 100,
};

/**
 * Default REST API configuration
 */
export const DEFAULT_API_CONFIG = {
  baseUrl: 'http://localhost:8080',
  apiVersion: 'v1',
  timeout: 30000, // 30 seconds
  headers: {
    'Content-Type': 'application/json',
  },
};

/**
 * Default GraphQL configuration
 */
export const DEFAULT_GRAPHQL_CONFIG = {
  endpoint: 'http://localhost:8080/graphql',
  timeout: 30000,
  headers: {
    'Content-Type': 'application/json',
  },
};

/**
 * Default WebSocket configuration
 */
export const DEFAULT_WS_CONFIG: Required<WebSocketConfig> = {
  url: 'ws://localhost:8080/ws',
  reconnect: true,
  reconnectInterval: 5000, // 5 seconds
  maxReconnectAttempts: 10,
  pingInterval: 30000, // 30 seconds
  headers: {},
};

/**
 * Default binary paths
 * Updated for v0.5.1 enterprise deployment - Dec 27, 2025
 */
export const DEFAULT_BINARY_PATHS = {
  server: 'builds/linux/rusty-db-server',
  cli: 'builds/linux/rusty-db-cli',
};

// ============================================================================
// Configuration Interfaces
// ============================================================================

/**
 * Complete RustyDB client configuration
 */
export interface RustyDbConfig {
  /**
   * Server configuration (for spawning server process)
   */
  server?: Partial<ServerConfig>;

  /**
   * REST API client configuration
   */
  api?: Partial<BaseClientConfig>;

  /**
   * GraphQL client configuration
   */
  graphql?: {
    endpoint?: string;
    timeout?: number;
    headers?: Record<string, string>;
  };

  /**
   * WebSocket configuration
   */
  websocket?: Partial<WebSocketConfig>;

  /**
   * Binary paths (for spawning processes)
   */
  binaries?: {
    server?: string;
    cli?: string;
  };

  /**
   * Auto-start server on client initialization
   */
  autoStart?: boolean;

  /**
   * Auto-stop server on client shutdown
   */
  autoStop?: boolean;

  /**
   * Server startup timeout (milliseconds)
   */
  startupTimeout?: number;

  /**
   * Server shutdown timeout (milliseconds)
   */
  shutdownTimeout?: number;

  /**
   * Environment variables to pass to server process
   */
  env?: Record<string, string>;

  /**
   * Working directory for server process
   */
  cwd?: string;

  /**
   * Server process logging
   */
  logging?: {
    stdout?: boolean;
    stderr?: boolean;
    file?: string;
  };
}

// ============================================================================
// Configuration Builder
// ============================================================================

/**
 * Configuration builder class for fluent API
 */
export class ConfigBuilder {
  private config: RustyDbConfig = {};

  /**
   * Set server configuration
   */
  server(config: Partial<ServerConfig>): this {
    this.config.server = { ...this.config.server, ...config };
    return this;
  }

  /**
   * Set REST API configuration
   */
  api(config: Partial<BaseClientConfig>): this {
    this.config.api = { ...this.config.api, ...config };
    return this;
  }

  /**
   * Set GraphQL configuration
   */
  graphql(endpoint: string, options?: { timeout?: number; headers?: Record<string, string> }): this {
    this.config.graphql = {
      endpoint,
      timeout: options?.timeout,
      headers: options?.headers,
    };
    return this;
  }

  /**
   * Set WebSocket configuration
   */
  websocket(config: Partial<WebSocketConfig>): this {
    this.config.websocket = { ...this.config.websocket, ...config };
    return this;
  }

  /**
   * Set binary paths
   */
  binaries(server?: string, cli?: string): this {
    this.config.binaries = {
      server: server || this.config.binaries?.server,
      cli: cli || this.config.binaries?.cli,
    };
    return this;
  }

  /**
   * Enable auto-start of server
   */
  autoStart(enabled: boolean = true): this {
    this.config.autoStart = enabled;
    return this;
  }

  /**
   * Enable auto-stop of server
   */
  autoStop(enabled: boolean = true): this {
    this.config.autoStop = enabled;
    return this;
  }

  /**
   * Set startup timeout
   */
  startupTimeout(ms: number): this {
    this.config.startupTimeout = ms;
    return this;
  }

  /**
   * Set shutdown timeout
   */
  shutdownTimeout(ms: number): this {
    this.config.shutdownTimeout = ms;
    return this;
  }

  /**
   * Set environment variables
   */
  env(env: Record<string, string>): this {
    this.config.env = { ...this.config.env, ...env };
    return this;
  }

  /**
   * Set working directory
   */
  cwd(cwd: string): this {
    this.config.cwd = cwd;
    return this;
  }

  /**
   * Enable logging
   */
  logging(options: { stdout?: boolean; stderr?: boolean; file?: string }): this {
    this.config.logging = { ...this.config.logging, ...options };
    return this;
  }

  /**
   * Build the final configuration
   */
  build(): RustyDbConfig {
    return this.config;
  }
}

// ============================================================================
// Configuration Helper Functions
// ============================================================================

/**
 * Create a new configuration builder
 */
export function createConfig(): ConfigBuilder {
  return new ConfigBuilder();
}

/**
 * Merge configurations with precedence (right-most wins)
 */
export function mergeConfigs(...configs: RustyDbConfig[]): RustyDbConfig {
  return configs.reduce((acc, config) => {
    return {
      ...acc,
      ...config,
      server: { ...acc.server, ...config.server },
      api: { ...acc.api, ...config.api },
      graphql: { ...acc.graphql, ...config.graphql },
      websocket: { ...acc.websocket, ...config.websocket },
      binaries: { ...acc.binaries, ...config.binaries },
      env: { ...acc.env, ...config.env },
      logging: { ...acc.logging, ...config.logging },
    };
  }, {});
}

/**
 * Get full server configuration with defaults
 */
export function getServerConfig(config?: Partial<ServerConfig>): Required<ServerConfig> {
  return {
    ...DEFAULT_SERVER_CONFIG,
    ...config,
  };
}

/**
 * Get full API configuration with defaults
 */
export function getApiConfig(config?: Partial<BaseClientConfig>): Required<BaseClientConfig> {
  return {
    ...DEFAULT_API_CONFIG,
    ...config,
  } as Required<BaseClientConfig>;
}

/**
 * Get full WebSocket configuration with defaults
 */
export function getWebSocketConfig(config?: Partial<WebSocketConfig>): Required<WebSocketConfig> {
  return {
    ...DEFAULT_WS_CONFIG,
    ...config,
  };
}

/**
 * Validate configuration
 */
export function validateConfig(config: RustyDbConfig): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  // Validate server config
  if (config.server) {
    if (config.server.port && (config.server.port < 1 || config.server.port > 65535)) {
      errors.push('Server port must be between 1 and 65535');
    }
    if (config.server.maxConnections && config.server.maxConnections < 1) {
      errors.push('Max connections must be at least 1');
    }
  }

  // Validate API config
  if (config.api) {
    if (config.api.baseUrl && !isValidUrl(config.api.baseUrl)) {
      errors.push('Invalid API base URL');
    }
    if (config.api.timeout && config.api.timeout < 0) {
      errors.push('API timeout must be non-negative');
    }
  }

  // Validate GraphQL config
  if (config.graphql) {
    if (config.graphql.endpoint && !isValidUrl(config.graphql.endpoint)) {
      errors.push('Invalid GraphQL endpoint URL');
    }
    if (config.graphql.timeout && config.graphql.timeout < 0) {
      errors.push('GraphQL timeout must be non-negative');
    }
  }

  // Validate WebSocket config
  if (config.websocket) {
    if (config.websocket.url && !isValidWsUrl(config.websocket.url)) {
      errors.push('Invalid WebSocket URL');
    }
    if (
      config.websocket.reconnectInterval &&
      config.websocket.reconnectInterval < 0
    ) {
      errors.push('Reconnect interval must be non-negative');
    }
    if (
      config.websocket.maxReconnectAttempts &&
      config.websocket.maxReconnectAttempts < 0
    ) {
      errors.push('Max reconnect attempts must be non-negative');
    }
  }

  // Validate timeouts
  if (config.startupTimeout && config.startupTimeout < 0) {
    errors.push('Startup timeout must be non-negative');
  }
  if (config.shutdownTimeout && config.shutdownTimeout < 0) {
    errors.push('Shutdown timeout must be non-negative');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Check if a string is a valid HTTP/HTTPS URL
 */
function isValidUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'http:' || parsed.protocol === 'https:';
  } catch {
    return false;
  }
}

/**
 * Check if a string is a valid WebSocket URL
 */
function isValidWsUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'ws:' || parsed.protocol === 'wss:';
  } catch {
    return false;
  }
}

/**
 * Load configuration from environment variables
 */
export function loadConfigFromEnv(): RustyDbConfig {
  return {
    server: {
      host: process.env.RUSTYDB_HOST,
      port: process.env.RUSTYDB_PORT ? parseInt(process.env.RUSTYDB_PORT, 10) : undefined,
      dataDir: process.env.RUSTYDB_DATA_DIR,
      logLevel: process.env.RUSTYDB_LOG_LEVEL as LogLevel | undefined,
      maxConnections: process.env.RUSTYDB_MAX_CONNECTIONS
        ? parseInt(process.env.RUSTYDB_MAX_CONNECTIONS, 10)
        : undefined,
    },
    api: {
      baseUrl: process.env.RUSTYDB_API_URL,
      timeout: process.env.RUSTYDB_API_TIMEOUT
        ? parseInt(process.env.RUSTYDB_API_TIMEOUT, 10)
        : undefined,
    },
    graphql: {
      endpoint: process.env.RUSTYDB_GRAPHQL_URL,
      timeout: process.env.RUSTYDB_GRAPHQL_TIMEOUT
        ? parseInt(process.env.RUSTYDB_GRAPHQL_TIMEOUT, 10)
        : undefined,
    },
    websocket: {
      url: process.env.RUSTYDB_WS_URL,
    },
    binaries: {
      server: process.env.RUSTYDB_SERVER_BINARY,
      cli: process.env.RUSTYDB_CLI_BINARY,
    },
    autoStart: process.env.RUSTYDB_AUTO_START === 'true',
    autoStop: process.env.RUSTYDB_AUTO_STOP === 'true',
  };
}

// ============================================================================
// Exports
// ============================================================================

export default {
  createConfig,
  mergeConfigs,
  getServerConfig,
  getApiConfig,
  getWebSocketConfig,
  validateConfig,
  loadConfigFromEnv,
  DEFAULT_SERVER_CONFIG,
  DEFAULT_API_CONFIG,
  DEFAULT_GRAPHQL_CONFIG,
  DEFAULT_WS_CONFIG,
  DEFAULT_BINARY_PATHS,
};
