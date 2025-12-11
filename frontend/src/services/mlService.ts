// ============================================================================
// Machine Learning Service
// ============================================================================

import { get, post, del, patch, buildQueryParams } from './api';
import type {
  UUID,
  Timestamp,
  PaginatedResponse,
  PaginationParams,
} from '../types';

// ============================================================================
// Request/Response Types
// ============================================================================

export interface MLModel {
  id: UUID;
  name: string;
  type: MLModelType;
  algorithm: string;
  status: MLModelStatus;
  version: number;
  accuracy?: number;
  precision?: number;
  recall?: number;
  f1Score?: number;
  trainingSamples: number;
  features: string[];
  targetColumn: string;
  hyperparameters: Record<string, unknown>;
  createdAt: Timestamp;
  updatedAt: Timestamp;
  trainedAt?: Timestamp;
  lastPredictionAt?: Timestamp;
}

export type MLModelType =
  | 'regression'
  | 'classification'
  | 'clustering'
  | 'neural_network'
  | 'decision_tree'
  | 'random_forest'
  | 'gradient_boosting'
  | 'svm'
  | 'naive_bayes'
  | 'knn';

export type MLModelStatus =
  | 'draft'
  | 'training'
  | 'trained'
  | 'failed'
  | 'deployed'
  | 'archived';

export interface CreateModelRequest {
  name: string;
  type: MLModelType;
  algorithm: string;
  trainingQuery: string;
  targetColumn: string;
  features?: string[];
  hyperparameters?: Record<string, unknown>;
  validationSplit?: number;
  testSplit?: number;
}

export interface TrainModelRequest {
  trainingQuery?: string;
  hyperparameters?: Record<string, unknown>;
  validationSplit?: number;
  testSplit?: number;
}

export interface PredictionRequest {
  input: Record<string, unknown> | Record<string, unknown>[];
  threshold?: number;
  returnProbabilities?: boolean;
}

export interface PredictionResponse {
  predictions: PredictionResult[];
  modelId: UUID;
  modelVersion: number;
  timestamp: Timestamp;
  duration: number;
}

export interface PredictionResult {
  prediction: unknown;
  probabilities?: Record<string, number>;
  confidence?: number;
  input: Record<string, unknown>;
}

export interface ModelMetrics {
  modelId: UUID;
  version: number;
  accuracy: number;
  precision: number;
  recall: number;
  f1Score: number;
  confusionMatrix?: number[][];
  roc?: { fpr: number[]; tpr: number[]; thresholds: number[] };
  featureImportance?: Record<string, number>;
  trainingMetrics: TrainingMetrics;
  validationMetrics: TrainingMetrics;
  testMetrics?: TrainingMetrics;
  timestamp: Timestamp;
}

export interface TrainingMetrics {
  loss: number;
  accuracy?: number;
  mae?: number;
  mse?: number;
  rmse?: number;
  r2?: number;
  epochs?: number[];
  lossHistory?: number[];
  accuracyHistory?: number[];
}

export interface ModelFilters extends PaginationParams {
  type?: MLModelType;
  status?: MLModelStatus;
  search?: string;
  createdAfter?: Timestamp;
  createdBefore?: Timestamp;
}

export interface ModelRetrainingRequest {
  incrementalData?: string;
  fullRetrain?: boolean;
  hyperparameters?: Record<string, unknown>;
}

export interface FeatureAnalysis {
  features: FeatureStats[];
  correlations: Record<string, Record<string, number>>;
  timestamp: Timestamp;
}

export interface FeatureStats {
  name: string;
  type: 'numeric' | 'categorical' | 'text' | 'datetime';
  mean?: number;
  median?: number;
  stdDev?: number;
  min?: number;
  max?: number;
  nullCount: number;
  uniqueCount: number;
  distribution?: { value: unknown; count: number }[];
}

// ============================================================================
// ML Model Management APIs
// ============================================================================

/**
 * List ML models with optional filtering and pagination
 */
export async function listModels(
  filters?: ModelFilters
): Promise<PaginatedResponse<MLModel>> {
  const queryString = filters ? buildQueryParams(filters) : '';
  const response = await get<PaginatedResponse<MLModel>>(
    `/ml/models${queryString}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch ML models');
  }

  return response.data;
}

/**
 * Create a new ML model
 */
export async function createModel(
  request: CreateModelRequest
): Promise<MLModel> {
  const response = await post<MLModel>('/ml/models', request);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to create ML model');
  }

  return response.data;
}

/**
 * Get ML model details by ID
 */
export async function getModel(modelId: UUID): Promise<MLModel> {
  const response = await get<MLModel>(`/ml/models/${modelId}`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch ML model');
  }

  return response.data;
}

/**
 * Update ML model configuration
 */
export async function updateModel(
  modelId: UUID,
  updates: Partial<CreateModelRequest>
): Promise<MLModel> {
  const response = await patch<MLModel>(`/ml/models/${modelId}`, updates);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to update ML model');
  }

  return response.data;
}

/**
 * Delete an ML model
 */
export async function deleteModel(modelId: UUID): Promise<void> {
  const response = await del<void>(`/ml/models/${modelId}`);

  if (!response.success) {
    throw new Error(response.error?.message || 'Failed to delete ML model');
  }
}

/**
 * Train or retrain an ML model
 */
export async function trainModel(
  modelId: UUID,
  request?: TrainModelRequest
): Promise<MLModel> {
  const response = await post<MLModel>(
    `/ml/models/${modelId}/train`,
    request || {}
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to train ML model');
  }

  return response.data;
}

/**
 * Get model training status
 */
export async function getTrainingStatus(modelId: UUID): Promise<{
  status: MLModelStatus;
  progress: number;
  currentEpoch?: number;
  totalEpochs?: number;
  currentLoss?: number;
  estimatedTimeRemaining?: number;
  error?: string;
}> {
  const response = await get<{
    status: MLModelStatus;
    progress: number;
    currentEpoch?: number;
    totalEpochs?: number;
    currentLoss?: number;
    estimatedTimeRemaining?: number;
    error?: string;
  }>(`/ml/models/${modelId}/training-status`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch training status');
  }

  return response.data;
}

/**
 * Run prediction using a trained model
 */
export async function predict(
  modelId: UUID,
  request: PredictionRequest
): Promise<PredictionResponse> {
  const response = await post<PredictionResponse>(
    `/ml/models/${modelId}/predict`,
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to run prediction');
  }

  return response.data;
}

/**
 * Get model performance metrics
 */
export async function getModelMetrics(modelId: UUID): Promise<ModelMetrics> {
  const response = await get<ModelMetrics>(`/ml/models/${modelId}/metrics`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch model metrics');
  }

  return response.data;
}

/**
 * Retrain model with new data
 */
export async function retrainModel(
  modelId: UUID,
  request: ModelRetrainingRequest
): Promise<MLModel> {
  const response = await post<MLModel>(
    `/ml/models/${modelId}/retrain`,
    request
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to retrain model');
  }

  return response.data;
}

/**
 * Deploy model for production use
 */
export async function deployModel(modelId: UUID): Promise<MLModel> {
  const response = await post<MLModel>(`/ml/models/${modelId}/deploy`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to deploy model');
  }

  return response.data;
}

/**
 * Archive a model
 */
export async function archiveModel(modelId: UUID): Promise<MLModel> {
  const response = await post<MLModel>(`/ml/models/${modelId}/archive`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to archive model');
  }

  return response.data;
}

/**
 * Analyze features for model training
 */
export async function analyzeFeatures(
  query: string,
  targetColumn: string
): Promise<FeatureAnalysis> {
  const response = await post<FeatureAnalysis>('/ml/features/analyze', {
    query,
    targetColumn,
  });

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to analyze features');
  }

  return response.data;
}

/**
 * Export model for external use
 */
export async function exportModel(
  modelId: UUID,
  format: 'onnx' | 'pmml' | 'pickle' | 'json' = 'json'
): Promise<Blob> {
  const response = await get<Blob>(
    `/ml/models/${modelId}/export${buildQueryParams({ format })}`
  );

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to export model');
  }

  return response.data;
}

/**
 * Get model version history
 */
export async function getModelVersions(modelId: UUID): Promise<MLModel[]> {
  const response = await get<MLModel[]>(`/ml/models/${modelId}/versions`);

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to fetch model versions');
  }

  return response.data;
}

/**
 * Rollback to a previous model version
 */
export async function rollbackModel(
  modelId: UUID,
  version: number
): Promise<MLModel> {
  const response = await post<MLModel>(`/ml/models/${modelId}/rollback`, {
    version,
  });

  if (!response.success || !response.data) {
    throw new Error(response.error?.message || 'Failed to rollback model');
  }

  return response.data;
}

// ============================================================================
// Export Service Object (Alternative Pattern)
// ============================================================================

export const mlService = {
  listModels,
  createModel,
  getModel,
  updateModel,
  deleteModel,
  trainModel,
  getTrainingStatus,
  predict,
  getModelMetrics,
  retrainModel,
  deployModel,
  archiveModel,
  analyzeFeatures,
  exportModel,
  getModelVersions,
  rollbackModel,
};
