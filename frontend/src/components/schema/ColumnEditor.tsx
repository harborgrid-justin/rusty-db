// ============================================================================
// Column Editor Component
// Editor for defining/modifying table columns
// ============================================================================

import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  PlusIcon,
  TrashIcon,
  XMarkIcon,
  CheckIcon,
} from '@heroicons/react/24/outline';
import type { DataType } from '../../types';
import type { ColumnDefinition } from '../../services/schemaService';
import clsx from 'clsx';

interface ColumnEditorProps {
  columns: ColumnDefinition[];
  onChange: (columns: ColumnDefinition[]) => void;
  isEditing?: boolean;
}

const DATA_TYPES: DataType[] = [
  'integer',
  'bigint',
  'smallint',
  'decimal',
  'numeric',
  'real',
  'double',
  'varchar',
  'char',
  'text',
  'boolean',
  'date',
  'time',
  'timestamp',
  'timestamptz',
  'uuid',
  'json',
  'jsonb',
  'bytea',
];

const defaultColumn: ColumnDefinition = {
  name: '',
  dataType: 'varchar',
  nullable: true,
};

export function ColumnEditor({ columns, onChange, isEditing = false }: ColumnEditorProps) {
  const [editingColumns, setEditingColumns] = useState<ColumnDefinition[]>(columns);
  const [errors, setErrors] = useState<Record<number, string>>({});

  useEffect(() => {
    setEditingColumns(columns);
  }, [columns]);

  const validateColumn = (column: ColumnDefinition, index: number): string | null => {
    if (!column.name.trim()) {
      return 'Column name is required';
    }
    if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(column.name)) {
      return 'Invalid column name (must start with letter or underscore)';
    }
    // Check for duplicate names
    const duplicateIndex = editingColumns.findIndex(
      (col, i) => i !== index && col.name.toLowerCase() === column.name.toLowerCase()
    );
    if (duplicateIndex !== -1) {
      return 'Duplicate column name';
    }
    return null;
  };

  const handleColumnChange = (
    index: number,
    field: keyof ColumnDefinition,
    value: unknown
  ) => {
    const newColumns = [...editingColumns];
    newColumns[index] = { ...newColumns[index], [field]: value };

    // Clear length/precision/scale for non-applicable types
    if (field === 'dataType') {
      const type = value as string;
      if (!['varchar', 'char'].includes(type)) {
        delete newColumns[index].length;
      }
      if (!['decimal', 'numeric'].includes(type)) {
        delete newColumns[index].precision;
        delete newColumns[index].scale;
      }
    }

    setEditingColumns(newColumns);

    // Validate
    const error = validateColumn(newColumns[index], index);
    const newErrors = { ...errors };
    if (error) {
      newErrors[index] = error;
    } else {
      delete newErrors[index];
    }
    setErrors(newErrors);

    onChange(newColumns);
  };

  const addColumn = () => {
    const newColumns = [...editingColumns, { ...defaultColumn }];
    setEditingColumns(newColumns);
    onChange(newColumns);
  };

  const removeColumn = (index: number) => {
    const newColumns = editingColumns.filter((_, i) => i !== index);
    setEditingColumns(newColumns);
    const newErrors = { ...errors };
    delete newErrors[index];
    setErrors(newErrors);
    onChange(newColumns);
  };

  const requiresLength = (dataType: string) => ['varchar', 'char'].includes(dataType);
  const requiresPrecision = (dataType: string) =>
    ['decimal', 'numeric'].includes(dataType);

  return (
    <div className="space-y-4">
      {/* Column List */}
      <div className="space-y-2">
        <AnimatePresence mode="popLayout">
          {editingColumns.map((column, index) => (
            <motion.div
              key={index}
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className={clsx(
                'card p-4 space-y-3',
                errors[index] && 'border-danger-500'
              )}
            >
              {/* Column Header */}
              <div className="flex items-center justify-between">
                <h4 className="text-sm font-medium text-dark-200">
                  Column {index + 1}
                </h4>
                <button
                  onClick={() => removeColumn(index)}
                  className="p-1 rounded hover:bg-dark-700 text-dark-400 hover:text-danger-400 transition-colors"
                  title="Remove Column"
                >
                  <TrashIcon className="w-4 h-4" />
                </button>
              </div>

              {/* Error Message */}
              {errors[index] && (
                <div className="text-sm text-danger-400 bg-danger-500/10 px-3 py-2 rounded">
                  {errors[index]}
                </div>
              )}

              {/* Column Fields */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
                {/* Name */}
                <div>
                  <label className="block text-xs font-medium text-dark-400 mb-1">
                    Name *
                  </label>
                  <input
                    type="text"
                    value={column.name}
                    onChange={(e) => handleColumnChange(index, 'name', e.target.value)}
                    className={clsx('input', errors[index] && 'border-danger-500')}
                    placeholder="column_name"
                  />
                </div>

                {/* Data Type */}
                <div>
                  <label className="block text-xs font-medium text-dark-400 mb-1">
                    Data Type *
                  </label>
                  <select
                    value={column.dataType}
                    onChange={(e) =>
                      handleColumnChange(index, 'dataType', e.target.value)
                    }
                    className="input"
                  >
                    {DATA_TYPES.map((type) => (
                      <option key={type} value={type}>
                        {type.toUpperCase()}
                      </option>
                    ))}
                  </select>
                </div>

                {/* Length (for varchar/char) */}
                {requiresLength(column.dataType) && (
                  <div>
                    <label className="block text-xs font-medium text-dark-400 mb-1">
                      Length
                    </label>
                    <input
                      type="number"
                      value={column.length || ''}
                      onChange={(e) =>
                        handleColumnChange(
                          index,
                          'length',
                          e.target.value ? parseInt(e.target.value) : undefined
                        )
                      }
                      className="input"
                      placeholder="255"
                      min="1"
                    />
                  </div>
                )}

                {/* Precision (for decimal/numeric) */}
                {requiresPrecision(column.dataType) && (
                  <>
                    <div>
                      <label className="block text-xs font-medium text-dark-400 mb-1">
                        Precision
                      </label>
                      <input
                        type="number"
                        value={column.precision || ''}
                        onChange={(e) =>
                          handleColumnChange(
                            index,
                            'precision',
                            e.target.value ? parseInt(e.target.value) : undefined
                          )
                        }
                        className="input"
                        placeholder="10"
                        min="1"
                      />
                    </div>
                    <div>
                      <label className="block text-xs font-medium text-dark-400 mb-1">
                        Scale
                      </label>
                      <input
                        type="number"
                        value={column.scale || ''}
                        onChange={(e) =>
                          handleColumnChange(
                            index,
                            'scale',
                            e.target.value ? parseInt(e.target.value) : undefined
                          )
                        }
                        className="input"
                        placeholder="2"
                        min="0"
                      />
                    </div>
                  </>
                )}

                {/* Default Value */}
                <div>
                  <label className="block text-xs font-medium text-dark-400 mb-1">
                    Default Value
                  </label>
                  <input
                    type="text"
                    value={column.defaultValue || ''}
                    onChange={(e) =>
                      handleColumnChange(index, 'defaultValue', e.target.value || undefined)
                    }
                    className="input"
                    placeholder="NULL"
                  />
                </div>
              </div>

              {/* Checkboxes */}
              <div className="flex items-center gap-4">
                <label className="flex items-center gap-2 text-sm text-dark-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={column.nullable !== false}
                    onChange={(e) =>
                      handleColumnChange(index, 'nullable', e.target.checked)
                    }
                    className="rounded border-dark-600 bg-dark-800 text-rusty-500 focus:ring-rusty-500 focus:ring-offset-dark-900"
                  />
                  <span>Nullable</span>
                </label>
              </div>

              {/* Comment */}
              <div>
                <label className="block text-xs font-medium text-dark-400 mb-1">
                  Comment
                </label>
                <input
                  type="text"
                  value={column.comment || ''}
                  onChange={(e) =>
                    handleColumnChange(index, 'comment', e.target.value || undefined)
                  }
                  className="input"
                  placeholder="Optional description"
                />
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>

      {/* Add Column Button */}
      <button
        onClick={addColumn}
        className="btn-secondary w-full flex items-center justify-center gap-2"
      >
        <PlusIcon className="w-4 h-4" />
        Add Column
      </button>

      {/* Validation Summary */}
      {Object.keys(errors).length > 0 && (
        <div className="bg-danger-500/10 border border-danger-500/20 rounded-lg p-4">
          <p className="text-sm text-danger-400 font-medium mb-2">
            Please fix the following errors:
          </p>
          <ul className="list-disc list-inside space-y-1">
            {Object.entries(errors).map(([index, error]) => (
              <li key={index} className="text-sm text-danger-400">
                Column {parseInt(index) + 1}: {error}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
