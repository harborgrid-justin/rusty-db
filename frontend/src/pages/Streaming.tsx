import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import { LineChart, Line, AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { Card, CardHeader } from '../components/common/Card';
import { Button } from '../components/common/Button';
import { Input, Textarea } from '../components/common/Input';
import { Select } from '../components/common/Select';
import { Tabs, TabList, TabPanel, TabPanels } from '../components/common/Tabs';
import { Table, Column } from '../components/common/Table';
import { Badge, StatusBadge } from '../components/common/Badge';

// CDC and Streaming types
interface CDCSubscription {
  id: string;
  name: string;
  tables: string[];
  operations: ('INSERT' | 'UPDATE' | 'DELETE')[];
  status: 'active' | 'paused' | 'error';
  createdAt: string;
  consumerGroup: string;
  lag: number;
  messagesPerSecond: number;
}

interface ChangeEvent {
  id: string;
  subscriptionId: string;
  table: string;
  operation: 'INSERT' | 'UPDATE' | 'DELETE';
  data: Record<string, any>;
  oldData?: Record<string, any>;
  timestamp: string;
  lsn: string;
}

interface ConsumerGroup {
  id: string;
  name: string;
  members: number;
  lag: number;
  offset: number;
  status: 'active' | 'rebalancing' | 'inactive';
}

interface StreamMetrics {
  timestamp: string;
  eventsPerSecond: number;
  bytesPerSecond: number;
  lag: number;
  consumers: number;
}

// Mock data
const MOCK_SUBSCRIPTIONS: CDCSubscription[] = [
  {
    id: 's1',
    name: 'Orders CDC Stream',
    tables: ['orders', 'order_items'],
    operations: ['INSERT', 'UPDATE', 'DELETE'],
    status: 'active',
    createdAt: '2025-03-01',
    consumerGroup: 'analytics-group',
    lag: 125,
    messagesPerSecond: 342,
  },
  {
    id: 's2',
    name: 'User Activity Stream',
    tables: ['users', 'user_sessions'],
    operations: ['INSERT', 'UPDATE'],
    status: 'active',
    createdAt: '2025-03-05',
    consumerGroup: 'ml-group',
    lag: 45,
    messagesPerSecond: 567,
  },
  {
    id: 's3',
    name: 'Inventory Updates',
    tables: ['inventory'],
    operations: ['UPDATE'],
    status: 'paused',
    createdAt: '2025-02-15',
    consumerGroup: 'warehouse-group',
    lag: 0,
    messagesPerSecond: 0,
  },
];

const MOCK_RECENT_EVENTS: ChangeEvent[] = [
  {
    id: 'e1',
    subscriptionId: 's1',
    table: 'orders',
    operation: 'INSERT',
    data: { id: 1234, user_id: 567, total: 99.99, status: 'pending' },
    timestamp: new Date().toISOString(),
    lsn: '0/16B8A78',
  },
  {
    id: 'e2',
    subscriptionId: 's1',
    table: 'orders',
    operation: 'UPDATE',
    data: { id: 1234, user_id: 567, total: 99.99, status: 'confirmed' },
    oldData: { id: 1234, user_id: 567, total: 99.99, status: 'pending' },
    timestamp: new Date(Date.now() - 5000).toISOString(),
    lsn: '0/16B8A79',
  },
  {
    id: 'e3',
    subscriptionId: 's2',
    table: 'users',
    operation: 'UPDATE',
    data: { id: 567, last_login: new Date().toISOString() },
    oldData: { id: 567, last_login: new Date(Date.now() - 3600000).toISOString() },
    timestamp: new Date(Date.now() - 10000).toISOString(),
    lsn: '0/16B8A7A',
  },
  {
    id: 'e4',
    subscriptionId: 's1',
    table: 'order_items',
    operation: 'INSERT',
    data: { id: 5678, order_id: 1234, product_id: 789, quantity: 2 },
    timestamp: new Date(Date.now() - 15000).toISOString(),
    lsn: '0/16B8A7B',
  },
];

const MOCK_CONSUMER_GROUPS: ConsumerGroup[] = [
  { id: 'cg1', name: 'analytics-group', members: 3, lag: 125, offset: 45678, status: 'active' },
  { id: 'cg2', name: 'ml-group', members: 2, lag: 45, offset: 34567, status: 'active' },
  { id: 'cg3', name: 'warehouse-group', members: 1, lag: 0, offset: 23456, status: 'inactive' },
  { id: 'cg4', name: 'backup-group', members: 4, lag: 350, offset: 56789, status: 'rebalancing' },
];

const MOCK_METRICS: StreamMetrics[] = Array.from({ length: 20 }, (_, i) => ({
  timestamp: new Date(Date.now() - (19 - i) * 5000).toLocaleTimeString(),
  eventsPerSecond: Math.floor(Math.random() * 500 + 300),
  bytesPerSecond: Math.floor(Math.random() * 50000 + 20000),
  lag: Math.floor(Math.random() * 200 + 50),
  consumers: 6,
}));

const TABLE_OPTIONS = [
  { value: 'orders', label: 'orders' },
  { value: 'order_items', label: 'order_items' },
  { value: 'users', label: 'users' },
  { value: 'user_sessions', label: 'user_sessions' },
  { value: 'inventory', label: 'inventory' },
  { value: 'products', label: 'products' },
];

export default function Streaming() {
  const [subscriptions, setSubscriptions] = useState<CDCSubscription[]>(MOCK_SUBSCRIPTIONS);
  const [recentEvents, setRecentEvents] = useState<ChangeEvent[]>(MOCK_RECENT_EVENTS);
  const [consumerGroups, setConsumerGroups] = useState<ConsumerGroup[]>(MOCK_CONSUMER_GROUPS);
  const [metrics, setMetrics] = useState<StreamMetrics[]>(MOCK_METRICS);
  const [selectedSubscription, setSelectedSubscription] = useState<CDCSubscription | null>(
    MOCK_SUBSCRIPTIONS[0]
  );
  const [activeTab, setActiveTab] = useState('subscriptions');

  // New subscription form
  const [newSubName, setNewSubName] = useState('');
  const [selectedTables, setSelectedTables] = useState<string[]>([]);
  const [selectedOps, setSelectedOps] = useState<string[]>(['INSERT', 'UPDATE', 'DELETE']);
  const [filterCondition, setFilterCondition] = useState('');
  const [consumerGroupName, setConsumerGroupName] = useState('');
  const [isCreating, setIsCreating] = useState(false);

  // Live feed control
  const [isLiveFeed, setIsLiveFeed] = useState(true);
  const eventFeedRef = useRef<HTMLDivElement>(null);

  // Simulate live events
  useEffect(() => {
    if (!isLiveFeed) return;

    const interval = setInterval(() => {
      const newEvent: ChangeEvent = {
        id: `e${Date.now()}`,
        subscriptionId: subscriptions[Math.floor(Math.random() * subscriptions.length)]?.id || 's1',
        table: ['orders', 'users', 'inventory'][Math.floor(Math.random() * 3)],
        operation: ['INSERT', 'UPDATE', 'DELETE'][Math.floor(Math.random() * 3)] as any,
        data: { id: Math.floor(Math.random() * 10000), timestamp: new Date().toISOString() },
        timestamp: new Date().toISOString(),
        lsn: `0/${Math.random().toString(16).substr(2, 8).toUpperCase()}`,
      };

      setRecentEvents(prev => [newEvent, ...prev.slice(0, 99)]);

      // Update metrics
      setMetrics(prev => [
        ...prev.slice(1),
        {
          timestamp: new Date().toLocaleTimeString(),
          eventsPerSecond: Math.floor(Math.random() * 500 + 300),
          bytesPerSecond: Math.floor(Math.random() * 50000 + 20000),
          lag: Math.floor(Math.random() * 200 + 50),
          consumers: 6,
        },
      ]);
    }, 2000);

    return () => clearInterval(interval);
  }, [isLiveFeed, subscriptions]);

  // Auto-scroll event feed
  useEffect(() => {
    if (isLiveFeed && eventFeedRef.current) {
      eventFeedRef.current.scrollTop = 0;
    }
  }, [recentEvents, isLiveFeed]);

  // Create subscription
  const createSubscription = async () => {
    setIsCreating(true);
    setTimeout(() => {
      const newSub: CDCSubscription = {
        id: `s${Date.now()}`,
        name: newSubName,
        tables: selectedTables,
        operations: selectedOps as any,
        status: 'active',
        createdAt: new Date().toISOString().split('T')[0],
        consumerGroup: consumerGroupName || 'default-group',
        lag: 0,
        messagesPerSecond: 0,
      };
      setSubscriptions(prev => [...prev, newSub]);
      setIsCreating(false);
      setNewSubName('');
      setSelectedTables([]);
      setFilterCondition('');
      setConsumerGroupName('');
    }, 1000);
  };

  // Toggle subscription status
  const toggleSubscription = (id: string) => {
    setSubscriptions(prev =>
      prev.map(sub =>
        sub.id === id
          ? { ...sub, status: sub.status === 'active' ? 'paused' : 'active' }
          : sub
      )
    );
  };

  const tabs = [
    { id: 'subscriptions', label: 'Subscriptions' },
    { id: 'events', label: 'Live Events', badge: recentEvents.length },
    { id: 'consumers', label: 'Consumer Groups' },
    { id: 'metrics', label: 'Metrics' },
    { id: 'create', label: 'Create Subscription' },
  ];

  const subscriptionColumns: Column<CDCSubscription>[] = [
    { key: 'name', header: 'Name', sortable: true },
    {
      key: 'tables',
      header: 'Tables',
      render: (val: string[]) => (
        <div className="flex flex-wrap gap-1">
          {val.map((table, i) => (
            <Badge key={i} variant="info" size="sm">
              {table}
            </Badge>
          ))}
        </div>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      render: (val) => (
        <StatusBadge
          status={val === 'active' ? 'active' : val === 'paused' ? 'pending' : 'error'}
        />
      ),
    },
    {
      key: 'messagesPerSecond',
      header: 'Msg/s',
      render: (val) => <span className="font-mono">{val}</span>,
    },
    {
      key: 'lag',
      header: 'Lag',
      render: (val) => (
        <Badge variant={val > 200 ? 'danger' : val > 100 ? 'warning' : 'success'} size="sm">
          {val}
        </Badge>
      ),
    },
    {
      key: 'id',
      header: 'Actions',
      render: (_, sub) => (
        <div className="flex items-center space-x-2">
          <Button
            size="sm"
            variant={sub.status === 'active' ? 'warning' : 'success'}
            onClick={() => toggleSubscription(sub.id)}
          >
            {sub.status === 'active' ? 'Pause' : 'Resume'}
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setSelectedSubscription(sub)}>
            View
          </Button>
        </div>
      ),
    },
  ];

  const eventColumns: Column<ChangeEvent>[] = [
    {
      key: 'timestamp',
      header: 'Time',
      width: '150px',
      render: (val) => new Date(val).toLocaleTimeString(),
    },
    { key: 'table', header: 'Table', width: '120px' },
    {
      key: 'operation',
      header: 'Op',
      width: '100px',
      render: (val) => (
        <Badge
          variant={val === 'INSERT' ? 'success' : val === 'UPDATE' ? 'warning' : 'danger'}
          size="sm"
        >
          {val}
        </Badge>
      ),
    },
    { key: 'lsn', header: 'LSN', width: '100px', render: (val) => <code className="text-xs">{val}</code> },
  ];

  const consumerColumns: Column<ConsumerGroup>[] = [
    { key: 'name', header: 'Group Name', sortable: true },
    { key: 'members', header: 'Members' },
    {
      key: 'lag',
      header: 'Lag',
      render: (val) => (
        <Badge variant={val > 200 ? 'danger' : val > 100 ? 'warning' : 'success'} size="sm">
          {val}
        </Badge>
      ),
    },
    { key: 'offset', header: 'Offset', render: (val) => <span className="font-mono">{val}</span> },
    {
      key: 'status',
      header: 'Status',
      render: (val) => (
        <StatusBadge
          status={
            val === 'active' ? 'active' : val === 'rebalancing' ? 'pending' : 'inactive'
          }
        />
      ),
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-white">Change Data Capture & Streaming</h1>
          <p className="text-gray-400 mt-1">Real-time database change streams and event processing</p>
        </div>
        <div className="flex items-center space-x-3">
          <Button
            variant={isLiveFeed ? 'success' : 'secondary'}
            onClick={() => setIsLiveFeed(!isLiveFeed)}
          >
            {isLiveFeed ? '🟢 Live' : '⏸️ Paused'}
          </Button>
          <Button variant="primary">New Subscription</Button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Active Subscriptions</p>
              <p className="text-2xl font-bold text-white">
                {subscriptions.filter(s => s.status === 'active').length}
              </p>
            </div>
            <div className="text-3xl">📡</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Events/sec</p>
              <p className="text-2xl font-bold text-white">
                {metrics[metrics.length - 1]?.eventsPerSecond || 0}
              </p>
            </div>
            <div className="text-3xl">⚡</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Total Lag</p>
              <p className="text-2xl font-bold text-white">
                {subscriptions.reduce((sum, s) => sum + s.lag, 0)}
              </p>
            </div>
            <div className="text-3xl">⏱️</div>
          </div>
        </Card>
        <Card padding="lg">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-400 mb-1">Consumer Groups</p>
              <p className="text-2xl font-bold text-white">{consumerGroups.length}</p>
            </div>
            <div className="text-3xl">👥</div>
          </div>
        </Card>
      </div>

      {/* Main Content */}
      <Tabs value={activeTab} onChange={setActiveTab}>
        <TabList tabs={tabs} />
        <TabPanels className="mt-6">
          {/* Subscriptions */}
          <TabPanel tabId="subscriptions">
            <Card>
              <CardHeader title="CDC Subscriptions" />
              <Table
                columns={subscriptionColumns}
                data={subscriptions}
                onRowClick={setSelectedSubscription}
              />
            </Card>

            {selectedSubscription && (
              <Card className="mt-6">
                <CardHeader title={`${selectedSubscription.name} - Details`} />
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  <div className="space-y-3">
                    <div>
                      <span className="text-sm text-gray-400">Subscription ID:</span>
                      <span className="ml-2 text-white font-mono">{selectedSubscription.id}</span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Tables:</span>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {selectedSubscription.tables.map((table, i) => (
                          <Badge key={i} variant="info">
                            {table}
                          </Badge>
                        ))}
                      </div>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Operations:</span>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {selectedSubscription.operations.map((op, i) => (
                          <Badge
                            key={i}
                            variant={
                              op === 'INSERT' ? 'success' : op === 'UPDATE' ? 'warning' : 'danger'
                            }
                          >
                            {op}
                          </Badge>
                        ))}
                      </div>
                    </div>
                  </div>
                  <div className="space-y-3">
                    <div>
                      <span className="text-sm text-gray-400">Consumer Group:</span>
                      <span className="ml-2 text-white">{selectedSubscription.consumerGroup}</span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Messages/sec:</span>
                      <span className="ml-2 text-white font-bold">
                        {selectedSubscription.messagesPerSecond}
                      </span>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Lag:</span>
                      <Badge
                        variant={
                          selectedSubscription.lag > 200
                            ? 'danger'
                            : selectedSubscription.lag > 100
                            ? 'warning'
                            : 'success'
                        }
                        className="ml-2"
                      >
                        {selectedSubscription.lag} messages
                      </Badge>
                    </div>
                    <div>
                      <span className="text-sm text-gray-400">Created:</span>
                      <span className="ml-2 text-white">{selectedSubscription.createdAt}</span>
                    </div>
                  </div>
                </div>
              </Card>
            )}
          </TabPanel>

          {/* Live Events */}
          <TabPanel tabId="events">
            <Card>
              <CardHeader
                title="Live Event Feed"
                action={
                  <div className="flex items-center space-x-2">
                    <span className="text-sm text-gray-400">{recentEvents.length} events</span>
                    <Button size="sm" variant="ghost" onClick={() => setRecentEvents([])}>
                      Clear
                    </Button>
                  </div>
                }
              />
              <div
                ref={eventFeedRef}
                className="space-y-2 max-h-96 overflow-y-auto"
                style={{ scrollBehavior: 'smooth' }}
              >
                {recentEvents.map(event => (
                  <motion.div
                    key={event.id}
                    initial={{ opacity: 0, x: -20 }}
                    animate={{ opacity: 1, x: 0 }}
                    className="p-3 bg-gray-800 rounded-lg border border-gray-700"
                  >
                    <div className="flex items-start justify-between mb-2">
                      <div className="flex items-center space-x-3">
                        <Badge
                          variant={
                            event.operation === 'INSERT'
                              ? 'success'
                              : event.operation === 'UPDATE'
                              ? 'warning'
                              : 'danger'
                          }
                        >
                          {event.operation}
                        </Badge>
                        <span className="text-white font-medium">{event.table}</span>
                      </div>
                      <span className="text-xs text-gray-400">
                        {new Date(event.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-xs">
                      <div>
                        <span className="text-gray-400">LSN:</span>
                        <code className="ml-2 text-blue-400">{event.lsn}</code>
                      </div>
                      <div>
                        <span className="text-gray-400">Sub:</span>
                        <span className="ml-2 text-gray-300">{event.subscriptionId}</span>
                      </div>
                    </div>
                    <details className="mt-2">
                      <summary className="text-xs text-gray-400 cursor-pointer hover:text-gray-300">
                        View data
                      </summary>
                      <pre className="mt-2 p-2 bg-gray-900 rounded text-xs text-gray-300 overflow-auto">
                        {JSON.stringify(event.data, null, 2)}
                      </pre>
                      {event.oldData && (
                        <pre className="mt-1 p-2 bg-gray-900 rounded text-xs text-gray-300 overflow-auto">
                          Old: {JSON.stringify(event.oldData, null, 2)}
                        </pre>
                      )}
                    </details>
                  </motion.div>
                ))}
              </div>
            </Card>

            <Card className="mt-6">
              <CardHeader title="Event Statistics" />
              <div className="grid grid-cols-3 gap-4">
                <div className="p-4 bg-gray-900 rounded-lg">
                  <p className="text-sm text-gray-400 mb-1">INSERT</p>
                  <p className="text-2xl font-bold text-green-500">
                    {recentEvents.filter(e => e.operation === 'INSERT').length}
                  </p>
                </div>
                <div className="p-4 bg-gray-900 rounded-lg">
                  <p className="text-sm text-gray-400 mb-1">UPDATE</p>
                  <p className="text-2xl font-bold text-yellow-500">
                    {recentEvents.filter(e => e.operation === 'UPDATE').length}
                  </p>
                </div>
                <div className="p-4 bg-gray-900 rounded-lg">
                  <p className="text-sm text-gray-400 mb-1">DELETE</p>
                  <p className="text-2xl font-bold text-red-500">
                    {recentEvents.filter(e => e.operation === 'DELETE').length}
                  </p>
                </div>
              </div>
            </Card>
          </TabPanel>

          {/* Consumer Groups */}
          <TabPanel tabId="consumers">
            <Card>
              <CardHeader title="Consumer Groups" />
              <Table columns={consumerColumns} data={consumerGroups} />
            </Card>

            <Card className="mt-6">
              <CardHeader title="Consumer Group Lag" />
              <ResponsiveContainer width="100%" height={300}>
                <BarChart
                  data={consumerGroups}
                  layout="vertical"
                  margin={{ top: 5, right: 30, left: 20, bottom: 5 }}
                >
                  <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                  <XAxis type="number" stroke="#9CA3AF" />
                  <YAxis dataKey="name" type="category" stroke="#9CA3AF" width={150} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#1F2937',
                      border: '1px solid #374151',
                      borderRadius: '0.5rem',
                    }}
                  />
                  <Bar dataKey="lag" fill="#3B82F6" />
                </BarChart>
              </ResponsiveContainer>
            </Card>
          </TabPanel>

          {/* Metrics */}
          <TabPanel tabId="metrics">
            <Card>
              <CardHeader title="Stream Metrics" />
              <ResponsiveContainer width="100%" height={300}>
                <AreaChart data={metrics}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                  <XAxis dataKey="timestamp" stroke="#9CA3AF" />
                  <YAxis stroke="#9CA3AF" />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#1F2937',
                      border: '1px solid #374151',
                      borderRadius: '0.5rem',
                    }}
                  />
                  <Legend />
                  <Area
                    type="monotone"
                    dataKey="eventsPerSecond"
                    stackId="1"
                    stroke="#3B82F6"
                    fill="#3B82F6"
                    name="Events/sec"
                  />
                </AreaChart>
              </ResponsiveContainer>
            </Card>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
              <Card>
                <CardHeader title="Throughput (Bytes/sec)" />
                <ResponsiveContainer width="100%" height={250}>
                  <LineChart data={metrics}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                    <XAxis dataKey="timestamp" stroke="#9CA3AF" />
                    <YAxis stroke="#9CA3AF" />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#1F2937',
                        border: '1px solid #374151',
                        borderRadius: '0.5rem',
                      }}
                    />
                    <Line
                      type="monotone"
                      dataKey="bytesPerSecond"
                      stroke="#10B981"
                      strokeWidth={2}
                      dot={false}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </Card>

              <Card>
                <CardHeader title="Consumer Lag" />
                <ResponsiveContainer width="100%" height={250}>
                  <LineChart data={metrics}>
                    <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                    <XAxis dataKey="timestamp" stroke="#9CA3AF" />
                    <YAxis stroke="#9CA3AF" />
                    <Tooltip
                      contentStyle={{
                        backgroundColor: '#1F2937',
                        border: '1px solid #374151',
                        borderRadius: '0.5rem',
                      }}
                    />
                    <Line
                      type="monotone"
                      dataKey="lag"
                      stroke="#F59E0B"
                      strokeWidth={2}
                      dot={false}
                    />
                  </LineChart>
                </ResponsiveContainer>
              </Card>
            </div>
          </TabPanel>

          {/* Create Subscription */}
          <TabPanel tabId="create">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <Card>
                <CardHeader title="Create CDC Subscription" />
                <div className="space-y-4">
                  <Input
                    label="Subscription Name"
                    value={newSubName}
                    onChange={(e) => setNewSubName(e.target.value)}
                    fullWidth
                    placeholder="e.g., Orders CDC Stream"
                  />
                  <div>
                    <label className="block text-sm font-medium text-gray-300 mb-2">
                      Tables to Monitor
                    </label>
                    <div className="grid grid-cols-2 gap-2">
                      {TABLE_OPTIONS.map(opt => (
                        <button
                          key={opt.value}
                          onClick={() =>
                            setSelectedTables(prev =>
                              prev.includes(opt.value)
                                ? prev.filter(t => t !== opt.value)
                                : [...prev, opt.value]
                            )
                          }
                          className={`p-2 rounded border text-sm transition-colors ${
                            selectedTables.includes(opt.value)
                              ? 'bg-blue-500 bg-opacity-20 border-blue-500 text-white'
                              : 'bg-gray-800 border-gray-700 text-gray-400 hover:border-gray-600'
                          }`}
                        >
                          {opt.label}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-300 mb-2">
                      Operations to Capture
                    </label>
                    <div className="flex space-x-2">
                      {['INSERT', 'UPDATE', 'DELETE'].map(op => (
                        <button
                          key={op}
                          onClick={() =>
                            setSelectedOps(prev =>
                              prev.includes(op) ? prev.filter(o => o !== op) : [...prev, op]
                            )
                          }
                          className={`px-3 py-2 rounded border text-sm transition-colors ${
                            selectedOps.includes(op)
                              ? 'bg-blue-500 bg-opacity-20 border-blue-500 text-white'
                              : 'bg-gray-800 border-gray-700 text-gray-400 hover:border-gray-600'
                          }`}
                        >
                          {op}
                        </button>
                      ))}
                    </div>
                  </div>
                  <Textarea
                    label="Filter Condition (SQL WHERE clause)"
                    value={filterCondition}
                    onChange={(e) => setFilterCondition(e.target.value)}
                    rows={3}
                    fullWidth
                    placeholder="e.g., status = 'active' AND created_at > NOW() - INTERVAL '1 day'"
                  />
                  <Input
                    label="Consumer Group Name"
                    value={consumerGroupName}
                    onChange={(e) => setConsumerGroupName(e.target.value)}
                    fullWidth
                    placeholder="e.g., analytics-group"
                  />
                  <Button
                    variant="primary"
                    fullWidth
                    onClick={createSubscription}
                    loading={isCreating}
                    disabled={!newSubName || selectedTables.length === 0}
                  >
                    Create Subscription
                  </Button>
                </div>
              </Card>

              <Card>
                <CardHeader title="Subscription Preview" />
                <div className="space-y-4">
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-2">Name:</p>
                    <p className="text-white font-medium">{newSubName || '(not set)'}</p>
                  </div>
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-2">Tables:</p>
                    <div className="flex flex-wrap gap-2">
                      {selectedTables.length > 0 ? (
                        selectedTables.map((table, i) => (
                          <Badge key={i} variant="info">
                            {table}
                          </Badge>
                        ))
                      ) : (
                        <span className="text-gray-500 text-sm">(none selected)</span>
                      )}
                    </div>
                  </div>
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-2">Operations:</p>
                    <div className="flex flex-wrap gap-2">
                      {selectedOps.map((op, i) => (
                        <Badge
                          key={i}
                          variant={
                            op === 'INSERT' ? 'success' : op === 'UPDATE' ? 'warning' : 'danger'
                          }
                        >
                          {op}
                        </Badge>
                      ))}
                    </div>
                  </div>
                  <div className="p-4 bg-gray-900 rounded-lg">
                    <p className="text-sm text-gray-400 mb-2">Consumer Group:</p>
                    <p className="text-white">{consumerGroupName || '(default-group)'}</p>
                  </div>
                  {filterCondition && (
                    <div className="p-4 bg-gray-900 rounded-lg">
                      <p className="text-sm text-gray-400 mb-2">Filter:</p>
                      <code className="text-xs text-blue-400">{filterCondition}</code>
                    </div>
                  )}
                </div>
              </Card>
            </div>
          </TabPanel>
        </TabPanels>
      </Tabs>
    </div>
  );
}
