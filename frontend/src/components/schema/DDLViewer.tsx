// ============================================================================
// DDL Viewer Component
// Display DDL with syntax highlighting
// ============================================================================

import { useState } from 'react';
import { motion } from 'framer-motion';
import {
  ClipboardDocumentIcon,
  CheckIcon,
  DocumentArrowDownIcon,
} from '@heroicons/react/24/outline';
import clsx from 'clsx';

interface DDLViewerProps {
  ddl: string;
  title?: string;
  language?: 'sql' | 'plpgsql';
  showLineNumbers?: boolean;
  maxHeight?: string;
}

export function DDLViewer({
  ddl,
  title = 'DDL Script',
  language = 'sql',
  showLineNumbers = true,
  maxHeight = '500px',
}: DDLViewerProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(ddl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy DDL:', error);
    }
  };

  const handleDownload = () => {
    const blob = new Blob([ddl], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${title.replace(/\s+/g, '_').toLowerCase()}.sql`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  // Simple syntax highlighting for SQL
  const highlightSQL = (code: string): string => {
    const keywords = [
      'CREATE',
      'TABLE',
      'ALTER',
      'DROP',
      'SELECT',
      'INSERT',
      'UPDATE',
      'DELETE',
      'FROM',
      'WHERE',
      'JOIN',
      'LEFT',
      'RIGHT',
      'INNER',
      'OUTER',
      'ON',
      'AND',
      'OR',
      'NOT',
      'NULL',
      'PRIMARY',
      'KEY',
      'FOREIGN',
      'REFERENCES',
      'UNIQUE',
      'INDEX',
      'DEFAULT',
      'CHECK',
      'CONSTRAINT',
      'CASCADE',
      'RESTRICT',
      'SET',
      'VIEW',
      'MATERIALIZED',
      'FUNCTION',
      'PROCEDURE',
      'RETURNS',
      'BEGIN',
      'END',
      'IF',
      'THEN',
      'ELSE',
      'ELSIF',
      'LOOP',
      'WHILE',
      'FOR',
      'RETURN',
      'COMMENT',
    ];

    const dataTypes = [
      'INTEGER',
      'BIGINT',
      'SMALLINT',
      'DECIMAL',
      'NUMERIC',
      'REAL',
      'DOUBLE',
      'VARCHAR',
      'CHAR',
      'TEXT',
      'BOOLEAN',
      'DATE',
      'TIME',
      'TIMESTAMP',
      'TIMESTAMPTZ',
      'UUID',
      'JSON',
      'JSONB',
      'BYTEA',
    ];

    let highlighted = code;

    // Highlight keywords
    keywords.forEach((keyword) => {
      const regex = new RegExp(`\\b${keyword}\\b`, 'gi');
      highlighted = highlighted.replace(
        regex,
        `<span class="text-purple-400 font-semibold">${keyword}</span>`
      );
    });

    // Highlight data types
    dataTypes.forEach((type) => {
      const regex = new RegExp(`\\b${type}\\b`, 'gi');
      highlighted = highlighted.replace(
        regex,
        `<span class="text-blue-400">${type}</span>`
      );
    });

    // Highlight strings
    highlighted = highlighted.replace(
      /'([^']*)'/g,
      `<span class="text-green-400">'$1'</span>`
    );

    // Highlight numbers
    highlighted = highlighted.replace(
      /\b(\d+)\b/g,
      `<span class="text-yellow-400">$1</span>`
    );

    // Highlight comments
    highlighted = highlighted.replace(
      /--(.*)$/gm,
      `<span class="text-dark-500 italic">--$1</span>`
    );

    return highlighted;
  };

  const lines = ddl.split('\n');
  const highlightedLines = lines.map((line) => highlightSQL(line));

  return (
    <div className="card">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-dark-700">
        <h3 className="text-sm font-medium text-dark-200">{title}</h3>
        <div className="flex items-center gap-2">
          <button
            onClick={handleDownload}
            className="p-2 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
            title="Download DDL"
          >
            <DocumentArrowDownIcon className="w-4 h-4" />
          </button>
          <button
            onClick={handleCopy}
            className="p-2 rounded hover:bg-dark-700 text-dark-400 hover:text-dark-200 transition-colors"
            title="Copy to Clipboard"
          >
            {copied ? (
              <motion.div
                initial={{ scale: 0.8 }}
                animate={{ scale: 1 }}
                className="text-success-400"
              >
                <CheckIcon className="w-4 h-4" />
              </motion.div>
            ) : (
              <ClipboardDocumentIcon className="w-4 h-4" />
            )}
          </button>
        </div>
      </div>

      {/* Code */}
      <div
        className="overflow-auto bg-dark-900"
        style={{ maxHeight }}
      >
        <div className="flex">
          {/* Line Numbers */}
          {showLineNumbers && (
            <div className="flex-shrink-0 py-4 px-3 bg-dark-800/50 border-r border-dark-700 select-none">
              {lines.map((_, i) => (
                <div
                  key={i}
                  className="text-right text-dark-500 text-sm leading-6 font-mono"
                  style={{ minWidth: '2rem' }}
                >
                  {i + 1}
                </div>
              ))}
            </div>
          )}

          {/* Code Content */}
          <div className="flex-1 py-4 px-4">
            {highlightedLines.map((line, i) => (
              <div
                key={i}
                className="text-sm leading-6 font-mono text-dark-200 whitespace-pre"
                dangerouslySetInnerHTML={{ __html: line || '&nbsp;' }}
              />
            ))}
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between px-4 py-2 border-t border-dark-700 text-xs text-dark-400">
        <span>{language.toUpperCase()}</span>
        <span>{lines.length} lines</span>
      </div>
    </div>
  );
}
