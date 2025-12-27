import { useState, useEffect, useRef } from 'react';
import { Card, CardHeader } from '../components/common/Card';
import { Button } from '../components/common/Button';
import { Input, Textarea } from '../components/common/Input';
import { Select } from '../components/common/Select';
import { Tabs, TabList, TabPanel, TabPanels } from '../components/common/Tabs';
import { Badge } from '../components/common/Badge';

// Graph data types
interface GraphNode {
  id: string;
  label: string;
  type: string;
  properties: Record<string, unknown>;
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
}

interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label: string;
  properties: Record<string, unknown>;
}

interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

// Mock data
const MOCK_GRAPH_DATA: GraphData = {
  nodes: [
    { id: '1', label: 'Alice', type: 'Person', properties: { age: 30, city: 'NYC' } },
    { id: '2', label: 'Bob', type: 'Person', properties: { age: 25, city: 'LA' } },
    { id: '3', label: 'Charlie', type: 'Person', properties: { age: 35, city: 'SF' } },
    { id: '4', label: 'TechCorp', type: 'Company', properties: { industry: 'Software', size: 500 } },
    { id: '5', label: 'DataInc', type: 'Company', properties: { industry: 'Analytics', size: 200 } },
    { id: '6', label: 'JavaScript', type: 'Skill', properties: { level: 'Advanced' } },
    { id: '7', label: 'Python', type: 'Skill', properties: { level: 'Expert' } },
  ],
  edges: [
    { id: 'e1', source: '1', target: '4', label: 'WORKS_AT', properties: { since: 2020 } },
    { id: 'e2', source: '2', target: '5', label: 'WORKS_AT', properties: { since: 2021 } },
    { id: 'e3', source: '3', target: '4', label: 'WORKS_AT', properties: { since: 2019 } },
    { id: 'e4', source: '1', target: '2', label: 'FRIENDS_WITH', properties: { since: 2015 } },
    { id: 'e5', source: '2', target: '3', label: 'FRIENDS_WITH', properties: { since: 2018 } },
    { id: 'e6', source: '1', target: '6', label: 'KNOWS', properties: { years: 8 } },
    { id: 'e7', source: '2', target: '7', label: 'KNOWS', properties: { years: 5 } },
    { id: 'e8', source: '3', target: '6', label: 'KNOWS', properties: { years: 10 } },
  ],
};

const ALGORITHM_OPTIONS = [
  { value: 'shortest_path', label: 'Shortest Path' },
  { value: 'pagerank', label: 'PageRank' },
  { value: 'community_detection', label: 'Community Detection' },
  { value: 'centrality', label: 'Centrality Analysis' },
  { value: 'connected_components', label: 'Connected Components' },
];

const NODE_TYPES = ['Person', 'Company', 'Skill', 'Location'];
const EDGE_TYPES = ['WORKS_AT', 'FRIENDS_WITH', 'KNOWS', 'LOCATED_IN', 'MANAGES'];

interface QueryResult {
  nodes: GraphNode[];
  edges: GraphEdge[];
  executionTime: number;
}

interface AlgorithmResult {
  path?: string[];
  length?: number;
  edges?: string[];
  scores?: Array<{ id: string; label: string; score: number }>;
  communities?: Array<{ id: number; nodes: string[] }>;
  components?: number;
  largest?: number;
}

export default function GraphDatabase() {
  const [graphData, setGraphData] = useState<GraphData>(MOCK_GRAPH_DATA);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [query, setQuery] = useState('MATCH (n:Person)-[r:WORKS_AT]->(c:Company) RETURN n, r, c LIMIT 100');
  const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
  const [isExecuting, setIsExecuting] = useState(false);
  const [activeTab, setActiveTab] = useState('graph');

  // Node/Edge creation form
  const [newNodeLabel, setNewNodeLabel] = useState('');
  const [newNodeType, setNewNodeType] = useState('Person');
  const [newNodeProps, setNewNodeProps] = useState('{}');
  const [newEdgeSource, setNewEdgeSource] = useState('');
  const [newEdgeTarget, setNewEdgeTarget] = useState('');
  const [newEdgeType, setNewEdgeType] = useState('WORKS_AT');
  const [newEdgeProps, setNewEdgeProps] = useState('{}');

  // Algorithm execution
  const [selectedAlgorithm, setSelectedAlgorithm] = useState('shortest_path');
  const [algorithmParams, setAlgorithmParams] = useState('{"source": "1", "target": "4"}');
  const [algorithmResult, setAlgorithmResult] = useState<AlgorithmResult | null>(null);

  const canvasRef = useRef<HTMLCanvasElement>(null);

  // Simple force-directed graph visualization
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Initialize positions if not set
    const nodes = graphData.nodes.map(node => ({
      ...node,
      x: node.x ?? Math.random() * width,
      y: node.y ?? Math.random() * height,
      vx: node.vx ?? 0,
      vy: node.vy ?? 0,
    }));

    // Simple force simulation
    const simulate = () => {
      // Apply forces
      const centerForce = 0.01;
      const repulsionForce = 1000;
      const linkForce = 0.02;

      // Center force
      nodes.forEach(node => {
        node.vx! += (width / 2 - node.x!) * centerForce;
        node.vy! += (height / 2 - node.y!) * centerForce;
      });

      // Repulsion between nodes
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const dx = nodes[j].x! - nodes[i].x!;
          const dy = nodes[j].y! - nodes[i].y!;
          const distance = Math.sqrt(dx * dx + dy * dy) || 1;
          const force = repulsionForce / (distance * distance);
          const fx = (dx / distance) * force;
          const fy = (dy / distance) * force;
          nodes[i].vx! -= fx;
          nodes[i].vy! -= fy;
          nodes[j].vx! += fx;
          nodes[j].vy! += fy;
        }
      }

      // Link force
      graphData.edges.forEach(edge => {
        const source = nodes.find(n => n.id === edge.source);
        const target = nodes.find(n => n.id === edge.target);
        if (source && target) {
          const dx = target.x! - source.x!;
          const dy = target.y! - source.y!;
          const distance = Math.sqrt(dx * dx + dy * dy) || 1;
          const targetDistance = 100;
          const force = (distance - targetDistance) * linkForce;
          const fx = (dx / distance) * force;
          const fy = (dy / distance) * force;
          source.vx! += fx;
          source.vy! += fy;
          target.vx! -= fx;
          target.vy! -= fy;
        }
      });

      // Update positions
      nodes.forEach(node => {
        node.vx! *= 0.9; // Damping
        node.vy! *= 0.9;
        node.x! += node.vx!;
        node.y! += node.vy!;
        // Keep within bounds
        node.x! = Math.max(30, Math.min(width - 30, node.x!));
        node.y! = Math.max(30, Math.min(height - 30, node.y!));
      });
    };

    // Render function
    const render = () => {
      ctx.clearRect(0, 0, width, height);

      // Draw edges
      ctx.strokeStyle = '#4B5563';
      ctx.lineWidth = 2;
      graphData.edges.forEach(edge => {
        const source = nodes.find(n => n.id === edge.source);
        const target = nodes.find(n => n.id === edge.target);
        if (source && target) {
          ctx.beginPath();
          ctx.moveTo(source.x!, source.y!);
          ctx.lineTo(target.x!, target.y!);
          ctx.stroke();

          // Draw edge label
          ctx.fillStyle = '#9CA3AF';
          ctx.font = '10px sans-serif';
          ctx.textAlign = 'center';
          const midX = (source.x! + target.x!) / 2;
          const midY = (source.y! + target.y!) / 2;
          ctx.fillText(edge.label, midX, midY);
        }
      });

      // Draw nodes
      nodes.forEach(node => {
        const color = node.type === 'Person' ? '#3B82F6' : node.type === 'Company' ? '#10B981' : '#F59E0B';

        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(node.x!, node.y!, 20, 0, 2 * Math.PI);
        ctx.fill();

        if (selectedNode?.id === node.id) {
          ctx.strokeStyle = '#FFFFFF';
          ctx.lineWidth = 3;
          ctx.stroke();
        }

        // Draw label
        ctx.fillStyle = '#FFFFFF';
        ctx.font = 'bold 11px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(node.label, node.x!, node.y! + 35);
      });
    };

    // Animation loop
    let animationId: number;
    const animate = () => {
      simulate();
      render();
      animationId = requestAnimationFrame(animate);
    };
    animate();

    // Cleanup
    return () => cancelAnimationFrame(animationId);
  }, [graphData, selectedNode]);

  // Handle canvas click
  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    // Find clicked node
    const clickedNode = graphData.nodes.find(node => {
      const dx = (node.x ?? 0) - x;
      const dy = (node.y ?? 0) - y;
      return Math.sqrt(dx * dx + dy * dy) < 20;
    });

    setSelectedNode(clickedNode || null);
  };

  // Execute query
  const executeQuery = async () => {
    setIsExecuting(true);
    // Simulate API call
    setTimeout(() => {
      setQueryResult({
        nodes: graphData.nodes.slice(0, 5),
        edges: graphData.edges.slice(0, 3),
        executionTime: 45,
      });
      setIsExecuting(false);
    }, 500);
  };

  // Add node
  const addNode = () => {
    try {
      const props = JSON.parse(newNodeProps);
      const newNode: GraphNode = {
        id: Date.now().toString(),
        label: newNodeLabel,
        type: newNodeType,
        properties: props,
      };
      setGraphData(prev => ({
        ...prev,
        nodes: [...prev.nodes, newNode],
      }));
      setNewNodeLabel('');
      setNewNodeProps('{}');
    } catch (error) {
      alert('Invalid JSON properties');
    }
  };

  // Add edge
  const addEdge = () => {
    try {
      const props = JSON.parse(newEdgeProps);
      const newEdge: GraphEdge = {
        id: `e${Date.now()}`,
        source: newEdgeSource,
        target: newEdgeTarget,
        label: newEdgeType,
        properties: props,
      };
      setGraphData(prev => ({
        ...prev,
        edges: [...prev.edges, newEdge],
      }));
      setNewEdgeSource('');
      setNewEdgeTarget('');
      setNewEdgeProps('{}');
    } catch (error) {
      alert('Invalid JSON properties');
    }
  };

  // Execute algorithm
  const executeAlgorithm = async () => {
    setIsExecuting(true);
    // Simulate algorithm execution
    setTimeout(() => {
      const mockResults: Record<string, AlgorithmResult> = {
        shortest_path: {
          path: ['1', '4'],
          length: 1,
          edges: ['e1'],
        },
        pagerank: {
          scores: graphData.nodes.map(n => ({ id: n.id, label: n.label, score: Math.random() })),
        },
        community_detection: {
          communities: [
            { id: 1, nodes: ['1', '2', '3'] },
            { id: 2, nodes: ['4', '5'] },
            { id: 3, nodes: ['6', '7'] },
          ],
        },
        centrality: {
          scores: graphData.nodes.map(n => ({ id: n.id, label: n.label, centrality: Math.random() })),
        },
        connected_components: {
          components: 1,
          largest: graphData.nodes.length,
        },
      };
      setAlgorithmResult(mockResults[selectedAlgorithm]);
      setIsExecuting(false);
    }, 800);
  };

  const tabs = [
    { id: 'graph', label: 'Graph View' },
    { id: 'query', label: 'PGQL Query' },
    { id: 'create', label: 'Create Elements' },
    { id: 'algorithms', label: 'Algorithms' },
    { id: 'properties', label: 'Properties' },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Graph Database</h1>
          <p className="text-gray-400 mt-1">Property graph database with PGQL query support</p>
        </div>
        <div className="flex items-center space-x-3">
          <Button variant="secondary">Import Graph</Button>
          <Button variant="primary">Export Graph</Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Total Nodes</p>
              <p className="text-2xl font-bold text-white">{graphData.nodes.length}</p>
            </div>
            <div className="text-3xl">🔵</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Total Edges</p>
              <p className="text-2xl font-bold text-white">{graphData.edges.length}</p>
            </div>
            <div className="text-3xl">↔️</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Node Types</p>
              <p className="text-2xl font-bold text-white">{new Set(graphData.nodes.map(n => n.type)).size}</p>
            </div>
            <div className="text-3xl">📊</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Edge Types</p>
              <p className="text-2xl font-bold text-white">{new Set(graphData.edges.map(e => e.label)).size}</p>
            </div>
            <div className="text-3xl">🔗</div>
          </div>
        </Card>
      </div>

      {/* Main Content */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Graph Visualization */}
        <div className="lg:col-span-2">
          <Card>
            <CardHeader title="Graph Visualization" />
            <div className="bg-gray-900 rounded-lg overflow-hidden">
              <canvas
                ref={canvasRef}
                width={800}
                height={600}
                onClick={handleCanvasClick}
                className="w-full cursor-pointer"
                style={{ maxHeight: '600px' }}
              />
            </div>
            <div className="mt-4 flex items-center space-x-2 text-sm text-gray-400">
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 rounded-full bg-blue-500"></div>
                <span>Person</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 rounded-full bg-green-500"></div>
                <span>Company</span>
              </div>
              <div className="flex items-center space-x-2">
                <div className="w-4 h-4 rounded-full bg-yellow-500"></div>
                <span>Skill</span>
              </div>
            </div>
          </Card>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          <Tabs value={activeTab} onChange={setActiveTab}>
            <TabList tabs={tabs} variant="pills" />
            <TabPanels className="mt-4">
              <TabPanel tabId="graph">
                <Card>
                  <CardHeader title="Graph Info" />
                  <div className="space-y-3 text-sm">
                    <div>
                      <span className="text-gray-400">Algorithm Complexity:</span>
                      <span className="ml-2 text-white">O(V + E)</span>
                    </div>
                    <div>
                      <span className="text-gray-400">Storage Engine:</span>
                      <span className="ml-2 text-white">Native Graph Store</span>
                    </div>
                    <div>
                      <span className="text-gray-400">Index Type:</span>
                      <span className="ml-2 text-white">Adjacency List</span>
                    </div>
                  </div>
                </Card>
              </TabPanel>

              <TabPanel tabId="query">
                <Card>
                  <CardHeader title="PGQL Query Editor" />
                  <div className="space-y-4">
                    <Textarea
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      rows={6}
                      fullWidth
                      placeholder="Enter PGQL query..."
                    />
                    <Button
                      variant="primary"
                      fullWidth
                      onClick={executeQuery}
                      loading={isExecuting}
                    >
                      Execute Query
                    </Button>
                    {queryResult && (
                      <div className="mt-4 p-3 bg-gray-900 rounded-lg">
                        <p className="text-xs text-gray-400 mb-2">
                          Execution time: {queryResult.executionTime}ms
                        </p>
                        <p className="text-sm text-white">
                          Returned {queryResult.nodes.length} nodes and {queryResult.edges.length} edges
                        </p>
                      </div>
                    )}
                  </div>
                </Card>
              </TabPanel>

              <TabPanel tabId="create">
                <Card>
                  <CardHeader title="Create Node" />
                  <div className="space-y-3">
                    <Input
                      label="Label"
                      value={newNodeLabel}
                      onChange={(e) => setNewNodeLabel(e.target.value)}
                      fullWidth
                    />
                    <Select
                      label="Type"
                      value={newNodeType}
                      onChange={(e) => setNewNodeType(e.target.value)}
                      options={NODE_TYPES.map(t => ({ value: t, label: t }))}
                      fullWidth
                    />
                    <Textarea
                      label="Properties (JSON)"
                      value={newNodeProps}
                      onChange={(e) => setNewNodeProps(e.target.value)}
                      rows={3}
                      fullWidth
                    />
                    <Button variant="primary" fullWidth onClick={addNode}>
                      Add Node
                    </Button>
                  </div>
                </Card>

                <Card className="mt-4">
                  <CardHeader title="Create Edge" />
                  <div className="space-y-3">
                    <Select
                      label="Source Node"
                      value={newEdgeSource}
                      onChange={(e) => setNewEdgeSource(e.target.value)}
                      options={graphData.nodes.map(n => ({ value: n.id, label: n.label }))}
                      fullWidth
                      placeholder="Select source"
                    />
                    <Select
                      label="Target Node"
                      value={newEdgeTarget}
                      onChange={(e) => setNewEdgeTarget(e.target.value)}
                      options={graphData.nodes.map(n => ({ value: n.id, label: n.label }))}
                      fullWidth
                      placeholder="Select target"
                    />
                    <Select
                      label="Type"
                      value={newEdgeType}
                      onChange={(e) => setNewEdgeType(e.target.value)}
                      options={EDGE_TYPES.map(t => ({ value: t, label: t }))}
                      fullWidth
                    />
                    <Textarea
                      label="Properties (JSON)"
                      value={newEdgeProps}
                      onChange={(e) => setNewEdgeProps(e.target.value)}
                      rows={2}
                      fullWidth
                    />
                    <Button variant="primary" fullWidth onClick={addEdge}>
                      Add Edge
                    </Button>
                  </div>
                </Card>
              </TabPanel>

              <TabPanel tabId="algorithms">
                <Card>
                  <CardHeader title="Graph Algorithms" />
                  <div className="space-y-4">
                    <Select
                      label="Algorithm"
                      value={selectedAlgorithm}
                      onChange={(e) => setSelectedAlgorithm(e.target.value)}
                      options={ALGORITHM_OPTIONS}
                      fullWidth
                    />
                    <Textarea
                      label="Parameters (JSON)"
                      value={algorithmParams}
                      onChange={(e) => setAlgorithmParams(e.target.value)}
                      rows={3}
                      fullWidth
                    />
                    <Button
                      variant="primary"
                      fullWidth
                      onClick={executeAlgorithm}
                      loading={isExecuting}
                    >
                      Execute Algorithm
                    </Button>
                    {algorithmResult && (
                      <div className="mt-4 p-3 bg-gray-900 rounded-lg">
                        <p className="text-sm text-white font-semibold mb-2">Results:</p>
                        <pre className="text-xs text-gray-300 overflow-auto">
                          {JSON.stringify(algorithmResult, null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                </Card>
              </TabPanel>

              <TabPanel tabId="properties">
                <Card>
                  <CardHeader title={selectedNode ? 'Node Properties' : 'Edge Properties'} />
                  {selectedNode ? (
                    <div className="space-y-3">
                      <div>
                        <span className="text-sm text-gray-400">ID:</span>
                        <span className="ml-2 text-white">{selectedNode.id}</span>
                      </div>
                      <div>
                        <span className="text-sm text-gray-400">Label:</span>
                        <span className="ml-2 text-white">{selectedNode.label}</span>
                      </div>
                      <div>
                        <span className="text-sm text-gray-400">Type:</span>
                        <Badge variant="primary" size="sm" className="ml-2">{selectedNode.type}</Badge>
                      </div>
                      <div>
                        <p className="text-sm text-gray-400 mb-2">Properties:</p>
                        <div className="bg-gray-900 rounded p-3">
                          <pre className="text-xs text-gray-300">
                            {JSON.stringify(selectedNode.properties, null, 2)}
                          </pre>
                        </div>
                      </div>
                    </div>
                  ) : (
                    <p className="text-sm text-gray-400">Click on a node to view its properties</p>
                  )}
                </Card>
              </TabPanel>
            </TabPanels>
          </Tabs>
        </div>
      </div>
    </div>
  );
}
