// ============================================================================
// Cluster Topology Visualization Component
// Interactive network diagram showing cluster topology
// ============================================================================

import { useEffect, useRef, useState, useMemo } from 'react';
import * as d3 from 'd3';
import type { ClusterTopology, ClusterNode, UUID } from '../../types';
import { ServerIcon, CheckCircleIcon, ExclamationCircleIcon, XCircleIcon } from '@heroicons/react/24/solid';

interface ClusterTopologyProps {
  topology: ClusterTopology;
  onNodeClick?: (node: ClusterNode) => void;
  height?: number;
  className?: string;
}

interface NodePosition extends ClusterNode {
  x: number;
  y: number;
  fx?: number | null;
  fy?: number | null;
}

interface Link {
  source: NodePosition;
  target: NodePosition;
  strength: number;
}

export function ClusterTopology({
  topology,
  onNodeClick,
  height = 600,
  className = '',
}: ClusterTopologyProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height });
  const [hoveredNode, setHoveredNode] = useState<UUID | null>(null);

  // Update dimensions on container resize
  useEffect(() => {
    if (!containerRef.current) return;

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        setDimensions({
          width: entry.contentRect.width,
          height,
        });
      }
    });

    resizeObserver.observe(containerRef.current);

    return () => resizeObserver.disconnect();
  }, [height]);

  // Prepare graph data
  const graphData = useMemo(() => {
    const nodes: NodePosition[] = topology.nodes.map((node) => ({
      ...node,
      x: dimensions.width / 2,
      y: dimensions.height / 2,
    }));

    const links: Link[] = [];

    // Create links from leader to followers
    const leader = nodes.find((n) => n.role === 'leader');
    if (leader) {
      nodes.forEach((node) => {
        if (node.role === 'follower' || node.role === 'observer') {
          links.push({
            source: leader as NodePosition,
            target: node,
            strength: node.role === 'follower' ? 1 : 0.5,
          });
        }
      });
    }

    return { nodes, links };
  }, [topology, dimensions]);

  // D3 visualization
  useEffect(() => {
    if (!svgRef.current || dimensions.width === 0) return;

    const svg = d3.select(svgRef.current);
    svg.selectAll('*').remove();

    const { width, height } = dimensions;

    // Create zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.5, 3])
      .on('zoom', (event) => {
        g.attr('transform', event.transform);
      });

    svg.call(zoom);

    const g = svg.append('g');

    // Create force simulation
    const simulation = d3
      .forceSimulation<NodePosition>(graphData.nodes)
      .force(
        'link',
        d3.forceLink<NodePosition, Link>(graphData.links)
          .id((d) => d.id)
          .distance(150)
          .strength((d) => d.strength)
      )
      .force('charge', d3.forceManyBody().strength(-500))
      .force('center', d3.forceCenter(width / 2, height / 2))
      .force('collision', d3.forceCollide().radius(60));

    // Create arrow markers for directed edges
    const defs = g.append('defs');

    defs
      .append('marker')
      .attr('id', 'arrowhead-active')
      .attr('markerWidth', 10)
      .attr('markerHeight', 10)
      .attr('refX', 40)
      .attr('refY', 3)
      .attr('orient', 'auto')
      .append('polygon')
      .attr('points', '0 0, 10 3, 0 6')
      .attr('fill', '#10b981');

    defs
      .append('marker')
      .attr('id', 'arrowhead-inactive')
      .attr('markerWidth', 10)
      .attr('markerHeight', 10)
      .attr('refX', 40)
      .attr('refY', 3)
      .attr('orient', 'auto')
      .append('polygon')
      .attr('points', '0 0, 10 3, 0 6')
      .attr('fill', '#6b7280');

    // Draw links
    const link = g
      .append('g')
      .selectAll('line')
      .data(graphData.links)
      .enter()
      .append('line')
      .attr('class', 'link')
      .attr('stroke', (d) =>
        d.target.status === 'healthy' ? '#10b981' : '#6b7280'
      )
      .attr('stroke-width', (d) => (d.strength === 1 ? 2 : 1))
      .attr('stroke-opacity', (d) => (d.strength === 1 ? 0.8 : 0.4))
      .attr('stroke-dasharray', (d) => (d.strength === 1 ? 'none' : '5,5'))
      .attr('marker-end', (d) =>
        d.target.status === 'healthy'
          ? 'url(#arrowhead-active)'
          : 'url(#arrowhead-inactive)'
      );

    // Draw nodes
    const node = g
      .append('g')
      .selectAll('g')
      .data(graphData.nodes)
      .enter()
      .append('g')
      .attr('class', 'node')
      .style('cursor', 'pointer')
      .call(
        d3
          .drag<SVGGElement, NodePosition>()
          .on('start', dragStarted)
          .on('drag', dragged)
          .on('end', dragEnded)
      )
      .on('click', (event, d) => {
        event.stopPropagation();
        onNodeClick?.(d);
      })
      .on('mouseenter', (event, d) => {
        setHoveredNode(d.id);
      })
      .on('mouseleave', () => {
        setHoveredNode(null);
      });

    // Node circles
    node
      .append('circle')
      .attr('r', (d) => (d.role === 'leader' ? 40 : 30))
      .attr('fill', (d) => getNodeColor(d))
      .attr('stroke', (d) => (d.role === 'leader' ? '#fbbf24' : '#e5e7eb'))
      .attr('stroke-width', (d) => (d.role === 'leader' ? 4 : 2));

    // Node icons (simplified representation)
    node
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dominant-baseline', 'central')
      .attr('font-size', (d) => (d.role === 'leader' ? '24' : '20'))
      .attr('fill', 'white')
      .text('⚙');

    // Node labels
    node
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', (d) => (d.role === 'leader' ? 55 : 45))
      .attr('font-size', '12')
      .attr('font-weight', 'bold')
      .attr('fill', '#1f2937')
      .text((d) => d.name);

    // Role labels
    node
      .append('text')
      .attr('text-anchor', 'middle')
      .attr('dy', (d) => (d.role === 'leader' ? 70 : 60))
      .attr('font-size', '10')
      .attr('fill', '#6b7280')
      .text((d) => d.role.toUpperCase());

    // Update positions on simulation tick
    simulation.on('tick', () => {
      link
        .attr('x1', (d) => d.source.x)
        .attr('y1', (d) => d.source.y)
        .attr('x2', (d) => d.target.x)
        .attr('y2', (d) => d.target.y);

      node.attr('transform', (d) => `translate(${d.x},${d.y})`);
    });

    function dragStarted(event: d3.D3DragEvent<SVGGElement, NodePosition, NodePosition>) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      event.subject.fx = event.subject.x;
      event.subject.fy = event.subject.y;
    }

    function dragged(event: d3.D3DragEvent<SVGGElement, NodePosition, NodePosition>) {
      event.subject.fx = event.x;
      event.subject.fy = event.y;
    }

    function dragEnded(event: d3.D3DragEvent<SVGGElement, NodePosition, NodePosition>) {
      if (!event.active) simulation.alphaTarget(0);
      event.subject.fx = null;
      event.subject.fy = null;
    }

    return () => {
      simulation.stop();
    };
  }, [graphData, dimensions, onNodeClick]);

  function getNodeColor(node: ClusterNode): string {
    switch (node.status) {
      case 'healthy':
        return '#10b981'; // green
      case 'degraded':
        return '#f59e0b'; // amber
      case 'unreachable':
        return '#ef4444'; // red
      case 'shutting_down':
        return '#6b7280'; // gray
      case 'failed':
        return '#dc2626'; // dark red
      default:
        return '#9ca3af'; // default gray
    }
  }

  function getStatusIcon(status: ClusterNode['status']) {
    switch (status) {
      case 'healthy':
        return <CheckCircleIcon className="w-5 h-5 text-green-500" />;
      case 'degraded':
        return <ExclamationCircleIcon className="w-5 h-5 text-amber-500" />;
      case 'unreachable':
      case 'failed':
        return <XCircleIcon className="w-5 h-5 text-red-500" />;
      default:
        return <ServerIcon className="w-5 h-5 text-gray-500" />;
    }
  }

  return (
    <div className={`relative ${className}`} ref={containerRef}>
      <svg
        ref={svgRef}
        width="100%"
        height={height}
        className="border border-gray-200 rounded-lg bg-gray-50"
      />

      {/* Legend */}
      <div className="absolute top-4 right-4 bg-white border border-gray-200 rounded-lg shadow-lg p-4 space-y-2">
        <div className="text-xs font-semibold text-gray-700 mb-2">Legend</div>

        <div className="flex items-center space-x-2 text-xs">
          <div className="w-4 h-4 rounded-full bg-green-500 border-2 border-amber-400"></div>
          <span className="text-gray-600">Leader</span>
        </div>

        <div className="flex items-center space-x-2 text-xs">
          <div className="w-4 h-4 rounded-full bg-green-500"></div>
          <span className="text-gray-600">Healthy</span>
        </div>

        <div className="flex items-center space-x-2 text-xs">
          <div className="w-4 h-4 rounded-full bg-amber-500"></div>
          <span className="text-gray-600">Degraded</span>
        </div>

        <div className="flex items-center space-x-2 text-xs">
          <div className="w-4 h-4 rounded-full bg-red-500"></div>
          <span className="text-gray-600">Unreachable</span>
        </div>

        <div className="border-t border-gray-200 pt-2 mt-2">
          <div className="flex items-center space-x-2 text-xs">
            <div className="w-8 h-0.5 bg-green-500"></div>
            <span className="text-gray-600">Replication</span>
          </div>

          <div className="flex items-center space-x-2 text-xs mt-1">
            <div className="w-8 h-0.5 bg-gray-400 border-dashed" style={{ borderTopWidth: 1 }}></div>
            <span className="text-gray-600">Observer</span>
          </div>
        </div>
      </div>

      {/* Hovered node info */}
      {hoveredNode && (
        <div className="absolute bottom-4 left-4 bg-white border border-gray-200 rounded-lg shadow-lg p-3 min-w-[200px]">
          {(() => {
            const node = topology.nodes.find((n) => n.id === hoveredNode);
            if (!node) return null;

            return (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-sm text-gray-900">
                    {node.name}
                  </span>
                  {getStatusIcon(node.status)}
                </div>

                <div className="text-xs space-y-1">
                  <div className="flex justify-between">
                    <span className="text-gray-500">Role:</span>
                    <span className="font-medium text-gray-700">
                      {node.role}
                    </span>
                  </div>

                  <div className="flex justify-between">
                    <span className="text-gray-500">Host:</span>
                    <span className="font-medium text-gray-700">
                      {node.host}:{node.port}
                    </span>
                  </div>

                  {node.metrics && (
                    <>
                      <div className="flex justify-between">
                        <span className="text-gray-500">CPU:</span>
                        <span className="font-medium text-gray-700">
                          {node.metrics.cpu.toFixed(1)}%
                        </span>
                      </div>

                      <div className="flex justify-between">
                        <span className="text-gray-500">Memory:</span>
                        <span className="font-medium text-gray-700">
                          {node.metrics.memory.toFixed(1)}%
                        </span>
                      </div>

                      {node.metrics.replicationLag !== undefined && (
                        <div className="flex justify-between">
                          <span className="text-gray-500">Lag:</span>
                          <span className="font-medium text-gray-700">
                            {(node.metrics.replicationLag / 1000).toFixed(2)}s
                          </span>
                        </div>
                      )}
                    </>
                  )}
                </div>
              </div>
            );
          })()}
        </div>
      )}

      {/* Controls hint */}
      <div className="absolute bottom-4 right-4 bg-white border border-gray-200 rounded-lg shadow-lg px-3 py-2">
        <p className="text-xs text-gray-600">
          Drag to move nodes • Scroll to zoom • Click for details
        </p>
      </div>
    </div>
  );
}
