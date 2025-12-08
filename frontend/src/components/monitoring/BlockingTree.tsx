import { motion } from 'framer-motion';
import type { BlockingTree as BlockingTreeType, BlockingTreeNode } from '../../services/monitoringService';

interface BlockingTreeProps {
  data: BlockingTreeType;
  onKillSession?: (sessionId: string) => void;
  className?: string;
}

interface TreeNodeProps {
  node: BlockingTreeNode;
  allNodes: BlockingTreeNode[];
  depth: number;
  onKillSession?: (sessionId: string) => void;
}

function TreeNode({ node, allNodes, depth, onKillSession }: TreeNodeProps) {
  const blockedSessions = allNodes.filter((n) => n.blockedBy === node.sessionId);

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  const getStateColor = (state: string) => {
    switch (state) {
      case 'active':
        return 'bg-green-500';
      case 'idle':
        return 'bg-gray-500';
      case 'idle_in_transaction':
        return 'bg-yellow-500';
      default:
        return 'bg-blue-500';
    }
  };

  const getLockTypeColor = (lockType: string) => {
    if (lockType.includes('Exclusive')) return 'text-red-400';
    if (lockType.includes('Share')) return 'text-yellow-400';
    return 'text-blue-400';
  };

  return (
    <motion.div
      initial={{ opacity: 0, x: -20 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ delay: depth * 0.1 }}
      className="relative"
      style={{ marginLeft: depth * 24 }}
    >
      {/* Connection line to parent */}
      {depth > 0 && (
        <div className="absolute left-0 top-0 w-6 h-6 border-l-2 border-b-2 border-gray-600" />
      )}

      <div className="bg-gray-800 border-l-4 border-red-500 rounded-lg p-4 mb-3 ml-6">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center space-x-3 mb-2">
              <span className={`w-2 h-2 rounded-full ${getStateColor(node.state)}`} />
              <span className="text-sm font-mono text-gray-300">
                Session: {node.sessionId.substring(0, 8)}...
              </span>
              <span className={`text-xs px-2 py-1 rounded ${getLockTypeColor(node.lockType)} bg-opacity-20`}>
                {node.lockType}
              </span>
              <span className="text-xs text-gray-500">
                State: {node.state}
              </span>
            </div>

            {node.query && (
              <div className="mb-2">
                <code className="text-xs text-gray-400 block truncate">
                  {node.query.substring(0, 100)}
                  {node.query.length > 100 && '...'}
                </code>
              </div>
            )}

            <div className="flex items-center space-x-4 text-xs text-gray-500">
              <span>Wait Time: <span className="text-orange-400 font-medium">{formatDuration(node.waitTime)}</span></span>
              {node.blocking.length > 0 && (
                <span>Blocking: <span className="text-red-400 font-medium">{node.blocking.length} session(s)</span></span>
              )}
              {node.blockedBy && (
                <span>Blocked by: <span className="text-yellow-400">{node.blockedBy.substring(0, 8)}...</span></span>
              )}
            </div>
          </div>

          {onKillSession && (
            <button
              onClick={() => onKillSession(node.sessionId)}
              className="ml-4 px-3 py-1 text-xs font-medium text-red-400 hover:text-red-300 hover:bg-red-900 hover:bg-opacity-20 rounded transition-colors"
            >
              Kill
            </button>
          )}
        </div>
      </div>

      {/* Render blocked sessions recursively */}
      {blockedSessions.length > 0 && (
        <div className="relative">
          {blockedSessions.map((blockedNode) => (
            <TreeNode
              key={blockedNode.sessionId}
              node={blockedNode}
              allNodes={allNodes}
              depth={depth + 1}
              onKillSession={onKillSession}
            />
          ))}
        </div>
      )}
    </motion.div>
  );
}

export function BlockingTree({ data, onKillSession, className = '' }: BlockingTreeProps) {
  // Find root nodes (not blocked by anyone)
  const rootNodes = data.nodes.filter((node) => !node.blockedBy);

  const totalBlocking = data.nodes.filter((n) => n.blocking.length > 0).length;
  const totalBlocked = data.nodes.filter((n) => n.blockedBy).length;
  const maxWaitTime = Math.max(...data.nodes.map((n) => n.waitTime), 0);

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  if (data.nodes.length === 0) {
    return (
      <div className={`bg-gray-800 rounded-lg p-8 text-center ${className}`}>
        <div className="text-green-400 text-5xl mb-4">✓</div>
        <h3 className="text-lg font-medium text-gray-300 mb-2">No Blocking Detected</h3>
        <p className="text-sm text-gray-500">All sessions are running smoothly</p>
      </div>
    );
  }

  return (
    <div className={className}>
      {/* Summary Stats */}
      <div className="mb-6 grid grid-cols-1 md:grid-cols-3 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-gray-800 rounded-lg p-4"
        >
          <div className="text-3xl font-bold text-red-400 mb-1">{totalBlocking}</div>
          <div className="text-sm text-gray-400">Blocking Sessions</div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gray-800 rounded-lg p-4"
        >
          <div className="text-3xl font-bold text-yellow-400 mb-1">{totalBlocked}</div>
          <div className="text-sm text-gray-400">Blocked Sessions</div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-gray-800 rounded-lg p-4"
        >
          <div className="text-3xl font-bold text-orange-400 mb-1">
            {formatDuration(maxWaitTime)}
          </div>
          <div className="text-sm text-gray-400">Max Wait Time</div>
        </motion.div>
      </div>

      {/* Legend */}
      <div className="mb-4 p-4 bg-gray-800 rounded-lg">
        <h3 className="text-sm font-medium text-gray-300 mb-3">Legend</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
          <div className="flex items-center space-x-2">
            <span className="w-3 h-3 rounded-full bg-green-500" />
            <span className="text-gray-400">Active</span>
          </div>
          <div className="flex items-center space-x-2">
            <span className="w-3 h-3 rounded-full bg-gray-500" />
            <span className="text-gray-400">Idle</span>
          </div>
          <div className="flex items-center space-x-2">
            <span className="w-3 h-3 rounded-full bg-yellow-500" />
            <span className="text-gray-400">In Transaction</span>
          </div>
          <div className="flex items-center space-x-2">
            <div className="w-3 h-1 bg-red-500" />
            <span className="text-gray-400">Blocking Session</span>
          </div>
        </div>
      </div>

      {/* Blocking Tree */}
      <div className="bg-gray-900 rounded-lg p-6">
        <h3 className="text-lg font-semibold text-gray-200 mb-4">Blocking Chain</h3>
        <div className="space-y-2">
          {rootNodes.map((node) => (
            <TreeNode
              key={node.sessionId}
              node={node}
              allNodes={data.nodes}
              depth={0}
              onKillSession={onKillSession}
            />
          ))}
        </div>
      </div>

      {/* Help Text */}
      <div className="mt-4 p-3 bg-blue-900 bg-opacity-20 border border-blue-500 border-opacity-30 rounded-lg">
        <p className="text-xs text-blue-300">
          <strong>Tip:</strong> Sessions at the top of each chain are blocking other sessions below them.
          Consider killing the root blocker to unblock all dependent sessions.
        </p>
      </div>
    </div>
  );
}
