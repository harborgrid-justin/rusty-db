import { useState, useRef, useEffect } from 'react';
import { motion } from 'framer-motion';
import { Card, CardHeader } from '../components/common/Card';
import { Button } from '../components/common/Button';
import { Input, Textarea } from '../components/common/Input';
import { Select } from '../components/common/Select';
import { Tabs, TabList, TabPanel, TabPanels } from '../components/common/Tabs';
import { Table, Column } from '../components/common/Table';
import { Badge } from '../components/common/Badge';

// Spatial data types
interface SpatialFeature {
  id: string;
  name: string;
  type: 'Point' | 'LineString' | 'Polygon';
  coordinates: number[] | number[][] | number[][][];
  properties: Record<string, any>;
  layer: string;
}

interface SpatialLayer {
  id: string;
  name: string;
  type: string;
  visible: boolean;
  color: string;
  featureCount: number;
}

interface SpatialIndex {
  name: string;
  type: 'R-Tree' | 'Quad-Tree' | 'Grid';
  layer: string;
  status: 'active' | 'building' | 'inactive';
}

// Mock data
const MOCK_FEATURES: SpatialFeature[] = [
  {
    id: 'f1',
    name: 'City Center',
    type: 'Point',
    coordinates: [400, 300],
    properties: { population: 50000, type: 'urban' },
    layer: 'cities',
  },
  {
    id: 'f2',
    name: 'Park',
    type: 'Polygon',
    coordinates: [[[200, 200], [300, 200], [300, 280], [200, 280], [200, 200]]],
    properties: { area: 10000, type: 'recreation' },
    layer: 'parks',
  },
  {
    id: 'f3',
    name: 'Main Street',
    type: 'LineString',
    coordinates: [[100, 100], [200, 150], [350, 200], [500, 250]],
    properties: { length: 500, type: 'highway' },
    layer: 'roads',
  },
  {
    id: 'f4',
    name: 'Suburb',
    type: 'Point',
    coordinates: [600, 400],
    properties: { population: 15000, type: 'suburban' },
    layer: 'cities',
  },
];

const MOCK_LAYERS: SpatialLayer[] = [
  { id: 'cities', name: 'Cities', type: 'Point', visible: true, color: '#3B82F6', featureCount: 2 },
  { id: 'roads', name: 'Roads', type: 'LineString', visible: true, color: '#F59E0B', featureCount: 1 },
  { id: 'parks', name: 'Parks', type: 'Polygon', visible: true, color: '#10B981', featureCount: 1 },
  { id: 'water', name: 'Water Bodies', type: 'Polygon', visible: false, color: '#06B6D4', featureCount: 0 },
];

const MOCK_INDEXES: SpatialIndex[] = [
  { name: 'cities_rtree', type: 'R-Tree', layer: 'cities', status: 'active' },
  { name: 'roads_rtree', type: 'R-Tree', layer: 'roads', status: 'active' },
  { name: 'parks_grid', type: 'Grid', layer: 'parks', status: 'active' },
];

const QUERY_TYPES = [
  { value: 'bbox', label: 'Bounding Box' },
  { value: 'radius', label: 'Radius Search' },
  { value: 'polygon', label: 'Polygon Intersection' },
  { value: 'nearest', label: 'Nearest Neighbor' },
  { value: 'contains', label: 'Contains' },
  { value: 'intersects', label: 'Intersects' },
];

export default function SpatialQueries() {
  const [features, setFeatures] = useState<SpatialFeature[]>(MOCK_FEATURES);
  const [layers, setLayers] = useState<SpatialLayer[]>(MOCK_LAYERS);
  const [indexes, setIndexes] = useState<SpatialIndex[]>(MOCK_INDEXES);
  const [selectedFeature, setSelectedFeature] = useState<SpatialFeature | null>(null);
  const [activeTab, setActiveTab] = useState('map');

  // Query builder state
  const [queryType, setQueryType] = useState('bbox');
  const [queryParams, setQueryParams] = useState('{"minX": 100, "minY": 100, "maxX": 500, "maxY": 400}');
  const [queryResults, setQueryResults] = useState<SpatialFeature[]>([]);
  const [isQuerying, setIsQuerying] = useState(false);

  // Drawing state
  const [drawingMode, setDrawingMode] = useState<'select' | 'bbox' | 'radius' | 'polygon'>('select');
  const [drawStart, setDrawStart] = useState<{ x: number; y: number } | null>(null);
  const [drawEnd, setDrawEnd] = useState<{ x: number; y: number } | null>(null);

  // Routing state
  const [routeStart, setRouteStart] = useState<string>('');
  const [routeEnd, setRouteEnd] = useState<string>('');
  const [routePath, setRoutePath] = useState<number[][] | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Render map
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Draw background
    ctx.fillStyle = '#1F2937';
    ctx.fillRect(0, 0, width, height);

    // Draw grid
    ctx.strokeStyle = '#374151';
    ctx.lineWidth = 1;
    for (let x = 0; x < width; x += 50) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }
    for (let y = 0; y < height; y += 50) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    // Draw features
    features.forEach(feature => {
      const layer = layers.find(l => l.id === feature.layer);
      if (!layer || !layer.visible) return;

      ctx.strokeStyle = layer.color;
      ctx.fillStyle = layer.color + '40'; // Add transparency

      if (feature.type === 'Point') {
        const [x, y] = feature.coordinates as number[];
        ctx.beginPath();
        ctx.arc(x, y, 8, 0, 2 * Math.PI);
        ctx.fill();
        if (selectedFeature?.id === feature.id) {
          ctx.strokeStyle = '#FFFFFF';
          ctx.lineWidth = 3;
          ctx.stroke();
        }
        // Label
        ctx.fillStyle = '#FFFFFF';
        ctx.font = '11px sans-serif';
        ctx.fillText(feature.name, x + 12, y + 4);
      } else if (feature.type === 'LineString') {
        const coords = feature.coordinates as number[][];
        ctx.beginPath();
        ctx.moveTo(coords[0][0], coords[0][1]);
        coords.slice(1).forEach(([x, y]) => ctx.lineTo(x, y));
        ctx.strokeStyle = layer.color;
        ctx.lineWidth = 3;
        ctx.stroke();
      } else if (feature.type === 'Polygon') {
        const coords = feature.coordinates as number[][][];
        const ring = coords[0];
        ctx.beginPath();
        ctx.moveTo(ring[0][0], ring[0][1]);
        ring.slice(1).forEach(([x, y]) => ctx.lineTo(x, y));
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
      }
    });

    // Draw selection/query area
    if (drawingMode === 'bbox' && drawStart && drawEnd) {
      ctx.strokeStyle = '#EF4444';
      ctx.lineWidth = 2;
      ctx.setLineDash([5, 5]);
      ctx.strokeRect(
        drawStart.x,
        drawStart.y,
        drawEnd.x - drawStart.x,
        drawEnd.y - drawStart.y
      );
      ctx.setLineDash([]);
    } else if (drawingMode === 'radius' && drawStart && drawEnd) {
      const radius = Math.sqrt(
        Math.pow(drawEnd.x - drawStart.x, 2) + Math.pow(drawEnd.y - drawStart.y, 2)
      );
      ctx.strokeStyle = '#EF4444';
      ctx.lineWidth = 2;
      ctx.setLineDash([5, 5]);
      ctx.beginPath();
      ctx.arc(drawStart.x, drawStart.y, radius, 0, 2 * Math.PI);
      ctx.stroke();
      ctx.setLineDash([]);
    }

    // Draw route path
    if (routePath && routePath.length > 1) {
      ctx.strokeStyle = '#8B5CF6';
      ctx.lineWidth = 4;
      ctx.setLineDash([10, 5]);
      ctx.beginPath();
      ctx.moveTo(routePath[0][0], routePath[0][1]);
      routePath.slice(1).forEach(([x, y]) => ctx.lineTo(x, y));
      ctx.stroke();
      ctx.setLineDash([]);
    }
  }, [features, layers, selectedFeature, drawStart, drawEnd, drawingMode, routePath]);

  // Handle canvas interactions
  const handleCanvasMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (drawingMode === 'select') {
      // Find clicked feature
      const clicked = features.find(f => {
        if (f.type === 'Point') {
          const [fx, fy] = f.coordinates as number[];
          return Math.sqrt(Math.pow(x - fx, 2) + Math.pow(y - fy, 2)) < 10;
        }
        return false;
      });
      setSelectedFeature(clicked || null);
    } else {
      setDrawStart({ x, y });
      setDrawEnd({ x, y });
    }
  };

  const handleCanvasMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!drawStart || drawingMode === 'select') return;

    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setDrawEnd({ x, y });
  };

  const handleCanvasMouseUp = () => {
    if (drawStart && drawEnd && drawingMode !== 'select') {
      // Perform query based on drawn area
      executeDrawnQuery();
    }
  };

  // Execute query
  const executeQuery = async () => {
    setIsQuerying(true);
    // Simulate API call
    setTimeout(() => {
      // Simple mock filtering
      const filtered = features.filter((_, i) => i % 2 === 0);
      setQueryResults(filtered);
      setIsQuerying(false);
    }, 500);
  };

  // Execute drawn query
  const executeDrawnQuery = () => {
    if (!drawStart || !drawEnd) return;

    const results: SpatialFeature[] = [];

    if (drawingMode === 'bbox') {
      const minX = Math.min(drawStart.x, drawEnd.x);
      const maxX = Math.max(drawStart.x, drawEnd.x);
      const minY = Math.min(drawStart.y, drawEnd.y);
      const maxY = Math.max(drawStart.y, drawEnd.y);

      features.forEach(f => {
        if (f.type === 'Point') {
          const [x, y] = f.coordinates as number[];
          if (x >= minX && x <= maxX && y >= minY && y <= maxY) {
            results.push(f);
          }
        }
      });
    } else if (drawingMode === 'radius') {
      const radius = Math.sqrt(
        Math.pow(drawEnd.x - drawStart.x, 2) + Math.pow(drawEnd.y - drawStart.y, 2)
      );

      features.forEach(f => {
        if (f.type === 'Point') {
          const [x, y] = f.coordinates as number[];
          const dist = Math.sqrt(Math.pow(x - drawStart.x, 2) + Math.pow(y - drawStart.y, 2));
          if (dist <= radius) {
            results.push(f);
          }
        }
      });
    }

    setQueryResults(results);
    setDrawStart(null);
    setDrawEnd(null);
  };

  // Calculate route
  const calculateRoute = () => {
    const start = features.find(f => f.id === routeStart);
    const end = features.find(f => f.id === routeEnd);

    if (start && end && start.type === 'Point' && end.type === 'Point') {
      // Simple straight line route (placeholder)
      const startCoords = start.coordinates as number[];
      const endCoords = end.coordinates as number[];
      setRoutePath([startCoords, endCoords]);
    }
  };

  // Toggle layer visibility
  const toggleLayer = (layerId: string) => {
    setLayers(prev =>
      prev.map(layer =>
        layer.id === layerId ? { ...layer, visible: !layer.visible } : layer
      )
    );
  };

  const tabs = [
    { id: 'map', label: 'Map View' },
    { id: 'query', label: 'Query Builder' },
    { id: 'layers', label: 'Layers' },
    { id: 'indexes', label: 'Spatial Indexes' },
    { id: 'routing', label: 'Routing' },
  ];

  const featureColumns: Column<SpatialFeature>[] = [
    { key: 'id', header: 'ID', width: '80px' },
    { key: 'name', header: 'Name' },
    { key: 'type', header: 'Type', render: (val) => <Badge variant="primary" size="sm">{val}</Badge> },
    { key: 'layer', header: 'Layer' },
  ];

  const indexColumns: Column<SpatialIndex>[] = [
    { key: 'name', header: 'Index Name' },
    { key: 'type', header: 'Type', render: (val) => <Badge variant="info" size="sm">{val}</Badge> },
    { key: 'layer', header: 'Layer' },
    {
      key: 'status',
      header: 'Status',
      render: (val) => (
        <Badge
          variant={val === 'active' ? 'success' : val === 'building' ? 'warning' : 'neutral'}
          size="sm"
          dot
        >
          {val}
        </Badge>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Geospatial Queries</h1>
          <p className="text-gray-400 mt-1">Spatial database with R-Tree indexing and routing</p>
        </div>
        <div className="flex items-center space-x-3">
          <Button variant="secondary">Import GeoJSON</Button>
          <Button variant="primary">Export Layer</Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Total Features</p>
              <p className="text-2xl font-bold text-white">{features.length}</p>
            </div>
            <div className="text-3xl">📍</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Active Layers</p>
              <p className="text-2xl font-bold text-white">{layers.filter(l => l.visible).length}</p>
            </div>
            <div className="text-3xl">🗺️</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Spatial Indexes</p>
              <p className="text-2xl font-bold text-white">{indexes.length}</p>
            </div>
            <div className="text-3xl">🌐</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Query Results</p>
              <p className="text-2xl font-bold text-white">{queryResults.length}</p>
            </div>
            <div className="text-3xl">🔍</div>
          </div>
        </Card>
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Map View */}
        <div className="lg:col-span-2">
          <Card>
            <CardHeader
              title="Interactive Map"
              action={
                <div className="flex items-center space-x-2">
                  <Button
                    variant={drawingMode === 'select' ? 'primary' : 'ghost'}
                    size="sm"
                    onClick={() => setDrawingMode('select')}
                  >
                    Select
                  </Button>
                  <Button
                    variant={drawingMode === 'bbox' ? 'primary' : 'ghost'}
                    size="sm"
                    onClick={() => setDrawingMode('bbox')}
                  >
                    Box
                  </Button>
                  <Button
                    variant={drawingMode === 'radius' ? 'primary' : 'ghost'}
                    size="sm"
                    onClick={() => setDrawingMode('radius')}
                  >
                    Radius
                  </Button>
                </div>
              }
            />
            <div className="bg-gray-900 rounded-lg overflow-hidden">
              <canvas
                ref={canvasRef}
                width={800}
                height={600}
                onMouseDown={handleCanvasMouseDown}
                onMouseMove={handleCanvasMouseMove}
                onMouseUp={handleCanvasMouseUp}
                className="w-full cursor-crosshair"
                style={{ maxHeight: '600px' }}
              />
            </div>
            <div className="mt-4 flex items-center justify-between">
              <div className="flex items-center space-x-4 text-sm text-gray-400">
                {layers.filter(l => l.visible).map(layer => (
                  <div key={layer.id} className="flex items-center space-x-2">
                    <div
                      className="w-4 h-4 rounded"
                      style={{ backgroundColor: layer.color }}
                    ></div>
                    <span>{layer.name}</span>
                  </div>
                ))}
              </div>
              <div className="text-sm text-gray-400">
                Mode: <span className="text-white">{drawingMode}</span>
              </div>
            </div>
          </Card>

          {/* Query Results */}
          {queryResults.length > 0 && (
            <Card className="mt-6">
              <CardHeader title={`Query Results (${queryResults.length})`} />
              <Table
                columns={featureColumns}
                data={queryResults}
                onRowClick={(feature) => setSelectedFeature(feature)}
                compact
              />
            </Card>
          )}
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          <Tabs value={activeTab} onChange={setActiveTab}>
            <TabList tabs={tabs} variant="pills" />
            <TabPanels className="mt-4">
              <TabPanel tabId="map">
                <Card>
                  <CardHeader title="Map Controls" />
                  <div className="space-y-3">
                    <Button variant="secondary" fullWidth>
                      Zoom In
                    </Button>
                    <Button variant="secondary" fullWidth>
                      Zoom Out
                    </Button>
                    <Button variant="secondary" fullWidth>
                      Reset View
                    </Button>
                    <Button variant="ghost" fullWidth>
                      Clear Selection
                    </Button>
                  </div>
                </Card>

                {selectedFeature && (
                  <Card className="mt-4">
                    <CardHeader title="Feature Details" />
                    <div className="space-y-2 text-sm">
                      <div>
                        <span className="text-gray-400">Name:</span>
                        <span className="ml-2 text-white">{selectedFeature.name}</span>
                      </div>
                      <div>
                        <span className="text-gray-400">Type:</span>
                        <Badge variant="primary" size="sm" className="ml-2">
                          {selectedFeature.type}
                        </Badge>
                      </div>
                      <div>
                        <span className="text-gray-400">Layer:</span>
                        <span className="ml-2 text-white">{selectedFeature.layer}</span>
                      </div>
                      <div>
                        <p className="text-gray-400 mb-1">Properties:</p>
                        <div className="bg-gray-900 rounded p-2">
                          <pre className="text-xs text-gray-300">
                            {JSON.stringify(selectedFeature.properties, null, 2)}
                          </pre>
                        </div>
                      </div>
                    </div>
                  </Card>
                )}
              </TabPanel>

              <TabPanel tabId="query">
                <Card>
                  <CardHeader title="Query Builder" />
                  <div className="space-y-4">
                    <Select
                      label="Query Type"
                      value={queryType}
                      onChange={(e) => setQueryType(e.target.value)}
                      options={QUERY_TYPES}
                      fullWidth
                    />
                    <Textarea
                      label="Parameters (JSON)"
                      value={queryParams}
                      onChange={(e) => setQueryParams(e.target.value)}
                      rows={6}
                      fullWidth
                    />
                    <Button
                      variant="primary"
                      fullWidth
                      onClick={executeQuery}
                      loading={isQuerying}
                    >
                      Execute Query
                    </Button>
                    <div className="p-3 bg-gray-900 rounded-lg text-xs text-gray-400">
                      <p className="font-semibold mb-1">Tips:</p>
                      <ul className="list-disc list-inside space-y-1">
                        <li>Use drawing tools on map</li>
                        <li>Spatial queries use R-Tree index</li>
                        <li>All coordinates are in map units</li>
                      </ul>
                    </div>
                  </div>
                </Card>
              </TabPanel>

              <TabPanel tabId="layers">
                <Card>
                  <CardHeader title="Layer Management" />
                  <div className="space-y-2">
                    {layers.map(layer => (
                      <div
                        key={layer.id}
                        className="flex items-center justify-between p-3 bg-gray-800 rounded-lg"
                      >
                        <div className="flex items-center space-x-3">
                          <div
                            className="w-4 h-4 rounded"
                            style={{ backgroundColor: layer.color }}
                          ></div>
                          <div>
                            <p className="text-sm text-white font-medium">{layer.name}</p>
                            <p className="text-xs text-gray-400">
                              {layer.featureCount} features
                            </p>
                          </div>
                        </div>
                        <Button
                          variant={layer.visible ? 'primary' : 'ghost'}
                          size="sm"
                          onClick={() => toggleLayer(layer.id)}
                        >
                          {layer.visible ? 'Hide' : 'Show'}
                        </Button>
                      </div>
                    ))}
                  </div>
                  <Button variant="primary" fullWidth className="mt-4">
                    Add New Layer
                  </Button>
                </Card>
              </TabPanel>

              <TabPanel tabId="indexes">
                <Card>
                  <CardHeader title="Spatial Indexes" />
                  <Table
                    columns={indexColumns}
                    data={indexes}
                    compact
                    emptyMessage="No spatial indexes"
                  />
                  <Button variant="primary" fullWidth className="mt-4">
                    Create Index
                  </Button>
                </Card>
              </TabPanel>

              <TabPanel tabId="routing">
                <Card>
                  <CardHeader title="Route Planner" />
                  <div className="space-y-4">
                    <Select
                      label="Start Point"
                      value={routeStart}
                      onChange={(e) => setRouteStart(e.target.value)}
                      options={features
                        .filter(f => f.type === 'Point')
                        .map(f => ({ value: f.id, label: f.name }))}
                      fullWidth
                      placeholder="Select start"
                    />
                    <Select
                      label="End Point"
                      value={routeEnd}
                      onChange={(e) => setRouteEnd(e.target.value)}
                      options={features
                        .filter(f => f.type === 'Point')
                        .map(f => ({ value: f.id, label: f.name }))}
                      fullWidth
                      placeholder="Select end"
                    />
                    <Button
                      variant="primary"
                      fullWidth
                      onClick={calculateRoute}
                      disabled={!routeStart || !routeEnd}
                    >
                      Calculate Route
                    </Button>
                    {routePath && (
                      <div className="p-3 bg-gray-900 rounded-lg">
                        <p className="text-sm text-white mb-2">Route Details:</p>
                        <div className="space-y-1 text-xs text-gray-400">
                          <p>Distance: ~350m (estimated)</p>
                          <p>Points: {routePath.length}</p>
                          <p>Algorithm: Dijkstra</p>
                        </div>
                      </div>
                    )}
                  </div>
                </Card>
              </TabPanel>
            </TabPanels>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
