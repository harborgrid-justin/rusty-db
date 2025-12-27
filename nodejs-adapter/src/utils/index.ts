/**
 * Utility functions for RustyDB adapter
 * @module utils
 */

import { ApiError, ErrorCode, Timestamp } from '../types';

// ============================================================================
// Error Handling Utilities
// ============================================================================

/**
 * Create a standardized API error
 */
export function createApiError(
  code: ErrorCode | string,
  message: string,
  details?: Record<string, unknown>
): ApiError {
  return {
    code,
    message,
    details,
  };
}

/**
 * Check if an error is an API error
 */
export function isApiError(error: unknown): error is ApiError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    'message' in error
  );
}

/**
 * Extract error message from various error types
 */
export function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (isApiError(error)) {
    return `[${error.code}] ${error.message}`;
  }
  return String(error);
}

/**
 * Wrap a function with error handling
 */
export function withErrorHandling<T extends (...args: unknown[]) => unknown>(
  fn: T,
  errorHandler?: (error: unknown) => void
): T {
  return ((...args: Parameters<T>) => {
    try {
      const result = fn(...args);
      if (result instanceof Promise) {
        return result.catch((error) => {
          if (errorHandler) {
            errorHandler(error);
          }
          throw error;
        });
      }
      return result;
    } catch (error) {
      if (errorHandler) {
        errorHandler(error);
      }
      throw error;
    }
  }) as T;
}

// ============================================================================
// Async Utilities
// ============================================================================

/**
 * Sleep for a specified number of milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Retry a function with exponential backoff
 */
export async function retry<T>(
  fn: () => Promise<T>,
  options: {
    maxAttempts?: number;
    initialDelay?: number;
    maxDelay?: number;
    factor?: number;
    shouldRetry?: (error: unknown) => boolean;
  } = {}
): Promise<T> {
  const {
    maxAttempts = 3,
    initialDelay = 1000,
    maxDelay = 30000,
    factor = 2,
    shouldRetry = () => true,
  } = options;

  let lastError: unknown;
  let delay = initialDelay;

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;

      if (attempt === maxAttempts || !shouldRetry(error)) {
        throw error;
      }

      await sleep(delay);
      delay = Math.min(delay * factor, maxDelay);
    }
  }

  throw lastError;
}

/**
 * Execute a function with a timeout
 */
export async function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  timeoutMessage: string = 'Operation timed out'
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs)
    ),
  ]);
}

/**
 * Create a deferred promise
 */
export interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason?: unknown) => void;
}

export function createDeferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}

// ============================================================================
// Validation Utilities
// ============================================================================

/**
 * Check if a value is a valid UUID
 */
export function isValidUuid(value: string): boolean {
  const uuidRegex =
    /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  return uuidRegex.test(value);
}

/**
 * Check if a value is a non-empty string
 */
export function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0;
}

/**
 * Check if a value is a positive number
 */
export function isPositiveNumber(value: unknown): value is number {
  return typeof value === 'number' && value > 0 && !isNaN(value);
}

/**
 * Check if a value is a non-negative number
 */
export function isNonNegativeNumber(value: unknown): value is number {
  return typeof value === 'number' && value >= 0 && !isNaN(value);
}

/**
 * Validate required fields in an object
 */
export function validateRequired<T extends Record<string, unknown>>(
  obj: T,
  fields: (keyof T)[]
): { valid: boolean; missing: string[] } {
  const missing = fields.filter((field) => {
    const value = obj[field];
    return value === undefined || value === null || value === '';
  });

  return {
    valid: missing.length === 0,
    missing: missing.map(String),
  };
}

// ============================================================================
// Data Transformation Utilities
// ============================================================================

/**
 * Convert snake_case to camelCase
 */
export function snakeToCamel(str: string): string {
  return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

/**
 * Convert camelCase to snake_case
 */
export function camelToSnake(str: string): string {
  return str.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}

/**
 * Deep clone an object
 */
export function deepClone<T>(obj: T): T {
  if (obj === null || typeof obj !== 'object') {
    return obj;
  }

  if (obj instanceof Date) {
    return new Date(obj.getTime()) as unknown as T;
  }

  if (obj instanceof Array) {
    return obj.map((item) => deepClone(item)) as unknown as T;
  }

  if (obj instanceof Object) {
    const clonedObj = {} as T;
    for (const key in obj) {
      if (Object.prototype.hasOwnProperty.call(obj, key)) {
        clonedObj[key] = deepClone(obj[key]);
      }
    }
    return clonedObj;
  }

  return obj;
}

/**
 * Omit keys from an object
 */
export function omit<T extends Record<string, unknown>, K extends keyof T>(
  obj: T,
  keys: K[]
): Omit<T, K> {
  const result = { ...obj };
  for (const key of keys) {
    delete result[key];
  }
  return result;
}

/**
 * Pick keys from an object
 */
export function pick<T extends Record<string, unknown>, K extends keyof T>(
  obj: T,
  keys: K[]
): Pick<T, K> {
  const result = {} as Pick<T, K>;
  for (const key of keys) {
    if (key in obj) {
      result[key] = obj[key];
    }
  }
  return result;
}

// ============================================================================
// Time Utilities
// ============================================================================

/**
 * Get current timestamp in milliseconds
 */
export function now(): Timestamp {
  return Date.now();
}

/**
 * Format a timestamp as ISO string
 */
export function formatTimestamp(timestamp: Timestamp): string {
  return new Date(timestamp).toISOString();
}

/**
 * Parse an ISO string to timestamp
 */
export function parseTimestamp(isoString: string): Timestamp {
  return new Date(isoString).getTime();
}

/**
 * Calculate duration between two timestamps
 */
export function duration(start: Timestamp, end: Timestamp = now()): number {
  return end - start;
}

/**
 * Format duration in human-readable format
 */
export function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms}ms`;
  }
  if (ms < 60000) {
    return `${(ms / 1000).toFixed(2)}s`;
  }
  if (ms < 3600000) {
    return `${(ms / 60000).toFixed(2)}m`;
  }
  return `${(ms / 3600000).toFixed(2)}h`;
}

// ============================================================================
// URL Utilities
// ============================================================================

/**
 * Build a URL with query parameters
 */
export function buildUrl(
  base: string,
  path: string,
  params?: Record<string, string | number | boolean | undefined>
): string {
  const url = new URL(path, base);

  if (params) {
    for (const [key, value] of Object.entries(params)) {
      if (value !== undefined) {
        url.searchParams.append(key, String(value));
      }
    }
  }

  return url.toString();
}

/**
 * Parse query parameters from a URL
 */
export function parseQueryParams(url: string): Record<string, string> {
  const parsed = new URL(url);
  const params: Record<string, string> = {};

  for (const [key, value] of parsed.searchParams.entries()) {
    params[key] = value;
  }

  return params;
}

// ============================================================================
// Collection Utilities
// ============================================================================

/**
 * Group array items by a key
 */
export function groupBy<T, K extends keyof T>(
  items: T[],
  key: K
): Map<T[K], T[]> {
  const map = new Map<T[K], T[]>();

  for (const item of items) {
    const value = item[key];
    const group = map.get(value) || [];
    group.push(item);
    map.set(value, group);
  }

  return map;
}

/**
 * Create a map from an array using a key function
 */
export function keyBy<T, K extends keyof T>(items: T[], key: K): Map<T[K], T> {
  const map = new Map<T[K], T>();

  for (const item of items) {
    map.set(item[key], item);
  }

  return map;
}

/**
 * Chunk an array into smaller arrays
 */
export function chunk<T>(array: T[], size: number): T[][] {
  const chunks: T[][] = [];

  for (let i = 0; i < array.length; i += size) {
    chunks.push(array.slice(i, i + size));
  }

  return chunks;
}

/**
 * Remove duplicates from an array
 */
export function unique<T>(array: T[]): T[] {
  return Array.from(new Set(array));
}

/**
 * Flatten a nested array
 */
export function flatten<T>(array: (T | T[])[]): T[] {
  return array.reduce<T[]>((acc, item) => {
    if (Array.isArray(item)) {
      acc.push(...flatten(item));
    } else {
      acc.push(item);
    }
    return acc;
  }, []);
}

// ============================================================================
// String Utilities
// ============================================================================

/**
 * Truncate a string to a maximum length
 */
export function truncate(
  str: string,
  maxLength: number,
  suffix: string = '...'
): string {
  if (str.length <= maxLength) {
    return str;
  }
  return str.slice(0, maxLength - suffix.length) + suffix;
}

/**
 * Capitalize the first letter of a string
 */
export function capitalize(str: string): string {
  if (str.length === 0) {
    return str;
  }
  return str.charAt(0).toUpperCase() + str.slice(1);
}

/**
 * Generate a random string
 */
export function randomString(length: number): string {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}

// ============================================================================
// Logging Utilities
// ============================================================================

/**
 * Simple logger interface
 */
export interface Logger {
  trace(message: string, ...args: unknown[]): void;
  debug(message: string, ...args: unknown[]): void;
  info(message: string, ...args: unknown[]): void;
  warn(message: string, ...args: unknown[]): void;
  error(message: string, ...args: unknown[]): void;
}

/**
 * Create a simple console logger
 */
export function createLogger(prefix: string = 'RustyDB'): Logger {
  const log = (level: string, message: string, ...args: unknown[]): void => {
    const timestamp = new Date().toISOString();
    // eslint-disable-next-line no-console
    console.log(`[${timestamp}] [${prefix}] [${level}] ${message}`, ...args);
  };

  return {
    trace: (message, ...args) => log('TRACE', message, ...args),
    debug: (message, ...args) => log('DEBUG', message, ...args),
    info: (message, ...args) => log('INFO', message, ...args),
    warn: (message, ...args) => log('WARN', message, ...args),
    error: (message, ...args) => log('ERROR', message, ...args),
  };
}

// ============================================================================
// Exports
// ============================================================================

export default {
  createApiError,
  isApiError,
  getErrorMessage,
  withErrorHandling,
  sleep,
  retry,
  withTimeout,
  createDeferred,
  isValidUuid,
  isNonEmptyString,
  isPositiveNumber,
  isNonNegativeNumber,
  validateRequired,
  snakeToCamel,
  camelToSnake,
  deepClone,
  omit,
  pick,
  now,
  formatTimestamp,
  parseTimestamp,
  duration,
  formatDuration,
  buildUrl,
  parseQueryParams,
  groupBy,
  keyBy,
  chunk,
  unique,
  flatten,
  truncate,
  capitalize,
  randomString,
  createLogger,
};
