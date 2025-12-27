/**
 * RustyDB Client - Binary spawning and process management
 * @module client
 */

import { spawn, ChildProcess } from 'cross-spawn';
import { EventEmitter } from 'eventemitter3';
import * as WebSocket from 'ws';
import {
  RustyDbConfig,
  DEFAULT_BINARY_PATHS,
  getServerConfig,
  getApiConfig,
  mergeConfigs,
} from './config';
import {
  HealthStatus,
  ServiceStatus,
  ApiError,
  ErrorCode,
  BaseClientConfig,
} from './types';
import { createLogger, Logger, sleep, withTimeout, createDeferred, Deferred } from './utils';

// ============================================================================
// Events
// ============================================================================

export interface ClientEvents {
  'server:starting': () => void;
  'server:started': () => void;
  'server:stopping': () => void;
  'server:stopped': () => void;
  'server:error': (error: Error) => void;
  'server:stdout': (data: string) => void;
  'server:stderr': (data: string) => void;
  'connection:open': () => void;
  'connection:close': () => void;
  'connection:error': (error: Error) => void;
  'health:change': (status: HealthStatus) => void;
}

// ============================================================================
// Server Process Manager
// ============================================================================

/**
 * Manages the RustyDB server process lifecycle
 */
export class ServerProcessManager extends EventEmitter<ClientEvents> {
  private process: ChildProcess | null = null;
  private config: RustyDbConfig;
  private logger: Logger;
  private startupDeferred: Deferred<void> | null = null;
  private shutdownDeferred: Deferred<void> | null = null;

  constructor(config: RustyDbConfig = {}, logger?: Logger) {
    super();
    this.config = config;
    this.logger = logger || createLogger('ServerProcess');
  }

  /**
   * Start the server process
   */
  async start(): Promise<void> {
    if (this.process) {
      throw new Error('Server process is already running');
    }

    this.emit('server:starting');
    this.logger.info('Starting RustyDB server process...');

    const serverConfig = getServerConfig(this.config.server);
    const binaryPath = this.config.binaries?.server || DEFAULT_BINARY_PATHS.server;

    const args = [
      '--host',
      serverConfig.host,
      '--port',
      serverConfig.port.toString(),
      '--data-dir',
      serverConfig.dataDir,
      '--log-level',
      serverConfig.logLevel,
      '--max-connections',
      serverConfig.maxConnections.toString(),
    ];

    this.logger.debug(`Spawning server: ${binaryPath} ${args.join(' ')}`);

    this.startupDeferred = createDeferred<void>();

    try {
      this.process = spawn(binaryPath, args, {
        cwd: this.config.cwd,
        env: {
          ...process.env,
          ...this.config.env,
        },
        stdio: ['ignore', 'pipe', 'pipe'],
      });

      if (!this.process.pid) {
        throw new Error('Failed to spawn server process');
      }

      this.setupProcessHandlers();

      // Wait for server to be ready (with timeout)
      const timeout = this.config.startupTimeout || 30000;
      await withTimeout(
        this.startupDeferred.promise,
        timeout,
        `Server startup timed out after ${timeout}ms`
      );

      this.logger.info(`Server started successfully (PID: ${this.process.pid})`);
      this.emit('server:started');
    } catch (error) {
      this.logger.error('Failed to start server:', error);
      this.cleanup();
      if (this.startupDeferred) {
        this.startupDeferred.reject(error);
      }
      throw error;
    }
  }

  /**
   * Stop the server process
   */
  async stop(): Promise<void> {
    if (!this.process) {
      return;
    }

    this.emit('server:stopping');
    this.logger.info('Stopping RustyDB server process...');

    this.shutdownDeferred = createDeferred<void>();

    try {
      // Send SIGTERM for graceful shutdown
      this.process.kill('SIGTERM');

      // Wait for server to exit (with timeout)
      const timeout = this.config.shutdownTimeout || 10000;
      await withTimeout(
        this.shutdownDeferred.promise,
        timeout,
        `Server shutdown timed out after ${timeout}ms`
      );

      this.logger.info('Server stopped successfully');
      this.emit('server:stopped');
    } catch (error) {
      this.logger.warn('Graceful shutdown failed, forcing kill:', error);
      this.process.kill('SIGKILL');
      this.cleanup();
      throw error;
    } finally {
      this.cleanup();
    }
  }

  /**
   * Check if the server process is running
   */
  isRunning(): boolean {
    return this.process !== null && !this.process.killed;
  }

  /**
   * Get the server process PID
   */
  getPid(): number | undefined {
    return this.process?.pid;
  }

  /**
   * Setup process event handlers
   */
  private setupProcessHandlers(): void {
    if (!this.process) {
      return;
    }

    this.process.stdout?.on('data', (data: Buffer) => {
      const output = data.toString().trim();
      if (this.config.logging?.stdout) {
        this.logger.debug(`[SERVER OUT] ${output}`);
      }
      this.emit('server:stdout', output);

      // Check for startup success message
      if (output.includes('Server listening on') || output.includes('RustyDB started')) {
        if (this.startupDeferred) {
          this.startupDeferred.resolve();
        }
      }
    });

    this.process.stderr?.on('data', (data: Buffer) => {
      const output = data.toString().trim();
      if (this.config.logging?.stderr) {
        this.logger.warn(`[SERVER ERR] ${output}`);
      }
      this.emit('server:stderr', output);
    });

    this.process.on('error', (error: Error) => {
      this.logger.error('Server process error:', error);
      this.emit('server:error', error);
      if (this.startupDeferred) {
        this.startupDeferred.reject(error);
      }
    });

    this.process.on('exit', (code: number | null, signal: string | null) => {
      this.logger.info(`Server process exited (code: ${code}, signal: ${signal})`);
      if (this.shutdownDeferred) {
        if (code === 0 || signal === 'SIGTERM') {
          this.shutdownDeferred.resolve();
        } else {
          this.shutdownDeferred.reject(
            new Error(`Server exited with code ${code} and signal ${signal}`)
          );
        }
      }
      this.cleanup();
    });
  }

  /**
   * Cleanup process resources
   */
  private cleanup(): void {
    if (this.process) {
      this.process.removeAllListeners();
      this.process = null;
    }
    this.startupDeferred = null;
    this.shutdownDeferred = null;
  }
}

// ============================================================================
// HTTP Client
// ============================================================================

/**
 * Base HTTP client for REST API communication
 */
export class HttpClient {
  protected baseUrl: string;
  protected apiVersion: string;
  protected timeout: number;
  protected headers: Record<string, string>;
  protected logger: Logger;

  constructor(config: BaseClientConfig, logger?: Logger) {
    this.baseUrl = config.baseUrl.replace(/\/$/, ''); // Remove trailing slash
    this.apiVersion = config.apiVersion || 'v1';
    this.timeout = config.timeout || 30000;
    this.headers = config.headers || {};
    this.logger = logger || createLogger('HttpClient');
  }

  /**
   * Build the full URL for an endpoint
   */
  protected buildUrl(path: string): string {
    const fullPath = path.startsWith('/') ? path : `/${path}`;
    return `${this.baseUrl}/api/${this.apiVersion}${fullPath}`;
  }

  /**
   * Make an HTTP request with error handling
   */
  protected async request<T>(
    method: string,
    path: string,
    body?: unknown,
    options?: {
      headers?: Record<string, string>;
      timeout?: number;
    }
  ): Promise<T> {
    const url = this.buildUrl(path);
    const requestOptions: RequestInit = {
      method,
      headers: {
        'Content-Type': 'application/json',
        ...this.headers,
        ...options?.headers,
      },
      signal: AbortSignal.timeout(options?.timeout || this.timeout),
    };

    if (body !== undefined) {
      requestOptions.body = JSON.stringify(body);
    }

    this.logger.debug(`${method} ${url}`);

    try {
      const response = await fetch(url, requestOptions);

      if (!response.ok) {
        const error: ApiError = await response.json().catch(() => ({
          code: ErrorCode.UNKNOWN_ERROR,
          message: `HTTP ${response.status}: ${response.statusText}`,
        }));
        this.logger.error(`Request failed: [${error.code}] ${error.message}`);
        throw new Error(`[${error.code}] ${error.message}`);
      }

      // Handle 204 No Content responses
      if (response.status === 204) {
        return undefined as T;
      }

      return await response.json();
    } catch (error) {
      if (error instanceof Error) {
        this.logger.error(`Request error: ${error.message}`);
        throw error;
      }
      throw new Error(`Request failed: ${String(error)}`);
    }
  }

  /**
   * GET request
   */
  async get<T>(path: string, options?: { headers?: Record<string, string>; timeout?: number }): Promise<T> {
    return this.request<T>('GET', path, undefined, options);
  }

  /**
   * POST request
   */
  async post<T>(path: string, body?: unknown, options?: { headers?: Record<string, string>; timeout?: number }): Promise<T> {
    return this.request<T>('POST', path, body, options);
  }

  /**
   * PUT request
   */
  async put<T>(path: string, body?: unknown, options?: { headers?: Record<string, string>; timeout?: number }): Promise<T> {
    return this.request<T>('PUT', path, body, options);
  }

  /**
   * DELETE request
   */
  async delete<T>(path: string, options?: { headers?: Record<string, string>; timeout?: number }): Promise<T> {
    return this.request<T>('DELETE', path, undefined, options);
  }

  /**
   * PATCH request
   */
  async patch<T>(path: string, body?: unknown, options?: { headers?: Record<string, string>; timeout?: number }): Promise<T> {
    return this.request<T>('PATCH', path, body, options);
  }

  /**
   * Health check
   */
  async healthCheck(): Promise<ServiceStatus> {
    return this.get<ServiceStatus>('/health');
  }
}

// ============================================================================
// WebSocket Client
// ============================================================================

/**
 * WebSocket client for real-time communication
 */
export class WebSocketClient extends EventEmitter<ClientEvents> {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnect: boolean;
  private reconnectInterval: number;
  private maxReconnectAttempts: number;
  private pingInterval: number;
  private headers: Record<string, string>;
  private logger: Logger;
  private reconnectCount: number = 0;
  private pingTimer: NodeJS.Timeout | null = null;

  constructor(config: RustyDbConfig, logger?: Logger) {
    super();
    const wsConfig = config.websocket || {};
    this.url = wsConfig.url || 'ws://localhost:8080/ws';
    this.reconnect = wsConfig.reconnect !== false;
    this.reconnectInterval = wsConfig.reconnectInterval || 5000;
    this.maxReconnectAttempts = wsConfig.maxReconnectAttempts || 10;
    this.pingInterval = wsConfig.pingInterval || 30000;
    this.headers = wsConfig.headers || {};
    this.logger = logger || createLogger('WebSocketClient');
  }

  /**
   * Connect to WebSocket server
   */
  async connect(): Promise<void> {
    if (this.ws) {
      throw new Error('WebSocket is already connected');
    }

    return new Promise((resolve, reject) => {
      this.logger.info(`Connecting to WebSocket: ${this.url}`);

      this.ws = new WebSocket(this.url, {
        headers: this.headers,
      });

      this.ws.on('open', () => {
        this.logger.info('WebSocket connected');
        this.reconnectCount = 0;
        this.startPing();
        this.emit('connection:open');
        resolve();
      });

      this.ws.on('close', () => {
        this.logger.info('WebSocket disconnected');
        this.stopPing();
        this.emit('connection:close');
        this.handleReconnect();
      });

      this.ws.on('error', (error: Error) => {
        this.logger.error('WebSocket error:', error);
        this.emit('connection:error', error);
        reject(error);
      });

      this.ws.on('message', (data: WebSocket.Data) => {
        this.handleMessage(data);
      });
    });
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect(): void {
    if (!this.ws) {
      return;
    }

    this.reconnect = false;
    this.stopPing();
    this.ws.close();
    this.ws = null;
    this.logger.info('WebSocket disconnected');
  }

  /**
   * Send a message
   */
  send(data: unknown): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket is not connected');
    }

    const message = typeof data === 'string' ? data : JSON.stringify(data);
    this.ws.send(message);
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  /**
   * Handle incoming messages
   */
  private handleMessage(data: WebSocket.Data): void {
    try {
      const message = typeof data === 'string' ? data : data.toString();
      const parsed = JSON.parse(message);
      this.logger.debug('Received message:', parsed);
      // Emit parsed message for subscribers
      this.emit('message', parsed);
    } catch (error) {
      this.logger.error('Failed to parse message:', error);
    }
  }

  /**
   * Handle reconnection logic
   */
  private async handleReconnect(): Promise<void> {
    if (!this.reconnect || this.reconnectCount >= this.maxReconnectAttempts) {
      this.logger.warn('Max reconnect attempts reached');
      return;
    }

    this.reconnectCount++;
    this.logger.info(`Reconnecting (attempt ${this.reconnectCount}/${this.maxReconnectAttempts})...`);

    await sleep(this.reconnectInterval);

    try {
      await this.connect();
    } catch (error) {
      this.logger.error('Reconnection failed:', error);
    }
  }

  /**
   * Start ping interval
   */
  private startPing(): void {
    this.stopPing();
    this.pingTimer = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.ws.ping();
      }
    }, this.pingInterval);
  }

  /**
   * Stop ping interval
   */
  private stopPing(): void {
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
  }
}

// ============================================================================
// Main RustyDB Client
// ============================================================================

/**
 * Main RustyDB client - coordinates server process, HTTP, and WebSocket
 */
export class RustyDbClient extends EventEmitter<ClientEvents> {
  private config: RustyDbConfig;
  private logger: Logger;
  private serverManager: ServerProcessManager | null = null;
  private httpClient: HttpClient;
  private wsClient: WebSocketClient | null = null;

  constructor(config: RustyDbConfig = {}) {
    super();
    this.config = mergeConfigs({}, config);
    this.logger = createLogger('RustyDbClient');

    // Initialize HTTP client
    const apiConfig = getApiConfig(this.config.api);
    this.httpClient = new HttpClient(apiConfig, this.logger);

    // Initialize WebSocket client if configured
    if (this.config.websocket) {
      this.wsClient = new WebSocketClient(this.config, this.logger);
      this.forwardEvents(this.wsClient);
    }
  }

  /**
   * Initialize the client (start server if auto-start is enabled)
   */
  async initialize(): Promise<void> {
    this.logger.info('Initializing RustyDB client...');

    if (this.config.autoStart) {
      this.serverManager = new ServerProcessManager(this.config, this.logger);
      this.forwardEvents(this.serverManager);
      await this.serverManager.start();

      // Wait for server to be healthy
      await this.waitForHealth();
    }

    // Connect WebSocket if configured
    if (this.wsClient && this.config.autoStart) {
      await this.wsClient.connect();
    }

    this.logger.info('RustyDB client initialized successfully');
  }

  /**
   * Shutdown the client (stop server if auto-stop is enabled)
   */
  async shutdown(): Promise<void> {
    this.logger.info('Shutting down RustyDB client...');

    // Disconnect WebSocket
    if (this.wsClient) {
      this.wsClient.disconnect();
    }

    // Stop server if auto-stop is enabled
    if (this.config.autoStop && this.serverManager) {
      await this.serverManager.stop();
    }

    this.logger.info('RustyDB client shut down successfully');
  }

  /**
   * Get HTTP client for making REST API calls
   */
  getHttpClient(): HttpClient {
    return this.httpClient;
  }

  /**
   * Get WebSocket client
   */
  getWebSocketClient(): WebSocketClient | null {
    return this.wsClient;
  }

  /**
   * Wait for server to be healthy
   */
  private async waitForHealth(maxAttempts: number = 30, interval: number = 1000): Promise<void> {
    for (let i = 0; i < maxAttempts; i++) {
      try {
        const status = await this.httpClient.healthCheck();
        if (status.status === HealthStatus.HEALTHY) {
          this.logger.info('Server is healthy');
          return;
        }
      } catch (error) {
        // Ignore errors and retry
      }
      await sleep(interval);
    }
    throw new Error('Server health check failed');
  }

  /**
   * Forward events from child emitters
   */
  private forwardEvents(emitter: EventEmitter<ClientEvents>): void {
    const events: (keyof ClientEvents)[] = [
      'server:starting',
      'server:started',
      'server:stopping',
      'server:stopped',
      'server:error',
      'server:stdout',
      'server:stderr',
      'connection:open',
      'connection:close',
      'connection:error',
      'health:change',
    ];

    events.forEach((event) => {
      emitter.on(event, (...args: unknown[]) => this.emit(event, ...args));
    });
  }
}

// ============================================================================
// Exports
// ============================================================================

export default RustyDbClient;
