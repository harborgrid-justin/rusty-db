import React, { useState, useMemo, useCallback } from 'react';
import { FixedSizeGrid as Grid } from 'react-window';
import AutoSizer from 'react-virtualized-auto-sizer';
import { QueryResult } from '../../stores/queryStore';
import { useExportResults } from '../../hooks/useQuery';

interface ResultsTableProps {
  result: QueryResult;
}

type SortDirection = 'asc' | 'desc' | null;

interface SortConfig {
  columnIndex: number | null;
  direction: SortDirection;
}

interface FilterConfig {
  [columnIndex: number]: string;
}

export const ResultsTable: React.FC<ResultsTableProps> = ({ result }) => {
  const [sortConfig, setSortConfig] = useState<SortConfig>({ columnIndex: null, direction: null });
  const [filters, setFilters] = useState<FilterConfig>({});
  const [selectedCells, setSelectedCells] = useState<Set<string>>(new Set());
  const { exportResults, isExporting } = useExportResults();

  // Column width (can be made dynamic)
  const COLUMN_WIDTH = 150;
  const ROW_HEIGHT = 35;
  const HEADER_HEIGHT = 40;

  // Sort and filter data
  const processedData = useMemo(() => {
    let data = [...result.rows];

    // Apply filters
    Object.entries(filters).forEach(([columnIndex, filterValue]) => {
      if (filterValue) {
        const colIdx = parseInt(columnIndex);
        data = data.filter(row => {
          const cellValue = row[colIdx];
          const strValue = cellValue === null ? 'NULL' : String(cellValue).toLowerCase();
          return strValue.includes(filterValue.toLowerCase());
        });
      }
    });

    // Apply sorting
    if (sortConfig.columnIndex !== null && sortConfig.direction) {
      data.sort((a, b) => {
        const aVal = a[sortConfig.columnIndex!];
        const bVal = b[sortConfig.columnIndex!];

        if (aVal === null && bVal === null) return 0;
        if (aVal === null) return sortConfig.direction === 'asc' ? 1 : -1;
        if (bVal === null) return sortConfig.direction === 'asc' ? -1 : 1;

        let comparison = 0;
        if (typeof aVal === 'number' && typeof bVal === 'number') {
          comparison = aVal - bVal;
        } else {
          comparison = String(aVal).localeCompare(String(bVal));
        }

        return sortConfig.direction === 'asc' ? comparison : -comparison;
      });
    }

    return data;
  }, [result.rows, sortConfig, filters]);

  const handleSort = useCallback((columnIndex: number) => {
    setSortConfig(prev => {
      if (prev.columnIndex === columnIndex) {
        const newDirection = prev.direction === 'asc' ? 'desc' : prev.direction === 'desc' ? null : 'asc';
        return { columnIndex: newDirection ? columnIndex : null, direction: newDirection };
      }
      return { columnIndex, direction: 'asc' };
    });
  }, []);

  const handleFilter = useCallback((columnIndex: number, value: string) => {
    setFilters(prev => {
      if (!value) {
        const { [columnIndex]: _, ...rest } = prev;
        return rest;
      }
      return { ...prev, [columnIndex]: value };
    });
  }, []);

  const formatCellValue = (value: any): string => {
    if (value === null || value === undefined) {
      return 'NULL';
    }
    if (typeof value === 'boolean') {
      return value ? 'true' : 'false';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value);
    }
    return String(value);
  };

  const getCellClassName = (value: any): string => {
    if (value === null || value === undefined) {
      return 'cell-null';
    }
    if (typeof value === 'number') {
      return 'cell-number';
    }
    if (typeof value === 'boolean') {
      return 'cell-boolean';
    }
    return 'cell-string';
  };

  const copyCellValue = useCallback((value: any) => {
    const text = formatCellValue(value);
    navigator.clipboard.writeText(text);
  }, []);

  const copyRow = useCallback((row: any[]) => {
    const text = row.map(formatCellValue).join('\t');
    navigator.clipboard.writeText(text);
  }, []);

  const copyAllResults = useCallback(() => {
    const header = result.columns.join('\t');
    const rows = processedData.map(row => row.map(formatCellValue).join('\t'));
    const text = [header, ...rows].join('\n');
    navigator.clipboard.writeText(text);
  }, [result.columns, processedData]);

  const handleExport = useCallback(async (format: 'csv' | 'json') => {
    try {
      const exportData: QueryResult = {
        ...result,
        rows: processedData,
      };
      await exportResults(format, exportData);
    } catch (error) {
      console.error('Export failed:', error);
    }
  }, [result, processedData, exportResults]);

  // Grid cell renderer
  const Cell = useCallback(
    ({ columnIndex, rowIndex, style }: any) => {
      const isHeader = rowIndex === 0;

      if (isHeader) {
        const column = result.columns[columnIndex];
        const isSorted = sortConfig.columnIndex === columnIndex;
        const sortIcon = isSorted
          ? sortConfig.direction === 'asc'
            ? ' ↑'
            : ' ↓'
          : '';

        return (
          <div
            style={{
              ...style,
              borderRight: '1px solid #333',
              borderBottom: '2px solid #444',
              background: '#1e1e1e',
              padding: '8px',
              fontWeight: 'bold',
              cursor: 'pointer',
              display: 'flex',
              flexDirection: 'column',
              gap: '4px',
            }}
            onClick={() => handleSort(columnIndex)}
          >
            <div style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {column}
              {sortIcon}
            </div>
            <input
              type="text"
              placeholder="Filter..."
              value={filters[columnIndex] || ''}
              onChange={(e) => handleFilter(columnIndex, e.target.value)}
              onClick={(e) => e.stopPropagation()}
              style={{
                width: '100%',
                padding: '2px 4px',
                background: '#2d2d2d',
                border: '1px solid #444',
                color: '#fff',
                fontSize: '11px',
              }}
            />
          </div>
        );
      }

      const dataRowIndex = rowIndex - 1;
      const row = processedData[dataRowIndex];
      if (!row) return null;

      const value = row[columnIndex];
      const cellKey = `${dataRowIndex}-${columnIndex}`;
      const isSelected = selectedCells.has(cellKey);

      return (
        <div
          style={{
            ...style,
            borderRight: '1px solid #333',
            borderBottom: '1px solid #333',
            padding: '8px',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            background: isSelected ? '#264f78' : dataRowIndex % 2 === 0 ? '#1e1e1e' : '#252526',
            cursor: 'pointer',
          }}
          className={getCellClassName(value)}
          onClick={() => copyCellValue(value)}
          onContextMenu={(e) => {
            e.preventDefault();
            copyRow(row);
          }}
          title={`${formatCellValue(value)}\n\nClick to copy cell\nRight-click to copy row`}
        >
          {formatCellValue(value)}
        </div>
      );
    },
    [result.columns, processedData, sortConfig, filters, selectedCells, handleSort, handleFilter, copyCellValue, copyRow]
  );

  return (
    <div className="results-table-container" style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Toolbar */}
      <div
        style={{
          padding: '8px 12px',
          background: '#1e1e1e',
          borderBottom: '1px solid #333',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <div style={{ fontSize: '13px', color: '#888' }}>
          {processedData.length} row{processedData.length !== 1 ? 's' : ''}
          {processedData.length !== result.rowCount && ` (filtered from ${result.rowCount})`}
          {' • '}
          Execution time: {result.executionTime}ms
          {result.affected !== undefined && ` • Affected: ${result.affected}`}
        </div>

        <div style={{ display: 'flex', gap: '8px' }}>
          <button
            onClick={copyAllResults}
            style={{
              padding: '4px 12px',
              background: '#0e639c',
              border: 'none',
              color: '#fff',
              cursor: 'pointer',
              fontSize: '12px',
            }}
            title="Copy all results to clipboard"
          >
            Copy All
          </button>
          <button
            onClick={() => handleExport('csv')}
            disabled={isExporting}
            style={{
              padding: '4px 12px',
              background: '#0e639c',
              border: 'none',
              color: '#fff',
              cursor: isExporting ? 'not-allowed' : 'pointer',
              fontSize: '12px',
              opacity: isExporting ? 0.5 : 1,
            }}
          >
            Export CSV
          </button>
          <button
            onClick={() => handleExport('json')}
            disabled={isExporting}
            style={{
              padding: '4px 12px',
              background: '#0e639c',
              border: 'none',
              color: '#fff',
              cursor: isExporting ? 'not-allowed' : 'pointer',
              fontSize: '12px',
              opacity: isExporting ? 0.5 : 1,
            }}
          >
            Export JSON
          </button>
        </div>
      </div>

      {/* Virtualized Grid */}
      <div style={{ flex: 1 }}>
        <AutoSizer>
          {({ height, width }) => (
            <Grid
              columnCount={result.columns.length}
              columnWidth={COLUMN_WIDTH}
              height={height}
              rowCount={processedData.length + 1} // +1 for header
              rowHeight={(index) => (index === 0 ? HEADER_HEIGHT : ROW_HEIGHT)}
              width={width}
              overscanRowCount={5}
              overscanColumnCount={2}
            >
              {Cell}
            </Grid>
          )}
        </AutoSizer>
      </div>

      <style>{`
        .cell-null {
          color: #808080;
          font-style: italic;
        }
        .cell-number {
          color: #b5cea8;
          text-align: right;
        }
        .cell-boolean {
          color: #569cd6;
        }
        .cell-string {
          color: #ce9178;
        }
      `}</style>
    </div>
  );
};

export default ResultsTable;
