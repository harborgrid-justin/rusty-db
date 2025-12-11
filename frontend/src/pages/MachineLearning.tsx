import { useState } from 'react';
import { motion } from 'framer-motion';
import { LineChart, Line, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { Card, CardHeader } from '../components/common/Card';
import { Button } from '../components/common/Button';
import { Input, Textarea } from '../components/common/Input';
import { Select } from '../components/common/Select';
import { Tabs, TabList, TabPanel, TabPanels } from '../components/common/Tabs';
import { Table, Column } from '../components/common/Table';
import { Badge, StatusBadge } from '../components/common/Badge';

// ML Model types
interface MLModel {
  id: string;
  name: string;
  type: 'classification' | 'regression' | 'clustering' | 'neural_network';
  status: 'active' | 'training' | 'inactive' | 'failed';
  accuracy: number;
  createdAt: string;
  lastTrained: string;
  features: number;
  version: string;
}

interface TrainingJob {
  id: string;
  modelId: string;
  status: 'running' | 'completed' | 'failed' | 'pending';
  progress: number;
  startTime: string;
  endTime?: string;
  accuracy?: number;
  loss?: number;
}

interface Prediction {
  id: string;
  modelId: string;
  input: Record<string, any>;
  output: any;
  confidence: number;
  timestamp: string;
}

// Mock data
const MOCK_MODELS: MLModel[] = [
  {
    id: 'm1',
    name: 'Customer Churn Predictor',
    type: 'classification',
    status: 'active',
    accuracy: 94.5,
    createdAt: '2025-01-15',
    lastTrained: '2025-02-20',
    features: 12,
    version: '2.1.0',
  },
  {
    id: 'm2',
    name: 'Sales Forecaster',
    type: 'regression',
    status: 'active',
    accuracy: 87.3,
    createdAt: '2025-01-20',
    lastTrained: '2025-03-01',
    features: 8,
    version: '1.5.0',
  },
  {
    id: 'm3',
    name: 'Customer Segmentation',
    type: 'clustering',
    status: 'training',
    accuracy: 0,
    createdAt: '2025-03-10',
    lastTrained: '2025-03-10',
    features: 15,
    version: '1.0.0',
  },
  {
    id: 'm4',
    name: 'Image Classifier',
    type: 'neural_network',
    status: 'inactive',
    accuracy: 92.1,
    createdAt: '2025-02-01',
    lastTrained: '2025-02-15',
    features: 784,
    version: '3.0.0',
  },
];

const MOCK_TRAINING_HISTORY = [
  { epoch: 1, accuracy: 65.2, loss: 0.85, valAccuracy: 63.1, valLoss: 0.92 },
  { epoch: 2, accuracy: 72.5, loss: 0.68, valAccuracy: 70.3, valLoss: 0.74 },
  { epoch: 3, accuracy: 78.9, loss: 0.54, valAccuracy: 76.8, valLoss: 0.61 },
  { epoch: 4, accuracy: 83.4, loss: 0.42, valAccuracy: 81.2, valLoss: 0.49 },
  { epoch: 5, accuracy: 87.1, loss: 0.35, valAccuracy: 85.3, valLoss: 0.41 },
  { epoch: 6, accuracy: 89.8, loss: 0.29, valAccuracy: 87.9, valLoss: 0.36 },
  { epoch: 7, accuracy: 91.5, loss: 0.25, valAccuracy: 89.6, valLoss: 0.32 },
  { epoch: 8, accuracy: 92.9, loss: 0.22, valAccuracy: 91.1, valLoss: 0.29 },
  { epoch: 9, accuracy: 94.1, loss: 0.19, valAccuracy: 92.3, valLoss: 0.27 },
  { epoch: 10, accuracy: 94.8, loss: 0.17, valAccuracy: 93.1, valLoss: 0.25 },
];

const MOCK_CONFUSION_MATRIX = [
  [85, 5, 3, 2],
  [4, 88, 6, 1],
  [2, 5, 90, 3],
  [1, 2, 4, 93],
];

const MOCK_FEATURE_IMPORTANCE = [
  { feature: 'tenure', importance: 0.245 },
  { feature: 'monthly_charges', importance: 0.189 },
  { feature: 'total_charges', importance: 0.156 },
  { feature: 'contract_type', importance: 0.134 },
  { feature: 'payment_method', importance: 0.098 },
  { feature: 'internet_service', importance: 0.087 },
  { feature: 'tech_support', importance: 0.056 },
  { feature: 'online_security', importance: 0.035 },
];

const MODEL_TYPES = [
  { value: 'classification', label: 'Classification' },
  { value: 'regression', label: 'Regression' },
  { value: 'clustering', label: 'Clustering' },
  { value: 'neural_network', label: 'Neural Network' },
];

const ALGORITHMS = [
  { value: 'logistic_regression', label: 'Logistic Regression' },
  { value: 'random_forest', label: 'Random Forest' },
  { value: 'xgboost', label: 'XGBoost' },
  { value: 'svm', label: 'Support Vector Machine' },
  { value: 'neural_network', label: 'Neural Network' },
  { value: 'kmeans', label: 'K-Means' },
  { value: 'dbscan', label: 'DBSCAN' },
];

export default function MachineLearning() {
  const [models, setModels] = useState<MLModel[]>(MOCK_MODELS);
  const [selectedModel, setSelectedModel] = useState<MLModel | null>(MOCK_MODELS[0]);
  const [activeTab, setActiveTab] = useState('catalog');

  // Training wizard state
  const [newModelName, setNewModelName] = useState('');
  const [newModelType, setNewModelType] = useState('classification');
  const [selectedAlgorithm, setSelectedAlgorithm] = useState('random_forest');
  const [trainingDataset, setTrainingDataset] = useState('');
  const [trainingParams, setTrainingParams] = useState('{"max_depth": 10, "n_estimators": 100}');
  const [isTraining, setIsTraining] = useState(false);

  // Prediction state
  const [predictionInput, setPredictionInput] = useState('{"tenure": 24, "monthly_charges": 65.5}');
  const [predictionResult, setPredictionResult] = useState<any>(null);
  const [isPredicting, setIsPredicting] = useState(false);

  // Model comparison
  const [comparisonModels, setComparisonModels] = useState<string[]>([]);

  // Start training
  const startTraining = async () => {
    setIsTraining(true);
    // Simulate training
    setTimeout(() => {
      const newModel: MLModel = {
        id: `m${Date.now()}`,
        name: newModelName,
        type: newModelType as any,
        status: 'training',
        accuracy: 0,
        createdAt: new Date().toISOString().split('T')[0],
        lastTrained: new Date().toISOString().split('T')[0],
        features: 10,
        version: '1.0.0',
      };
      setModels(prev => [...prev, newModel]);
      setIsTraining(false);
      setNewModelName('');
      setTrainingDataset('');
    }, 2000);
  };

  // Run prediction
  const runPrediction = async () => {
    setIsPredicting(true);
    // Simulate prediction
    setTimeout(() => {
      setP redictionResult({
        prediction: 'churn',
        confidence: 0.876,
        probabilities: {
          churn: 0.876,
          stay: 0.124,
        },
        explanation: 'High monthly charges and short tenure indicate churn risk',
      });
      setIsPredicting(false);
    }, 500);
  };

  // Toggle model comparison
  const toggleComparison = (modelId: string) => {
    setComparisonModels(prev =>
      prev.includes(modelId)
        ? prev.filter(id => id !== modelId)
        : [...prev, modelId]
    );
  };

  const tabs = [
    { id: 'catalog', label: 'Model Catalog' },
    { id: 'train', label: 'Train Model' },
    { id: 'predict', label: 'Predictions' },
    { id: 'metrics', label: 'Metrics' },
    { id: 'compare', label: 'Compare Models' },
  ];

  const modelColumns: Column<MLModel>[] = [
    { key: 'name', header: 'Model Name', sortable: true },
    {
      key: 'type',
      header: 'Type',
      render: (val) => <Badge variant="primary" size="sm">{val}</Badge>,
    },
    {
      key: 'status',
      header: 'Status',
      render: (val) => (
        <StatusBadge
          status={val === 'active' ? 'active' : val === 'training' ? 'pending' : 'inactive'}
        />
      ),
    },
    {
      key: 'accuracy',
      header: 'Accuracy',
      render: (val) => (val > 0 ? `${val.toFixed(1)}%` : 'N/A'),
    },
    { key: 'version', header: 'Version' },
    { key: 'lastTrained', header: 'Last Trained' },
    {
      key: 'id',
      header: 'Actions',
      render: (_, model) => (
        <div className="flex items-center space-x-2">
          <Button size="sm" variant="ghost" onClick={() => setSelectedModel(model)}>
            View
          </Button>
          <Button size="sm" variant="ghost">
            Deploy
          </Button>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Machine Learning</h1>
          <p className="text-gray-400 mt-1">In-database ML model training and inference</p>
        </div>
        <div className="flex items-center space-x-3">
          <Button variant="secondary">Import Model</Button>
          <Button variant="primary">New Model</Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Total Models</p>
              <p className="text-2xl font-bold text-white">{models.length}</p>
            </div>
            <div className="text-3xl">🤖</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Active Models</p>
              <p className="text-2xl font-bold text-white">
                {models.filter(m => m.status === 'active').length}
              </p>
            </div>
            <div className="text-3xl">✅</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Avg Accuracy</p>
              <p className="text-2xl font-bold text-white">
                {(
                  models.reduce((sum, m) => sum + m.accuracy, 0) / models.length
                ).toFixed(1)}%
              </p>
            </div>
            <div className="text-3xl">📊</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Training Jobs</p>
              <p className="text-2xl font-bold text-white">
                {models.filter(m => m.status === 'training').length}
              </p>
            </div>
            <div className="text-3xl">⚡</div>
          </div>
        </Card>
      </div>

      {/* Main Content */}
      <Tabs value={activeTab} onChange={setActiveTab}>
        <TabList tabs={tabs} />
        <TabPanels className="mt-6">
          {/* Model Catalog */}
          <TabPanel tabId="catalog">
            <Card>
              <CardHeader title="Model Catalog" />
              <Table
                columns={modelColumns}
                data={models}
                onRowClick={setSelectedModel}
                sortable
              />
            </Card>

            {selectedModel && (
              <Card className="mt-6">
                <CardHeader title={`${selectedModel.name} - Details`} />
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div className="space-y-3">
                    <div>
                      <span className="text-sm text-gray-400">Model ID:</span>
                      <span className="ml-2 text-white">{selectedModel.id}</span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Type:</span>
                      <Badge variant="primary" size="sm" className="ml-2">
                        {selectedModel.type}
                      </Badge>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Status:</span>
                      <StatusBadge
                        status={
                          selectedModel.status === 'active'
                            ? 'active'
                            : selectedModel.status === 'training'
                            ? 'pending'
                            : 'inactive'
                        }
                        size="sm"
                      />
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Features:</span>
                      <span className="ml-2 text-white">{selectedModel.features}</span>
                    </div>
                  </div>
                  <div className="space-y-3">
                    <div>
                      <span className="text-sm text-gray-400">Accuracy:</span>
                      <span className="ml-2 text-white font-bold">
                        {selectedModel.accuracy.toFixed(1)}%
                      </span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Version:</span>
                      <span className="ml-2 text-white">{selectedModel.version}</span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Created:</span>
                      <span className="ml-2 text-white">{selectedModel.createdAt}</span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Last Trained:</span>
                      <span className="ml-2 text-white">{selectedModel.lastTrained}</span>
                    </div>
                  </div>
                </div>
              </Card>
            )}
          </TabPanel>

          {/* Train Model */}
          <TabPanel tabId="train">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <Card>
                <CardHeader title="Training Wizard" />
                <div className="space-y-4">
                  <Input
                    label="Model Name"
                    value={newModelName}
                    onChange={(e) => setNewModelName(e.target.value)}
                    fullWidth
                    placeholder="e.g., Customer Churn Predictor"
                  />
                  <Select
                    label="Model Type"
                    value={newModelType}
                    onChange={(e) => setNewModelType(e.target.value)}
                    options={MODEL_TYPES}
                    fullWidth
                  />
                  <Select
                    label="Algorithm"
                    value={selectedAlgorithm}
                    onChange={(e) => setSelectedAlgorithm(e.target.value)}
                    options={ALGORITHMS}
                    fullWidth
                  />
                  <Input
                    label="Training Dataset"
                    value={trainingDataset}
                    onChange={(e) => setTrainingDataset(e.target.value)}
                    fullWidth
                    placeholder="SELECT * FROM customer_data"
                  />
                  <Textarea
                    label="Hyperparameters (JSON)"
                    value={trainingParams}
                    onChange={(e) => setTrainingParams(e.target.value)}
                    rows={4}
                    fullWidth
                  />
                  <Button
                    variant="primary"
                    fullWidth
                    onClick={startTraining}
                    loading={isTraining}
                    disabled={!newModelName || !trainingDataset}
                  >
                    Start Training
                  </Button>
                </div>
              </Card>

              <Card>
                <CardHeader title="Training History" />
                <ResponsiveContainer width="100%" height={300}>
                  <LineChart data={MOCK_TRAINING_HISTORY}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                    <XAxis dataKey="epoch" stroke="#9CA3AF" />
                    <YAxis stroke="#9CA3AF" />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#1F2937',
                        border: '1px solid #374151',
                        borderRadius: '0.5rem',
                      }}
                    />
                    <Legend />
                    <Line
                      type="monotone"
                      dataKey="accuracy"
                      stroke="#3B82F6"
                      strokeWidth={2}
                      name="Training Accuracy"
                    />
                    <Line
                      type="monotone"
                      dataKey="valAccuracy"
                      stroke="#10B981"
                      strokeWidth={2}
                      name="Validation Accuracy"
                    />
                  </LineChart>
                </ResponsiveContainer>

                <ResponsiveContainer width="100%" height={300} className="mt-6">
                  <LineChart data={MOCK_TRAINING_HISTORY}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                    <XAxis dataKey="epoch" stroke="#9CA3AF" />
                    <YAxis stroke="#9CA3AF" />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#1F2937',
                        border: '1px solid #374151',
                        borderRadius: '0.5rem',
                      }}
                    />
                    <Legend />
                    <Line
                      type="monotone"
                      dataKey="loss"
                      stroke="#EF4444"
                      strokeWidth={2}
                      name="Training Loss"
                    />
                    <Line
                      type="monotone"
                      dataKey="valLoss"
                      stroke="#F59E0B"
                      strokeWidth={2}
                      name="Validation Loss"
                    />
                  </LineChart>
                </ResponsiveContainer>
              </Card>
            </div>
          </TabPanel>

          {/* Predictions */}
          <TabPanel tabId="predict">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <Card>
                <CardHeader title="Prediction Interface" />
                <div className="space-y-4">
                  <Select
                    label="Select Model"
                    value={selectedModel?.id || ''}
                    onChange={(e) =>
                      setSelectedModel(models.find(m => m.id === e.target.value) || null)
                    }
                    options={models
                      .filter(m => m.status === 'active')
                      .map(m => ({ value: m.id, label: m.name }))}
                    fullWidth
                  />
                  <Textarea
                    label="Input Features (JSON)"
                    value={predictionInput}
                    onChange={(e) => setPredictionInput(e.target.value)}
                    rows={8}
                    fullWidth
                    helperText="Enter feature values in JSON format"
                  />
                  <Button
                    variant="primary"
                    fullWidth
                    onClick={runPrediction}
                    loading={isPredicting}
                    disabled={!selectedModel}
                  >
                    Run Prediction
                  </Button>
                </div>
              </Card>

              <Card>
                <CardHeader title="Prediction Results" />
                {predictionResult ? (
                  <div className="space-y-4">
                    <div className="p-4 bg-green-900 bg-opacity-20 border border-green-500 rounded-lg">
                      <p className="text-sm text-gray-400 mb-2">Prediction:</p>
                      <p className="text-2xl font-bold text-white">
                        {predictionResult.prediction}
                      </p>
                      <p className="text-sm text-gray-400 mt-2">
                        Confidence: {(predictionResult.confidence * 100).toFixed(1)}%
                      </p>
                    </div>

                    <div>
                      <p className="text-sm text-gray-400 mb-2">Probabilities:</p>
                      <div className="space-y-2">
                        {Object.entries(predictionResult.probabilities).map(
                          ([key, value]: [string, any]) => (
                            <div key={key} className="flex items-center justify-between">
                              <span className="text-white">{key}:</span>
                              <div className="flex items-center space-x-2">
                                <div className="w-32 h-2 bg-gray-700 rounded-full overflow-hidden">
                                  <div
                                    className="h-full bg-blue-500"
                                    style={{ width: `${value * 100}%` }}
                                  ></div>
                                </div>
                                <span className="text-gray-400 text-sm w-12 text-right">
                                  {(value * 100).toFixed(1)}%
                                </span>
                              </div>
                            </div>
                          )
                        )}
                      </div>
                    </div>

                    <div className="p-3 bg-gray-900 rounded-lg">
                      <p className="text-sm text-gray-400 mb-1">Explanation:</p>
                      <p className="text-sm text-white">{predictionResult.explanation}</p>
                    </div>
                  </div>
                ) : (
                  <p className="text-gray-400 text-center py-12">
                    Run a prediction to see results
                  </p>
                )}
              </Card>
            </div>
          </TabPanel>

          {/* Metrics */}
          <TabPanel tabId="metrics">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <Card>
                <CardHeader title="Confusion Matrix" />
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr>
                        <th className="p-2"></th>
                        <th className="p-2 text-xs text-gray-400">Pred 0</th>
                        <th className="p-2 text-xs text-gray-400">Pred 1</th>
                        <th className="p-2 text-xs text-gray-400">Pred 2</th>
                        <th className="p-2 text-xs text-gray-400">Pred 3</th>
                      </tr>
                    </thead>
                    <tbody>
                      {MOCK_CONFUSION_MATRIX.map((row, i) => (
                        <tr key={i}>
                          <td className="p-2 text-xs text-gray-400">Actual {i}</td>
                          {row.map((val, j) => (
                            <td
                              key={j}
                              className="p-2 text-center"
                              style={{
                                backgroundColor:
                                  i === j
                                    ? `rgba(16, 185, 129, ${val / 100})`
                                    : `rgba(239, 68, 68, ${val / 100})`,
                              }}
                            >
                              <span className="text-white font-bold">{val}</span>
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Card>

              <Card>
                <CardHeader title="Feature Importance" />
                <ResponsiveContainer width="100%" height={300}>
                  <BarChart data={MOCK_FEATURE_IMPORTANCE} layout="vertical">
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                    <XAxis type="number" stroke="#9CA3AF" />
                    <YAxis dataKey="feature" type="category" stroke="#9CA3AF" width={120} />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#1F2937',
                        border: '1px solid #374151',
                        borderRadius: '0.5rem',
                      }}
                    />
                    <Bar dataKey="importance" fill="#3B82F6" />
                  </BarChart>
                </ResponsiveContainer>
              </Card>

              <Card>
                <CardHeader title="Model Performance Metrics" />
                <div className="grid grid-cols-2 gap-4">
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-1">Precision</p>
                    <p className="text-2xl font-bold text-white">91.2%</p>
                  </div>
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-1">Recall</p>
                    <p className="text-2xl font-bold text-white">89.7%</p>
                  </div>
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-1">F1 Score</p>
                    <p className="text-2xl font-bold text-white">90.4%</p>
                  </div>
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-1">AUC-ROC</p>
                    <p className="text-2xl font-bold text-white">0.945</p>
                  </div>
                </div>
              </Card>

              <Card>
                <CardHeader title="ROC Curve (Placeholder)" />
                <div className="h-64 bg-gray-900 rounded-lg flex items-center justify-center">
                  <p className="text-gray-400">ROC Curve visualization would go here</p>
                </div>
              </Card>
            </div>
          </TabPanel>

          {/* Compare Models */}
          <TabPanel tabId="compare">
            <Card>
              <CardHeader title="Model Comparison" />
              <div className="mb-6">
                <p className="text-sm text-gray-400 mb-3">Select models to compare:</p>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                  {models.map(model => (
                    <button
                      key={model.id}
                      onClick={() => toggleComparison(model.id)}
                      className={`p-3 rounded-lg border transition-colors ${
                        comparisonModels.includes(model.id)
                          ? 'bg-blue-500 bg-opacity-20 border-blue-500'
                          : 'bg-gray-800 border-gray-700 hover:border-gray-600'
                      }`}
                    >
                      <p className="text-sm text-white font-medium">{model.name}</p>
                      <p className="text-xs text-gray-400 mt-1">
                        {model.accuracy.toFixed(1)}% accuracy
                      </p>
                    </button>
                  ))}
                </div>
              </div>

              {comparisonModels.length > 0 && (
                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="border-b border-gray-700">
                        <th className="p-3 text-left text-sm text-gray-400">Metric</th>
                        {comparisonModels.map(id => {
                          const model = models.find(m => m.id === id);
                          return (
                            <th key={id} className="p-3 text-center text-sm text-gray-400">
                              {model?.name}
                            </th>
                          );
                        })}
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-gray-700">
                      <tr>
                        <td className="p-3 text-sm text-gray-400">Accuracy</td>
                        {comparisonModels.map(id => {
                          const model = models.find(m => m.id === id);
                          return (
                            <td key={id} className="p-3 text-center text-white font-bold">
                              {model?.accuracy.toFixed(1)}%
                            </td>
                          );
                        })}
                      </tr>
                      <tr>
                        <td className="p-3 text-sm text-gray-400">Type</td>
                        {comparisonModels.map(id => {
                          const model = models.find(m => m.id === id);
                          return (
                            <td key={id} className="p-3 text-center">
                              <Badge variant="primary" size="sm">
                                {model?.type}
                              </Badge>
                            </td>
                          );
                        })}
                      </tr>
                      <tr>
                        <td className="p-3 text-sm text-gray-400">Features</td>
                        {comparisonModels.map(id => {
                          const model = models.find(m => m.id === id);
                          return (
                            <td key={id} className="p-3 text-center text-white">
                              {model?.features}
                            </td>
                          );
                        })}
                      </tr>
                      <tr>
                        <td className="p-3 text-sm text-gray-400">Version</td>
                        {comparisonModels.map(id => {
                          const model = models.find(m => m.id === id);
                          return (
                            <td key={id} className="p-3 text-center text-white">
                              {model?.version}
                            </td>
                          );
                        })}
                      </tr>
                      <tr>
                        <td className="p-3 text-sm text-gray-400">Last Trained</td>
                        {comparisonModels.map(id => {
                          const model = models.find(m => m.id === id);
                          return (
                            <td key={id} className="p-3 text-center text-white">
                              {model?.lastTrained}
                            </td>
                          );
                        })}
                      </tr>
                    </tbody>
                  </table>
                </div>
              )}
            </Card>
          </TabPanel>
        </TabPanels>
      </Tabs>
    </div>
  );
}
