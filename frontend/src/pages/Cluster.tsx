// ============================================================================
// Cluster Management Page
// Overview of cluster topology, nodes, and health
// ============================================================================

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  ServerIcon,
  PlusIcon,
  ArrowPathIcon,
  ChartBarIcon,
  ExclamationTriangleIcon,
  CheckCircleIcon,
} from '@heroicons/react/24/outline';
import { ClusterTopology } from '../components/cluster/ClusterTopology';
import { NodeCard } from '../components/cluster/NodeCard';
import { NodeList } from '../components/cluster/NodeList';
import { AddNodeWizard } from '../components/cluster/AddNodeWizard';
import {
  useClusterTopology,
  useNodes,
  useClusterHealth,
  useAddNode,
  useRemoveNode,
  usePromoteNode,
  useDemoteNode,
  useResyncNode,
} from '../hooks/useCluster';
import type { ClusterNode } from '../types';
import clsx from 'clsx';

type ViewMode = 'topology' | 'cards' | 'table';

export default function Cluster() {
  const [viewMode, setViewMode] = useState<ViewMode>('topology');
  const [showAddNodeWizard, setShowAddNodeWizard] = useState(false);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  // Data hooks
  const { data: topology, isLoading: topologyLoading } = useClusterTopology();
  const { data: nodes = [], isLoading: nodesLoading } = useNodes();
  const { data: health, isLoading: healthLoading } = useClusterHealth();

  // Mutation hooks
  const addNodeMutation = useAddNode();
  const removeNodeMutation = useRemoveNode();
  const promoteNodeMutation = usePromoteNode();
  const demoteNodeMutation = useDemoteNode();
  const resyncNodeMutation = useResyncNode();

  const isLoading = topologyLoading || nodesLoading || healthLoading;

  // Get leader node
  const leaderNode = nodes.find((node) => node.role === 'leader');

  // Calculate stats
  const stats = {
    totalNodes: nodes.length,
    healthyNodes: nodes.filter((n) => n.status === 'healthy').length,
    followers: nodes.filter((n) => n.role === 'follower').length,
    observers: nodes.filter((n) => n.role === 'observer').length,
  };

  function handleNodeClick(node: ClusterNode) {
    setSelectedNodeId(node.id);
  }

  function handlePromote(nodeId: string) {
    if (confirm('Are you sure you want to promote this node to leader?')) {
      promoteNodeMutation.mutate(nodeId);
    }
  }

  function handleDemote(nodeId: string) {
    if (confirm('Are you sure you want to demote the current leader?')) {
      demoteNodeMutation.mutate(nodeId);
    }
  }

  function handleRemove(nodeId: string) {
    if (
      confirm(
        'Are you sure you want to remove this node? This action cannot be undone.'
      )
    ) {
      removeNodeMutation.mutate({ nodeId });
    }
  }

  function handleResync(nodeId: string) {
    if (confirm('Are you sure you want to resync this node?')) {
      resyncNodeMutation.mutate(nodeId);
    }
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-900 flex items-center space-x-3">
              <ServerIcon className="w-8 h-8 text-blue-600" />
              <span>Cluster Management</span>
            </h1>
            <p className="text-gray-600 mt-1">
              Manage cluster topology, nodes, and replication
            </p>
          </div>

          <div className="flex items-center space-x-3">
            <button
              onClick={() => setShowAddNodeWizard(true)}
              className="flex items-center space-x-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              <PlusIcon className="w-5 h-5" />
              <span>Add Node</span>
            </button>
          </div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-4 gap-4">
          <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-gray-600">Total Nodes</div>
                <div className="text-2xl font-bold text-gray-900 mt-1">
                  {stats.totalNodes}
                </div>
              </div>
              <ServerIcon className="w-8 h-8 text-gray-400" />
            </div>
          </div>

          <div className="bg-green-50 rounded-lg p-4 border border-green-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-green-700">Healthy</div>
                <div className="text-2xl font-bold text-green-900 mt-1">
                  {stats.healthyNodes}
                </div>
              </div>
              <CheckCircleIcon className="w-8 h-8 text-green-500" />
            </div>
          </div>

          <div className="bg-blue-50 rounded-lg p-4 border border-blue-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-blue-700">Followers</div>
                <div className="text-2xl font-bold text-blue-900 mt-1">
                  {stats.followers}
                </div>
              </div>
              <ArrowPathIcon className="w-8 h-8 text-blue-500" />
            </div>
          </div>

          <div className="bg-gray-50 rounded-lg p-4 border border-gray-200">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-gray-600">Observers</div>
                <div className="text-2xl font-bold text-gray-900 mt-1">
                  {stats.observers}
                </div>
              </div>
              <ChartBarIcon className="w-8 h-8 text-gray-400" />
            </div>
          </div>
        </div>

        {/* Health Alert */}
        {health && !health.healthy && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg"
          >
            <div className="flex items-start space-x-3">
              <ExclamationTriangleIcon className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
              <div className="flex-1">
                <div className="text-sm font-medium text-red-900">
                  Cluster Health Warning
                </div>
                <ul className="mt-2 space-y-1">
                  {health.issues.map((issue, idx) => (
                    <li key={idx} className="text-sm text-red-700">
                      • {issue}
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </motion.div>
        )}
      </div>

      {/* View Mode Selector */}
      <div className="bg-white border-b border-gray-200 px-6 py-3">
        <div className="flex space-x-1 bg-gray-100 rounded-lg p-1 w-fit">
          {(['topology', 'cards', 'table'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => setViewMode(mode)}
              className={clsx(
                'px-4 py-2 text-sm font-medium rounded-md transition-colors capitalize',
                viewMode === mode
                  ? 'bg-white text-gray-900 shadow-sm'
                  : 'text-gray-600 hover:text-gray-900'
              )}
            >
              {mode}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <ArrowPathIcon className="w-8 h-8 text-blue-500 animate-spin" />
          </div>
        ) : (
          <motion.div
            key={viewMode}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
          >
            {/* Topology View */}
            {viewMode === 'topology' && topology && (
              <ClusterTopology
                topology={topology}
                onNodeClick={handleNodeClick}
                height={600}
              />
            )}

            {/* Cards View */}
            {viewMode === 'cards' && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                {nodes.map((node) => (
                  <motion.div
                    key={node.id}
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.2 }}
                  >
                    <NodeCard
                      node={node}
                      onPromote={handlePromote}
                      onDemote={handleDemote}
                      onRemove={handleRemove}
                      onResync={handleResync}
                      onViewDetails={handleNodeClick}
                    />
                  </motion.div>
                ))}
              </div>
            )}

            {/* Table View */}
            {viewMode === 'table' && (
              <NodeList
                nodes={nodes}
                onNodeClick={handleNodeClick}
                onPromote={handlePromote}
                onDemote={handleDemote}
                onRemove={handleRemove}
                onResync={handleResync}
              />
            )}
          </motion.div>
        )}
      </div>

      {/* Add Node Wizard */}
      <AddNodeWizard
        isOpen={showAddNodeWizard}
        onClose={() => setShowAddNodeWizard(false)}
        onAddNode={async (config) => {
          await addNodeMutation.mutateAsync(config);
        }}
      />
    </div>
  );
}
