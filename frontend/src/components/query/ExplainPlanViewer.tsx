import React, { useState, useCallback } from 'react';
import { ExplainPlan, PlanNode } from '../../stores/queryStore';

interface ExplainPlanViewerProps {
  plan: ExplainPlan;
}

interface PlanNodeItemProps {
  node: PlanNode;
  depth: number;
  isLast: boolean;
  parentPrefix: string;
}

const PlanNodeItem: React.FC<PlanNodeItemProps> = ({ node, depth, isLast, parentPrefix }) => {
  const [isExpanded, setIsExpanded] = useState(true);

  const hasChildren = node.children && node.children.length > 0;
  const toggleExpand = useCallback(() => {
    if (hasChildren) {
      setIsExpanded(prev => !prev);
    }
  }, [hasChildren]);

  // Calculate performance metrics
  const rowsAccuracy = node.actualRows !== undefined
    ? Math.abs((node.actualRows - node.rows) / Math.max(node.rows, 1)) * 100
    : null;

  const hasWarning = node.warning || (rowsAccuracy !== null && rowsAccuracy > 50);

  // Build tree prefix
  const connector = isLast ? '└─' : '├─';
  const prefix = depth === 0 ? '' : `${parentPrefix}${connector} `;
  const childPrefix = depth === 0 ? '' : `${parentPrefix}${isLast ? '   ' : '│  '}`;

  return (
    <div className="plan-node">
      <div
        className={`plan-node-header ${hasWarning ? 'warning' : ''}`}
        onClick={toggleExpand}
        style={{
          cursor: hasChildren ? 'pointer' : 'default',
          paddingLeft: `${depth * 20}px`,
          padding: '8px',
          borderLeft: hasWarning ? '3px solid #f48771' : 'none',
          background: hasWarning ? '#3a2626' : 'transparent',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {hasChildren && (
            <span style={{ fontSize: '12px', color: '#888' }}>
              {isExpanded ? '▼' : '▶'}
            </span>
          )}
          {!hasChildren && <span style={{ width: '12px' }} />}

          <span style={{ fontFamily: 'monospace', color: '#666', fontSize: '12px' }}>
            {prefix}
          </span>

          <span style={{ fontWeight: 'bold', color: '#4ec9b0' }}>
            {node.operation}
          </span>

          {node.table && (
            <span style={{ color: '#dcdcaa' }}>
              on {node.table}
            </span>
          )}

          {node.index && (
            <span style={{ color: '#9cdcfe', fontSize: '12px' }}>
              using {node.index}
            </span>
          )}

          {hasWarning && (
            <span style={{ color: '#f48771', fontSize: '12px', marginLeft: 'auto' }}>
              ⚠ {node.warning || 'Poor row estimate'}
            </span>
          )}
        </div>

        <div style={{ display: 'flex', gap: '16px', marginTop: '4px', fontSize: '12px', color: '#888' }}>
          <span>
            Cost: <span style={{ color: '#b5cea8' }}>{node.cost.toFixed(2)}</span>
          </span>

          <span>
            Rows: <span style={{ color: '#b5cea8' }}>{node.rows.toLocaleString()}</span>
            {node.actualRows !== undefined && (
              <>
                {' '}→{' '}
                <span style={{ color: rowsAccuracy! > 50 ? '#f48771' : '#4ec9b0' }}>
                  {node.actualRows.toLocaleString()}
                </span>
                {rowsAccuracy !== null && (
                  <span style={{ color: '#666', fontSize: '11px' }}>
                    {' '}({rowsAccuracy.toFixed(0)}% diff)
                  </span>
                )}
              </>
            )}
          </span>

          {node.time !== undefined && (
            <span>
              Time: <span style={{ color: '#ce9178' }}>{node.time.toFixed(2)}ms</span>
            </span>
          )}
        </div>

        {node.details && Object.keys(node.details).length > 0 && (
          <div style={{ marginTop: '4px', fontSize: '11px', color: '#666' }}>
            {Object.entries(node.details).map(([key, value]) => (
              <div key={key} style={{ marginLeft: '20px' }}>
                {key}: <span style={{ color: '#9cdcfe' }}>{String(value)}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {hasChildren && isExpanded && (
        <div className="plan-node-children">
          {node.children.map((child, index) => (
            <PlanNodeItem
              key={child.id}
              node={child}
              depth={depth + 1}
              isLast={index === node.children.length - 1}
              parentPrefix={childPrefix}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const ExplainPlanViewer: React.FC<ExplainPlanViewerProps> = ({ plan }) => {
  const [viewMode, setViewMode] = useState<'tree' | 'json'>('tree');

  // Find most expensive nodes
  const expensiveNodes = useMemo(() => {
    const findExpensive = (nodes: PlanNode[], acc: PlanNode[] = []): PlanNode[] => {
      nodes.forEach(node => {
        acc.push(node);
        if (node.children) {
          findExpensive(node.children, acc);
        }
      });
      return acc;
    };

    const allNodes = findExpensive(plan.nodes);
    return allNodes
      .sort((a, b) => b.cost - a.cost)
      .slice(0, 5);
  }, [plan]);

  // Calculate total warnings
  const warningCount = useMemo(() => {
    const countWarnings = (nodes: PlanNode[]): number => {
      return nodes.reduce((count, node) => {
        let nodeWarnings = node.warning ? 1 : 0;
        if (node.actualRows !== undefined) {
          const accuracy = Math.abs((node.actualRows - node.rows) / Math.max(node.rows, 1)) * 100;
          if (accuracy > 50) nodeWarnings++;
        }
        if (node.children) {
          nodeWarnings += countWarnings(node.children);
        }
        return count + nodeWarnings;
      }, 0);
    };
    return countWarnings(plan.nodes);
  }, [plan]);

  return (
    <div className="explain-plan-viewer" style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      {/* Header */}
      <div
        style={{
          padding: '12px',
          background: '#1e1e1e',
          borderBottom: '1px solid #333',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <div>
          <h3 style={{ margin: 0, fontSize: '16px' }}>Query Execution Plan</h3>
          <div style={{ fontSize: '12px', color: '#888', marginTop: '4px' }}>
            Total Cost: <span style={{ color: '#b5cea8' }}>{plan.totalCost.toFixed(2)}</span>
            {' • '}
            Estimated Rows: <span style={{ color: '#b5cea8' }}>{plan.estimatedRows.toLocaleString()}</span>
            {warningCount > 0 && (
              <>
                {' • '}
                <span style={{ color: '#f48771' }}>
                  ⚠ {warningCount} warning{warningCount !== 1 ? 's' : ''}
                </span>
              </>
            )}
          </div>
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            onClick={() => setViewMode('tree')}
            style={{
              padding: '6px 12px',
              background: viewMode === 'tree' ? '#0e639c' : '#2d2d2d',
              border: 'none',
              color: '#fff',
              cursor: 'pointer',
              fontSize: '12px',
            }}
          >
            Tree View
          </button>
          <button
            onClick={() => setViewMode('json')}
            style={{
              padding: '6px 12px',
              background: viewMode === 'json' ? '#0e639c' : '#2d2d2d',
              border: 'none',
              color: '#fff',
              cursor: 'pointer',
              fontSize: '12px',
            }}
          >
            JSON View
          </button>
        </div>
      </div>

      {/* Expensive Operations Summary */}
      {expensiveNodes.length > 0 && (
        <div
          style={{
            padding: '12px',
            background: '#252526',
            borderBottom: '1px solid #333',
          }}
        >
          <div style={{ fontSize: '13px', fontWeight: 'bold', marginBottom: '8px' }}>
            Most Expensive Operations:
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            {expensiveNodes.map((node, index) => (
              <div key={node.id} style={{ fontSize: '12px', display: 'flex', gap: '8px' }}>
                <span style={{ color: '#666' }}>{index + 1}.</span>
                <span style={{ color: '#4ec9b0' }}>{node.operation}</span>
                {node.table && <span style={{ color: '#dcdcaa' }}>on {node.table}</span>}
                <span style={{ color: '#888', marginLeft: 'auto' }}>
                  Cost: <span style={{ color: '#b5cea8' }}>{node.cost.toFixed(2)}</span>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Content */}
      <div style={{ flex: 1, overflow: 'auto', padding: '12px', background: '#1e1e1e' }}>
        {viewMode === 'tree' ? (
          <div className="plan-tree">
            {plan.nodes.map((node, index) => (
              <PlanNodeItem
                key={node.id}
                node={node}
                depth={0}
                isLast={index === plan.nodes.length - 1}
                parentPrefix=""
              />
            ))}
          </div>
        ) : (
          <pre
            style={{
              margin: 0,
              fontSize: '12px',
              fontFamily: 'monospace',
              color: '#d4d4d4',
              whiteSpace: 'pre-wrap',
              wordWrap: 'break-word',
            }}
          >
            {JSON.stringify(plan, null, 2)}
          </pre>
        )}
      </div>

      <style>{`
        .plan-node {
          font-family: 'Consolas', 'Monaco', monospace;
          font-size: 13px;
          line-height: 1.6;
        }

        .plan-node-header {
          transition: background-color 0.2s;
        }

        .plan-node-header:hover {
          background: #2a2d2e !important;
        }

        .plan-node-header.warning:hover {
          background: #442a2a !important;
        }
      `}</style>
    </div>
  );
};

function useMemo<T>(factory: () => T, deps: React.DependencyList): T {
  return React.useMemo(factory, deps);
}

export default ExplainPlanViewer;
