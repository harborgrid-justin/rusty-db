// ============================================================================
// Relationship Diagram Component
// Simple ERD visualization for table relationships
// ============================================================================

import { useRef, useState } from 'react';
import {
  TableCellsIcon,
  ArrowsRightLeftIcon,
  MagnifyingGlassMinusIcon,
  MagnifyingGlassPlusIcon,
} from '@heroicons/react/24/outline';
import type { Table, ForeignKey } from '../../types';
import clsx from 'clsx';

interface RelationshipDiagramProps {
  tables: Table[];
  selectedTable?: string;
  onTableClick?: (tableName: string) => void;
}

interface TableNode {
  table: Table;
  x: number;
  y: number;
  width: number;
  height: number;
}

export function RelationshipDiagram({
  tables,
  selectedTable,
  onTableClick,
}: RelationshipDiagramProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });

  // Calculate table positions (simple grid layout)
  const calculateLayout = (): TableNode[] => {
    const nodes: TableNode[] = [];
    const columnWidth = 250;
    const rowHeight = 200;
    const padding = 50;
    const columns = Math.ceil(Math.sqrt(tables.length));

    tables.forEach((table, index) => {
      const col = index % columns;
      const row = Math.floor(index / columns);

      const columnCount = Math.min(table.columns.length, 8);
      const height = 60 + columnCount * 24;

      nodes.push({
        table,
        x: col * (columnWidth + padding),
        y: row * (rowHeight + padding),
        width: columnWidth,
        height,
      });
    });

    return nodes;
  };

  const nodes = calculateLayout();

  // Find relationships
  const getRelationships = (): Array<{
    from: TableNode;
    to: TableNode;
    fk: ForeignKey;
  }> => {
    const relationships: Array<{ from: TableNode; to: TableNode; fk: ForeignKey }> = [];

    nodes.forEach((node) => {
      node.table.foreignKeys.forEach((fk) => {
        const targetNode = nodes.find((n) => n.table.name === fk.referencedTable);
        if (targetNode) {
          relationships.push({ from: node, to: targetNode, fk });
        }
      });
    });

    return relationships;
  };

  const relationships = getRelationships();

  // Calculate SVG viewBox
  const viewBox = {
    minX: -50,
    minY: -50,
    width: Math.max(...nodes.map((n) => n.x + n.width)) + 100,
    height: Math.max(...nodes.map((n) => n.y + n.height)) + 100,
  };

  // Mouse handlers for pan
  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.target === svgRef.current) {
      setIsDragging(true);
      setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setPan({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y,
      });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const handleZoomIn = () => {
    setZoom(Math.min(zoom * 1.2, 3));
  };

  const handleZoomOut = () => {
    setZoom(Math.max(zoom / 1.2, 0.3));
  };

  const handleResetView = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };

  // Draw relationship line
  const drawRelationshipLine = (from: TableNode, to: TableNode) => {
    const startX = from.x + from.width / 2;
    const startY = from.y + from.height / 2;
    const endX = to.x + to.width / 2;
    const endY = to.y + to.height / 2;

    // Simple straight line for now
    const path = `M ${startX} ${startY} L ${endX} ${endY}`;

    return path;
  };

  if (tables.length === 0) {
    return (
      <div className="card text-center py-12">
        <ArrowsRightLeftIcon className="w-16 h-16 text-dark-400 mx-auto mb-4" />
        <h3 className="text-lg font-medium text-dark-200 mb-2">No Tables to Display</h3>
        <p className="text-dark-400">Create tables to see their relationships</p>
      </div>
    );
  }

  return (
    <div className="card relative">
      {/* Controls */}
      <div className="absolute top-4 right-4 z-10 flex items-center gap-2">
        <button
          onClick={handleZoomIn}
          className="p-2 bg-dark-800 border border-dark-700 rounded hover:bg-dark-700 text-dark-300"
          title="Zoom In"
        >
          <MagnifyingGlassPlusIcon className="w-4 h-4" />
        </button>
        <button
          onClick={handleZoomOut}
          className="p-2 bg-dark-800 border border-dark-700 rounded hover:bg-dark-700 text-dark-300"
          title="Zoom Out"
        >
          <MagnifyingGlassMinusIcon className="w-4 h-4" />
        </button>
        <button
          onClick={handleResetView}
          className="px-3 py-2 bg-dark-800 border border-dark-700 rounded hover:bg-dark-700 text-dark-300 text-xs"
        >
          Reset
        </button>
      </div>

      {/* SVG Canvas */}
      <svg
        ref={svgRef}
        className="w-full h-[600px] bg-dark-900 cursor-move"
        viewBox={`${viewBox.minX} ${viewBox.minY} ${viewBox.width} ${viewBox.height}`}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        style={{
          transform: `scale(${zoom}) translate(${pan.x / zoom}px, ${pan.y / zoom}px)`,
        }}
      >
        {/* Relationship Lines */}
        <g className="relationships">
          {relationships.map((rel, index) => (
            <g key={index}>
              <path
                d={drawRelationshipLine(rel.from, rel.to)}
                stroke="#6B7280"
                strokeWidth="2"
                fill="none"
                markerEnd="url(#arrowhead)"
                opacity="0.5"
              />
            </g>
          ))}
        </g>

        {/* Arrow marker definition */}
        <defs>
          <marker
            id="arrowhead"
            markerWidth="10"
            markerHeight="10"
            refX="9"
            refY="3"
            orient="auto"
          >
            <polygon points="0 0, 10 3, 0 6" fill="#6B7280" />
          </marker>
        </defs>

        {/* Table Nodes */}
        <g className="tables">
          {nodes.map((node) => {
            const isSelected = node.table.name === selectedTable;
            const hasRelationship = relationships.some(
              (rel) =>
                rel.from.table.name === node.table.name ||
                rel.to.table.name === node.table.name
            );

            return (
              <g
                key={node.table.name}
                transform={`translate(${node.x}, ${node.y})`}
                onClick={() => onTableClick?.(node.table.name)}
                className="cursor-pointer"
              >
                {/* Table Background */}
                <rect
                  width={node.width}
                  height={node.height}
                  rx="8"
                  className={clsx(
                    'transition-all',
                    isSelected
                      ? 'fill-rusty-500/20 stroke-rusty-500'
                      : hasRelationship
                      ? 'fill-dark-800 stroke-dark-600'
                      : 'fill-dark-800 stroke-dark-700'
                  )}
                  strokeWidth="2"
                />

                {/* Table Header */}
                <rect
                  width={node.width}
                  height="40"
                  rx="8"
                  className={clsx(
                    isSelected ? 'fill-rusty-500/30' : 'fill-dark-700'
                  )}
                />

                {/* Table Icon and Name */}
                <foreignObject x="10" y="8" width={node.width - 20} height="40">
                  <div className="flex items-center gap-2 h-full">
                    <TableCellsIcon className="w-5 h-5 text-dark-300 flex-shrink-0" />
                    <span className="text-sm font-semibold text-dark-100 truncate">
                      {node.table.name}
                    </span>
                    <span className="text-xs text-dark-400 ml-auto flex-shrink-0">
                      {node.table.columns.length} cols
                    </span>
                  </div>
                </foreignObject>

                {/* Columns (show first 8) */}
                <foreignObject x="10" y="48" width={node.width - 20} height={node.height - 50}>
                  <div className="space-y-1">
                    {node.table.columns.slice(0, 8).map((col) => (
                      <div
                        key={col.name}
                        className="flex items-center gap-2 text-xs text-dark-300"
                      >
                        {col.isPrimaryKey && (
                          <span className="text-blue-400 font-semibold">PK</span>
                        )}
                        {col.isForeignKey && (
                          <span className="text-purple-400 font-semibold">FK</span>
                        )}
                        <span className="truncate">{col.name}</span>
                        <span className="text-dark-500 ml-auto">{col.dataType}</span>
                      </div>
                    ))}
                    {node.table.columns.length > 8 && (
                      <div className="text-xs text-dark-500 italic">
                        +{node.table.columns.length - 8} more...
                      </div>
                    )}
                  </div>
                </foreignObject>
              </g>
            );
          })}
        </g>
      </svg>

      {/* Legend */}
      <div className="absolute bottom-4 left-4 bg-dark-800 border border-dark-700 rounded-lg p-3 space-y-2">
        <div className="text-xs font-medium text-dark-300 mb-2">Legend</div>
        <div className="flex items-center gap-2 text-xs text-dark-400">
          <div className="w-4 h-4 bg-dark-700 border-2 border-dark-600 rounded" />
          <span>Table</span>
        </div>
        <div className="flex items-center gap-2 text-xs text-dark-400">
          <div className="w-4 h-4 bg-rusty-500/20 border-2 border-rusty-500 rounded" />
          <span>Selected</span>
        </div>
        <div className="flex items-center gap-2 text-xs text-dark-400">
          <div className="w-8 h-0.5 bg-dark-500" />
          <span>Foreign Key</span>
        </div>
      </div>

      {/* Info */}
      <div className="absolute bottom-4 right-4 bg-dark-800 border border-dark-700 rounded-lg px-3 py-2 text-xs text-dark-400">
        {tables.length} tables • {relationships.length} relationships
      </div>
    </div>
  );
}
