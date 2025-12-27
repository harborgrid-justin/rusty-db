import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useQueryStore } from '../stores/queryStore';
import {
  useQueryExecution,
  useQueryHistory,
  useSavedQueries,
  useExplainPlan,
  useSqlFormatter,
} from '../hooks/useQuery';
import SqlEditor from '../components/query/SqlEditor';
import ResultsTable from '../components/query/ResultsTable';
import ExplainPlanViewer from '../components/query/ExplainPlanViewer';
import QueryTabs from '../components/query/QueryTabs';

type SidebarView = 'history' | 'saved' | null;
type ResultView = 'results' | 'explain' | null;

export const QueryEditor: React.FC = () => {
  const {
    tabs,
    activeTabId,
    addTab,
    closeTab,
    setActiveTab,
    updateTab,
    reorderTabs,
  } = useQueryStore();

  const [sidebarView, setSidebarView] = useState<SidebarView>('saved');
  const [resultView, setResultView] = useState<ResultView>('results');
  const [sidebarWidth, setSidebarWidth] = useState(300);
  const [isResizing, setIsResizing] = useState(false);

  // Get active tab
  const activeTab = useMemo(
    () => tabs.find(tab => tab.id === activeTabId),
    [tabs, activeTabId]
  );

  // Hooks for active tab
  const { executeQuery, cancelQuery, isExecuting } = useQueryExecution(activeTabId || '');
  const { formatSql, isFormatting } = useSqlFormatter();
  const { explainPlan, explain } = useExplainPlan();
  const {
    history,
    isLoading: historyLoading,
    searchTerm,
    setSearchTerm,
    clearHistory,
  } = useQueryHistory();
  const {
    savedQueries,
    isLoading: savedLoading,
    saveQuery,
    deleteQuery,
  } = useSavedQueries();

  // Initialize with one tab if empty
  useEffect(() => {
    if (tabs.length === 0) {
      addTab();
    }
  }, [tabs.length, addTab]);

  // Handle SQL change
  const handleSqlChange = useCallback(
    (sql: string) => {
      if (activeTabId) {
        updateTab(activeTabId, { sql });
      }
    },
    [activeTabId, updateTab]
  );

  // Handle query execution
  const handleExecute = useCallback(async () => {
    if (!activeTab || !activeTab.sql.trim()) return;

    await executeQuery(activeTab.sql);
    setResultView('results');
  }, [activeTab, executeQuery]);

  // Handle explain
  const handleExplain = useCallback(async () => {
    if (!activeTab || !activeTab.sql.trim()) return;

    try {
      await explain(activeTab.sql);
      setResultView('explain');
    } catch (error) {
      console.error('Explain failed:', error);
    }
  }, [activeTab, explain]);

  // Handle format
  const handleFormat = useCallback(async () => {
    if (!activeTab || !activeTab.sql.trim()) return;

    try {
      const formatted = await formatSql(activeTab.sql);
      updateTab(activeTabId!, { sql: formatted });
    } catch (error) {
      console.error('Format failed:', error);
    }
  }, [activeTab, activeTabId, formatSql, updateTab]);

  // Handle save query
  const handleSaveQuery = useCallback(async () => {
    if (!activeTab || !activeTab.sql.trim()) return;

    const name = prompt('Enter query name:');
    if (!name) return;

    try {
      await saveQuery(name, activeTab.sql);
      alert('Query saved successfully!');
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Unknown error';
      alert(`Failed to save query: ${message}`);
    }
  }, [activeTab, saveQuery]);

  // Handle load saved query
  const handleLoadSavedQuery = useCallback(
    (sql: string, name: string) => {
      const tabId = addTab();
      updateTab(tabId, { sql, title: name });
    },
    [addTab, updateTab]
  );

  // Handle load from history
  const handleLoadHistory = useCallback(
    (sql: string) => {
      if (activeTabId) {
        updateTab(activeTabId, { sql });
      }
    },
    [activeTabId, updateTab]
  );

  // Handle cancel query
  const handleCancel = useCallback(() => {
    if (activeTab?.queryId) {
      cancelQuery(activeTab.queryId);
    }
  }, [activeTab, cancelQuery]);

  // Sidebar resize handlers
  const startResize = useCallback(() => {
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = window.innerWidth - e.clientX;
      setSidebarWidth(Math.max(200, Math.min(600, newWidth)));
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing]);

  return (
    <div className="query-editor-page" style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Tabs */}
      <QueryTabs
        tabs={tabs}
        activeTabId={activeTabId}
        onTabClick={setActiveTab}
        onTabClose={closeTab}
        onTabAdd={addTab}
        onTabReorder={reorderTabs}
      />

      {/* Main Content */}
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
        {/* Editor and Results */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
          {/* Toolbar */}
          <div
            style={{
              padding: '8px 12px',
              background: '#252526',
              borderBottom: '1px solid #1e1e1e',
              display: 'flex',
              gap: '8px',
              alignItems: 'center',
            }}
          >
            <button
              onClick={handleExecute}
              disabled={!activeTab || isExecuting || !activeTab.sql.trim()}
              style={{
                padding: '6px 16px',
                background: '#0e639c',
                border: 'none',
                color: '#fff',
                cursor: !activeTab || isExecuting || !activeTab.sql.trim() ? 'not-allowed' : 'pointer',
                fontSize: '13px',
                fontWeight: 'bold',
                opacity: !activeTab || isExecuting || !activeTab.sql.trim() ? 0.5 : 1,
              }}
              title="Execute query (Ctrl+Enter)"
            >
              {isExecuting ? 'Executing...' : 'Execute'}
            </button>

            {isExecuting && (
              <button
                onClick={handleCancel}
                style={{
                  padding: '6px 16px',
                  background: '#a1260d',
                  border: 'none',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                Cancel
              </button>
            )}

            <button
              onClick={handleExplain}
              disabled={!activeTab || isExecuting || !activeTab.sql.trim()}
              style={{
                padding: '6px 16px',
                background: '#2d2d2d',
                border: '1px solid #444',
                color: '#fff',
                cursor: !activeTab || isExecuting || !activeTab.sql.trim() ? 'not-allowed' : 'pointer',
                fontSize: '13px',
                opacity: !activeTab || isExecuting || !activeTab.sql.trim() ? 0.5 : 1,
              }}
              title="Explain query execution plan"
            >
              Explain
            </button>

            <button
              onClick={handleFormat}
              disabled={!activeTab || isFormatting || !activeTab.sql.trim()}
              style={{
                padding: '6px 16px',
                background: '#2d2d2d',
                border: '1px solid #444',
                color: '#fff',
                cursor: !activeTab || isFormatting || !activeTab.sql.trim() ? 'not-allowed' : 'pointer',
                fontSize: '13px',
                opacity: !activeTab || isFormatting || !activeTab.sql.trim() ? 0.5 : 1,
              }}
              title="Format SQL (Ctrl+Shift+F)"
            >
              {isFormatting ? 'Formatting...' : 'Format'}
            </button>

            <div style={{ flex: 1 }} />

            <button
              onClick={handleSaveQuery}
              disabled={!activeTab || !activeTab.sql.trim()}
              style={{
                padding: '6px 16px',
                background: '#2d2d2d',
                border: '1px solid #444',
                color: '#fff',
                cursor: !activeTab || !activeTab.sql.trim() ? 'not-allowed' : 'pointer',
                fontSize: '13px',
                opacity: !activeTab || !activeTab.sql.trim() ? 0.5 : 1,
              }}
            >
              Save Query
            </button>
          </div>

          {/* Editor */}
          <div style={{ height: '50%', borderBottom: '1px solid #1e1e1e' }}>
            {activeTab ? (
              <SqlEditor
                value={activeTab.sql}
                onChange={handleSqlChange}
                onExecute={handleExecute}
                onFormat={handleFormat}
                height="100%"
              />
            ) : (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  height: '100%',
                  color: '#888',
                }}
              >
                No active tab
              </div>
            )}
          </div>

          {/* Results */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
            {/* Results Toolbar */}
            <div
              style={{
                padding: '8px 12px',
                background: '#252526',
                borderBottom: '1px solid #1e1e1e',
                display: 'flex',
                gap: '8px',
              }}
            >
              <button
                onClick={() => setResultView('results')}
                style={{
                  padding: '6px 12px',
                  background: resultView === 'results' ? '#0e639c' : 'transparent',
                  border: 'none',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                Results
              </button>
              <button
                onClick={() => setResultView('explain')}
                style={{
                  padding: '6px 12px',
                  background: resultView === 'explain' ? '#0e639c' : 'transparent',
                  border: 'none',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                Explain Plan
              </button>
            </div>

            {/* Results Content */}
            <div style={{ flex: 1, overflow: 'hidden' }}>
              {resultView === 'results' ? (
                activeTab?.result ? (
                  <ResultsTable result={activeTab.result} />
                ) : activeTab?.error ? (
                  <div style={{ padding: '20px', color: '#f48771', fontFamily: 'monospace' }}>
                    <strong>Error:</strong>
                    <pre style={{ marginTop: '8px', whiteSpace: 'pre-wrap' }}>{activeTab.error}</pre>
                  </div>
                ) : (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      height: '100%',
                      color: '#888',
                    }}
                  >
                    Execute a query to see results
                  </div>
                )
              ) : resultView === 'explain' ? (
                explainPlan ? (
                  <ExplainPlanViewer plan={explainPlan} />
                ) : (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      height: '100%',
                      color: '#888',
                    }}
                  >
                    Click Explain to see query execution plan
                  </div>
                )
              ) : null}
            </div>
          </div>
        </div>

        {/* Resize Handle */}
        {sidebarView && (
          <div
            onMouseDown={startResize}
            style={{
              width: '4px',
              background: '#1e1e1e',
              cursor: 'col-resize',
              userSelect: 'none',
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = '#007acc';
            }}
            onMouseLeave={(e) => {
              if (!isResizing) {
                e.currentTarget.style.background = '#1e1e1e';
              }
            }}
          />
        )}

        {/* Sidebar */}
        {sidebarView && (
          <div
            style={{
              width: `${sidebarWidth}px`,
              background: '#252526',
              borderLeft: '1px solid #1e1e1e',
              display: 'flex',
              flexDirection: 'column',
            }}
          >
            {/* Sidebar Header */}
            <div
              style={{
                padding: '8px 12px',
                background: '#2d2d2d',
                borderBottom: '1px solid #1e1e1e',
                display: 'flex',
                gap: '8px',
              }}
            >
              <button
                onClick={() => setSidebarView(sidebarView === 'saved' ? null : 'saved')}
                style={{
                  padding: '6px 12px',
                  background: sidebarView === 'saved' ? '#0e639c' : 'transparent',
                  border: 'none',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                Saved Queries
              </button>
              <button
                onClick={() => setSidebarView(sidebarView === 'history' ? null : 'history')}
                style={{
                  padding: '6px 12px',
                  background: sidebarView === 'history' ? '#0e639c' : 'transparent',
                  border: 'none',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: '13px',
                }}
              >
                History
              </button>
              <button
                onClick={() => setSidebarView(null)}
                style={{
                  marginLeft: 'auto',
                  padding: '6px 12px',
                  background: 'transparent',
                  border: 'none',
                  color: '#888',
                  cursor: 'pointer',
                  fontSize: '16px',
                }}
              >
                ×
              </button>
            </div>

            {/* Sidebar Content */}
            <div style={{ flex: 1, overflow: 'auto', padding: '12px' }}>
              {sidebarView === 'saved' && (
                <div>
                  {savedLoading ? (
                    <div style={{ color: '#888', fontSize: '13px' }}>Loading...</div>
                  ) : savedQueries.length === 0 ? (
                    <div style={{ color: '#888', fontSize: '13px' }}>No saved queries</div>
                  ) : (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                      {savedQueries.map(query => (
                        <div
                          key={query.id}
                          style={{
                            padding: '12px',
                            background: '#1e1e1e',
                            border: '1px solid #333',
                            borderRadius: '4px',
                            cursor: 'pointer',
                          }}
                          onClick={() => handleLoadSavedQuery(query.sql, query.name)}
                        >
                          <div
                            style={{
                              display: 'flex',
                              justifyContent: 'space-between',
                              alignItems: 'center',
                              marginBottom: '4px',
                            }}
                          >
                            <strong style={{ fontSize: '13px' }}>{query.name}</strong>
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                if (confirm(`Delete "${query.name}"?`)) {
                                  deleteQuery(query.id);
                                }
                              }}
                              style={{
                                background: 'none',
                                border: 'none',
                                color: '#888',
                                cursor: 'pointer',
                                fontSize: '16px',
                              }}
                            >
                              ×
                            </button>
                          </div>
                          {query.description && (
                            <div style={{ fontSize: '12px', color: '#888', marginBottom: '8px' }}>
                              {query.description}
                            </div>
                          )}
                          <pre
                            style={{
                              fontSize: '11px',
                              color: '#ce9178',
                              margin: 0,
                              whiteSpace: 'pre-wrap',
                              wordWrap: 'break-word',
                              maxHeight: '100px',
                              overflow: 'hidden',
                            }}
                          >
                            {query.sql}
                          </pre>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {sidebarView === 'history' && (
                <div>
                  <input
                    type="text"
                    placeholder="Search history..."
                    value={searchTerm}
                    onChange={(e) => setSearchTerm(e.target.value)}
                    style={{
                      width: '100%',
                      padding: '8px',
                      background: '#1e1e1e',
                      border: '1px solid #333',
                      color: '#fff',
                      marginBottom: '12px',
                      fontSize: '13px',
                    }}
                  />

                  {historyLoading ? (
                    <div style={{ color: '#888', fontSize: '13px' }}>Loading...</div>
                  ) : history.length === 0 ? (
                    <div style={{ color: '#888', fontSize: '13px' }}>No query history</div>
                  ) : (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                      {history.map(item => (
                        <div
                          key={item.id}
                          style={{
                            padding: '12px',
                            background: '#1e1e1e',
                            border: item.error ? '1px solid #f48771' : '1px solid #333',
                            borderRadius: '4px',
                            cursor: 'pointer',
                          }}
                          onClick={() => handleLoadHistory(item.sql)}
                        >
                          <div style={{ fontSize: '11px', color: '#888', marginBottom: '4px' }}>
                            {new Date(item.timestamp).toLocaleString()}
                            {' • '}
                            {item.executionTime}ms
                            {item.rowCount !== undefined && ` • ${item.rowCount} rows`}
                          </div>
                          <pre
                            style={{
                              fontSize: '11px',
                              color: item.error ? '#f48771' : '#ce9178',
                              margin: 0,
                              whiteSpace: 'pre-wrap',
                              wordWrap: 'break-word',
                              maxHeight: '80px',
                              overflow: 'hidden',
                            }}
                          >
                            {item.sql}
                          </pre>
                          {item.error && (
                            <div style={{ fontSize: '11px', color: '#f48771', marginTop: '4px' }}>
                              Error: {item.error}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}

                  {history.length > 0 && (
                    <button
                      onClick={() => {
                        if (confirm('Clear all history?')) {
                          clearHistory();
                        }
                      }}
                      style={{
                        marginTop: '12px',
                        padding: '6px 12px',
                        background: '#a1260d',
                        border: 'none',
                        color: '#fff',
                        cursor: 'pointer',
                        fontSize: '12px',
                        width: '100%',
                      }}
                    >
                      Clear History
                    </button>
                  )}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Sidebar Toggle (when hidden) */}
        {!sidebarView && (
          <div
            style={{
              position: 'absolute',
              right: 0,
              top: '50%',
              transform: 'translateY(-50%)',
              display: 'flex',
              flexDirection: 'column',
              gap: '4px',
            }}
          >
            <button
              onClick={() => setSidebarView('saved')}
              style={{
                padding: '8px',
                background: '#2d2d2d',
                border: '1px solid #444',
                borderRight: 'none',
                color: '#fff',
                cursor: 'pointer',
                fontSize: '11px',
                writingMode: 'vertical-rl',
              }}
              title="Show saved queries"
            >
              Saved
            </button>
            <button
              onClick={() => setSidebarView('history')}
              style={{
                padding: '8px',
                background: '#2d2d2d',
                border: '1px solid #444',
                borderRight: 'none',
                color: '#fff',
                cursor: 'pointer',
                fontSize: '11px',
                writingMode: 'vertical-rl',
              }}
              title="Show query history"
            >
              History
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default QueryEditor;
