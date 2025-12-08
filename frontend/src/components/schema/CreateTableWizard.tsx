// ============================================================================
// Create Table Wizard Component
// Multi-step wizard for creating tables
// ============================================================================

import { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  CheckIcon,
  XMarkIcon,
} from '@heroicons/react/24/outline';
import { ColumnEditor } from './ColumnEditor';
import type { CreateTableRequest, ColumnDefinition, ForeignKeyDefinition, ConstraintDefinition } from '../../services/schemaService';
import clsx from 'clsx';

interface CreateTableWizardProps {
  onSubmit: (definition: CreateTableRequest) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

type Step = 'basic' | 'columns' | 'constraints' | 'review';

export function CreateTableWizard({
  onSubmit,
  onCancel,
  isSubmitting = false,
}: CreateTableWizardProps) {
  const [currentStep, setCurrentStep] = useState<Step>('basic');
  const [tableName, setTableName] = useState('');
  const [schema, setSchema] = useState('public');
  const [comment, setComment] = useState('');
  const [columns, setColumns] = useState<ColumnDefinition[]>([
    {
      name: 'id',
      dataType: 'integer',
      nullable: false,
    },
  ]);
  const [primaryKey, setPrimaryKey] = useState<string[]>(['id']);
  const [foreignKeys, setForeignKeys] = useState<ForeignKeyDefinition[]>([]);
  const [constraints, setConstraints] = useState<ConstraintDefinition[]>([]);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const steps: { id: Step; label: string }[] = [
    { id: 'basic', label: 'Basic Info' },
    { id: 'columns', label: 'Columns' },
    { id: 'constraints', label: 'Constraints' },
    { id: 'review', label: 'Review' },
  ];

  const currentStepIndex = steps.findIndex((s) => s.id === currentStep);

  const validateBasic = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!tableName.trim()) {
      newErrors.tableName = 'Table name is required';
    } else if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(tableName)) {
      newErrors.tableName = 'Invalid table name (must start with letter or underscore)';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const validateColumns = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (columns.length === 0) {
      newErrors.columns = 'At least one column is required';
    }

    const hasInvalidColumn = columns.some((col) => !col.name.trim());
    if (hasInvalidColumn) {
      newErrors.columns = 'All columns must have a name';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleNext = () => {
    let isValid = true;

    if (currentStep === 'basic') {
      isValid = validateBasic();
    } else if (currentStep === 'columns') {
      isValid = validateColumns();
    }

    if (!isValid) return;

    const nextIndex = currentStepIndex + 1;
    if (nextIndex < steps.length) {
      setCurrentStep(steps[nextIndex].id);
    }
  };

  const handlePrevious = () => {
    const prevIndex = currentStepIndex - 1;
    if (prevIndex >= 0) {
      setCurrentStep(steps[prevIndex].id);
    }
  };

  const handleSubmit = () => {
    const definition: CreateTableRequest = {
      name: tableName,
      schema,
      columns,
      primaryKey: primaryKey.length > 0 ? primaryKey : undefined,
      foreignKeys: foreignKeys.length > 0 ? foreignKeys : undefined,
      constraints: constraints.length > 0 ? constraints : undefined,
      comment: comment || undefined,
    };

    onSubmit(definition);
  };

  const addConstraint = () => {
    setConstraints([
      ...constraints,
      { type: 'unique', columns: [] },
    ]);
  };

  const removeConstraint = (index: number) => {
    setConstraints(constraints.filter((_, i) => i !== index));
  };

  const updateConstraint = (index: number, updates: Partial<ConstraintDefinition>) => {
    const newConstraints = [...constraints];
    newConstraints[index] = { ...newConstraints[index], ...updates };
    setConstraints(newConstraints);
  };

  const generateDDL = (): string => {
    let ddl = `CREATE TABLE ${schema}.${tableName} (\n`;

    ddl += columns
      .map((col) => {
        let colDef = `  ${col.name} ${col.dataType.toUpperCase()}`;
        if (col.length) colDef += `(${col.length})`;
        if (col.precision) colDef += `(${col.precision}${col.scale ? `, ${col.scale}` : ''})`;
        if (!col.nullable) colDef += ' NOT NULL';
        if (col.defaultValue) colDef += ` DEFAULT ${col.defaultValue}`;
        return colDef;
      })
      .join(',\n');

    if (primaryKey.length > 0) {
      ddl += `,\n  PRIMARY KEY (${primaryKey.join(', ')})`;
    }

    constraints.forEach((constraint) => {
      if (constraint.type === 'unique') {
        ddl += `,\n  UNIQUE (${constraint.columns.join(', ')})`;
      } else if (constraint.type === 'check' && constraint.expression) {
        ddl += `,\n  CHECK (${constraint.expression})`;
      }
    });

    ddl += '\n);';

    if (comment) {
      ddl += `\n\nCOMMENT ON TABLE ${schema}.${tableName} IS '${comment}';`;
    }

    return ddl;
  };

  return (
    <div className="card max-w-4xl mx-auto">
      {/* Header */}
      <div className="border-b border-dark-700 px-6 py-4">
        <h2 className="text-xl font-semibold text-dark-100">Create New Table</h2>
        <p className="text-sm text-dark-400 mt-1">
          Follow the steps to create a new database table
        </p>
      </div>

      {/* Steps Progress */}
      <div className="px-6 py-4 border-b border-dark-700">
        <div className="flex items-center justify-between">
          {steps.map((step, index) => (
            <div key={step.id} className="flex items-center flex-1">
              <div className="flex items-center">
                <div
                  className={clsx(
                    'w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium transition-colors',
                    index < currentStepIndex
                      ? 'bg-success-500 text-white'
                      : index === currentStepIndex
                      ? 'bg-rusty-500 text-white'
                      : 'bg-dark-700 text-dark-400'
                  )}
                >
                  {index < currentStepIndex ? (
                    <CheckIcon className="w-4 h-4" />
                  ) : (
                    index + 1
                  )}
                </div>
                <span
                  className={clsx(
                    'ml-2 text-sm font-medium',
                    index <= currentStepIndex ? 'text-dark-200' : 'text-dark-400'
                  )}
                >
                  {step.label}
                </span>
              </div>
              {index < steps.length - 1 && (
                <div
                  className={clsx(
                    'flex-1 h-0.5 mx-4',
                    index < currentStepIndex ? 'bg-success-500' : 'bg-dark-700'
                  )}
                />
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Step Content */}
      <div className="px-6 py-6 min-h-[400px]">
        <AnimatePresence mode="wait">
          {currentStep === 'basic' && (
            <motion.div
              key="basic"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Table Name *
                </label>
                <input
                  type="text"
                  value={tableName}
                  onChange={(e) => setTableName(e.target.value)}
                  className={clsx('input', errors.tableName && 'border-danger-500')}
                  placeholder="users"
                />
                {errors.tableName && (
                  <p className="text-sm text-danger-400 mt-1">{errors.tableName}</p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Schema
                </label>
                <input
                  type="text"
                  value={schema}
                  onChange={(e) => setSchema(e.target.value)}
                  className="input"
                  placeholder="public"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Comment
                </label>
                <textarea
                  value={comment}
                  onChange={(e) => setComment(e.target.value)}
                  className="input"
                  rows={3}
                  placeholder="Optional table description..."
                />
              </div>
            </motion.div>
          )}

          {currentStep === 'columns' && (
            <motion.div
              key="columns"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
            >
              <ColumnEditor columns={columns} onChange={setColumns} />
              {errors.columns && (
                <p className="text-sm text-danger-400 mt-2">{errors.columns}</p>
              )}
            </motion.div>
          )}

          {currentStep === 'constraints' && (
            <motion.div
              key="constraints"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              {/* Primary Key */}
              <div>
                <label className="block text-sm font-medium text-dark-300 mb-2">
                  Primary Key
                </label>
                <div className="flex flex-wrap gap-2">
                  {columns.map((col) => (
                    <label
                      key={col.name}
                      className="flex items-center gap-2 px-3 py-2 bg-dark-800 border border-dark-700 rounded cursor-pointer hover:bg-dark-700 transition-colors"
                    >
                      <input
                        type="checkbox"
                        checked={primaryKey.includes(col.name)}
                        onChange={(e) => {
                          if (e.target.checked) {
                            setPrimaryKey([...primaryKey, col.name]);
                          } else {
                            setPrimaryKey(primaryKey.filter((pk) => pk !== col.name));
                          }
                        }}
                        className="rounded border-dark-600 bg-dark-800 text-rusty-500"
                      />
                      <span className="text-sm text-dark-200">{col.name}</span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Unique Constraints */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <label className="block text-sm font-medium text-dark-300">
                    Unique Constraints
                  </label>
                  <button onClick={addConstraint} className="btn-secondary btn-sm">
                    Add Constraint
                  </button>
                </div>

                {constraints.length === 0 ? (
                  <p className="text-sm text-dark-400 italic">No constraints defined</p>
                ) : (
                  <div className="space-y-2">
                    {constraints.map((constraint, index) => (
                      <div key={index} className="card p-3 flex items-start gap-3">
                        <div className="flex-1 grid grid-cols-2 gap-3">
                          <div>
                            <label className="block text-xs text-dark-400 mb-1">
                              Type
                            </label>
                            <select
                              value={constraint.type}
                              onChange={(e) =>
                                updateConstraint(index, {
                                  type: e.target.value as ConstraintDefinition['type'],
                                })
                              }
                              className="input-sm"
                            >
                              <option value="unique">UNIQUE</option>
                              <option value="check">CHECK</option>
                            </select>
                          </div>

                          {constraint.type === 'check' ? (
                            <div>
                              <label className="block text-xs text-dark-400 mb-1">
                                Expression
                              </label>
                              <input
                                type="text"
                                value={constraint.expression || ''}
                                onChange={(e) =>
                                  updateConstraint(index, { expression: e.target.value })
                                }
                                className="input-sm"
                                placeholder="age >= 18"
                              />
                            </div>
                          ) : (
                            <div>
                              <label className="block text-xs text-dark-400 mb-1">
                                Columns
                              </label>
                              <select
                                multiple
                                value={constraint.columns}
                                onChange={(e) => {
                                  const selected = Array.from(
                                    e.target.selectedOptions,
                                    (option) => option.value
                                  );
                                  updateConstraint(index, { columns: selected });
                                }}
                                className="input-sm"
                              >
                                {columns.map((col) => (
                                  <option key={col.name} value={col.name}>
                                    {col.name}
                                  </option>
                                ))}
                              </select>
                            </div>
                          )}
                        </div>

                        <button
                          onClick={() => removeConstraint(index)}
                          className="p-1 rounded hover:bg-dark-700 text-danger-400"
                        >
                          <XMarkIcon className="w-4 h-4" />
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </motion.div>
          )}

          {currentStep === 'review' && (
            <motion.div
              key="review"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="space-y-4"
            >
              <div>
                <h3 className="text-sm font-medium text-dark-300 mb-2">
                  Table Definition
                </h3>
                <pre className="bg-dark-900 border border-dark-700 rounded-lg p-4 text-sm text-dark-200 overflow-x-auto">
                  {generateDDL()}
                </pre>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <h4 className="text-sm font-medium text-dark-400 mb-1">Schema</h4>
                  <p className="text-dark-200">{schema}</p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-dark-400 mb-1">Table Name</h4>
                  <p className="text-dark-200">{tableName}</p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-dark-400 mb-1">Columns</h4>
                  <p className="text-dark-200">{columns.length}</p>
                </div>
                <div>
                  <h4 className="text-sm font-medium text-dark-400 mb-1">
                    Primary Key
                  </h4>
                  <p className="text-dark-200">
                    {primaryKey.length > 0 ? primaryKey.join(', ') : 'None'}
                  </p>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Footer Actions */}
      <div className="border-t border-dark-700 px-6 py-4 flex items-center justify-between">
        <button onClick={onCancel} className="btn-secondary" disabled={isSubmitting}>
          Cancel
        </button>

        <div className="flex items-center gap-2">
          {currentStepIndex > 0 && (
            <button
              onClick={handlePrevious}
              className="btn-secondary flex items-center gap-2"
              disabled={isSubmitting}
            >
              <ChevronLeftIcon className="w-4 h-4" />
              Previous
            </button>
          )}

          {currentStepIndex < steps.length - 1 ? (
            <button
              onClick={handleNext}
              className="btn-primary flex items-center gap-2"
            >
              Next
              <ChevronRightIcon className="w-4 h-4" />
            </button>
          ) : (
            <button
              onClick={handleSubmit}
              className="btn-primary flex items-center gap-2"
              disabled={isSubmitting}
            >
              {isSubmitting ? (
                <>
                  <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin" />
                  Creating...
                </>
              ) : (
                <>
                  <CheckIcon className="w-4 h-4" />
                  Create Table
                </>
              )}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
