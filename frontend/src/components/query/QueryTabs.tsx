import React, { useState, useRef, useCallback } from 'react';
import { QueryTab } from '../../stores/queryStore';

interface QueryTabsProps {
  tabs: QueryTab[];
  activeTabId: string | null;
  onTabClick: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onTabAdd: () => void;
  onTabReorder?: (fromIndex: number, toIndex: number) => void;
}

interface TabContextMenuProps {
  tab: QueryTab;
  x: number;
  y: number;
  onClose: () => void;
  onCloseTab: (tabId: string) => void;
  onCloseOthers: (tabId: string) => void;
  onCloseAll: () => void;
  onCloseRight: (tabId: string) => void;
}

const TabContextMenu: React.FC<TabContextMenuProps> = ({
  tab,
  x,
  y,
  onClose,
  onCloseTab,
  onCloseOthers,
  onCloseAll,
  onCloseRight,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  React.useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [onClose]);

  const menuItems = [
    {
      label: 'Close',
      onClick: () => {
        onCloseTab(tab.id);
        onClose();
      },
    },
    {
      label: 'Close Others',
      onClick: () => {
        onCloseOthers(tab.id);
        onClose();
      },
    },
    {
      label: 'Close to the Right',
      onClick: () => {
        onCloseRight(tab.id);
        onClose();
      },
    },
    {
      label: 'Close All',
      onClick: () => {
        onCloseAll();
        onClose();
      },
    },
  ];

  return (
    <div
      ref={menuRef}
      style={{
        position: 'fixed',
        left: x,
        top: y,
        background: '#252526',
        border: '1px solid #454545',
        borderRadius: '4px',
        boxShadow: '0 2px 8px rgba(0, 0, 0, 0.5)',
        zIndex: 1000,
        minWidth: '180px',
      }}
    >
      {menuItems.map((item, index) => (
        <div
          key={index}
          onClick={item.onClick}
          style={{
            padding: '8px 16px',
            cursor: 'pointer',
            fontSize: '13px',
            color: '#cccccc',
            borderBottom: index < menuItems.length - 1 ? '1px solid #333' : 'none',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.background = '#2a2d2e';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.background = 'transparent';
          }}
        >
          {item.label}
        </div>
      ))}
    </div>
  );
};

export const QueryTabs: React.FC<QueryTabsProps> = ({
  tabs,
  activeTabId,
  onTabClick,
  onTabClose,
  onTabAdd,
  onTabReorder,
}) => {
  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const [dropTargetIndex, setDropTargetIndex] = useState<number | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    tab: QueryTab;
    x: number;
    y: number;
  } | null>(null);

  const handleDragStart = useCallback((e: React.DragEvent, index: number) => {
    setDraggedIndex(index);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', index.toString());
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent, index: number) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDropTargetIndex(index);
  }, []);

  const handleDragEnd = useCallback(() => {
    setDraggedIndex(null);
    setDropTargetIndex(null);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent, toIndex: number) => {
      e.preventDefault();
      const fromIndex = parseInt(e.dataTransfer.getData('text/plain'));

      if (fromIndex !== toIndex && onTabReorder) {
        onTabReorder(fromIndex, toIndex);
      }

      setDraggedIndex(null);
      setDropTargetIndex(null);
    },
    [onTabReorder]
  );

  const handleContextMenu = useCallback((e: React.MouseEvent, tab: QueryTab) => {
    e.preventDefault();
    setContextMenu({ tab, x: e.clientX, y: e.clientY });
  }, []);

  const handleCloseTab = useCallback(
    (tabId: string) => {
      const tab = tabs.find(t => t.id === tabId);
      if (tab?.isDirty) {
        if (confirm('This tab has unsaved changes. Close anyway?')) {
          onTabClose(tabId);
        }
      } else {
        onTabClose(tabId);
      }
    },
    [tabs, onTabClose]
  );

  const handleCloseOthers = useCallback(
    (keepTabId: string) => {
      const hasUnsaved = tabs.some(tab => tab.id !== keepTabId && tab.isDirty);
      if (hasUnsaved) {
        if (!confirm('Some tabs have unsaved changes. Close them anyway?')) {
          return;
        }
      }
      tabs.forEach(tab => {
        if (tab.id !== keepTabId) {
          onTabClose(tab.id);
        }
      });
    },
    [tabs, onTabClose]
  );

  const handleCloseAll = useCallback(() => {
    const hasUnsaved = tabs.some(tab => tab.isDirty);
    if (hasUnsaved) {
      if (!confirm('Some tabs have unsaved changes. Close all anyway?')) {
        return;
      }
    }
    tabs.forEach(tab => onTabClose(tab.id));
  }, [tabs, onTabClose]);

  const handleCloseRight = useCallback(
    (fromTabId: string) => {
      const fromIndex = tabs.findIndex(tab => tab.id === fromTabId);
      const tabsToClose = tabs.slice(fromIndex + 1);
      const hasUnsaved = tabsToClose.some(tab => tab.isDirty);

      if (hasUnsaved) {
        if (!confirm('Some tabs have unsaved changes. Close them anyway?')) {
          return;
        }
      }

      tabsToClose.forEach(tab => onTabClose(tab.id));
    },
    [tabs, onTabClose]
  );

  return (
    <>
      <div
        className="query-tabs"
        style={{
          display: 'flex',
          alignItems: 'center',
          background: '#2d2d2d',
          borderBottom: '1px solid #1e1e1e',
          overflowX: 'auto',
          overflowY: 'hidden',
          height: '36px',
        }}
      >
        {tabs.map((tab, index) => {
          const isActive = tab.id === activeTabId;
          const isDragging = draggedIndex === index;
          const isDropTarget = dropTargetIndex === index;

          return (
            <div
              key={tab.id}
              draggable
              onDragStart={(e) => handleDragStart(e, index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDragEnd={handleDragEnd}
              onDrop={(e) => handleDrop(e, index)}
              onClick={() => onTabClick(tab.id)}
              onContextMenu={(e) => handleContextMenu(e, tab)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                padding: '8px 12px',
                background: isActive ? '#1e1e1e' : 'transparent',
                borderRight: '1px solid #252526',
                cursor: 'pointer',
                opacity: isDragging ? 0.5 : 1,
                borderTop: isActive ? '2px solid #007acc' : '2px solid transparent',
                borderLeft: isDropTarget ? '2px solid #007acc' : 'none',
                minWidth: '120px',
                maxWidth: '200px',
                position: 'relative',
              }}
            >
              {/* Status indicator */}
              {tab.isExecuting && (
                <div
                  style={{
                    width: '8px',
                    height: '8px',
                    borderRadius: '50%',
                    background: '#4ec9b0',
                    animation: 'pulse 1.5s ease-in-out infinite',
                  }}
                  title="Executing query..."
                />
              )}

              {/* Title */}
              <span
                style={{
                  flex: 1,
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  fontSize: '13px',
                  color: isActive ? '#ffffff' : '#cccccc',
                }}
                title={tab.title}
              >
                {tab.title}
                {tab.isDirty && ' •'}
              </span>

              {/* Close button */}
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleCloseTab(tab.id);
                }}
                style={{
                  background: 'none',
                  border: 'none',
                  color: '#888',
                  cursor: 'pointer',
                  padding: '2px',
                  display: 'flex',
                  alignItems: 'center',
                  fontSize: '16px',
                  lineHeight: 1,
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.color = '#fff';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.color = '#888';
                }}
                title="Close tab"
              >
                ×
              </button>
            </div>
          );
        })}

        {/* Add tab button */}
        <button
          onClick={onTabAdd}
          style={{
            background: 'none',
            border: 'none',
            color: '#888',
            cursor: 'pointer',
            padding: '8px 16px',
            fontSize: '16px',
            lineHeight: 1,
            minWidth: '40px',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.color = '#fff';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.color = '#888';
          }}
          title="New query tab"
        >
          +
        </button>
      </div>

      {/* Context menu */}
      {contextMenu && (
        <TabContextMenu
          tab={contextMenu.tab}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          onCloseTab={handleCloseTab}
          onCloseOthers={handleCloseOthers}
          onCloseAll={handleCloseAll}
          onCloseRight={handleCloseRight}
        />
      )}

      <style>{`
        @keyframes pulse {
          0%, 100% {
            opacity: 1;
          }
          50% {
            opacity: 0.5;
          }
        }

        .query-tabs::-webkit-scrollbar {
          height: 4px;
        }

        .query-tabs::-webkit-scrollbar-track {
          background: #1e1e1e;
        }

        .query-tabs::-webkit-scrollbar-thumb {
          background: #424242;
          border-radius: 2px;
        }

        .query-tabs::-webkit-scrollbar-thumb:hover {
          background: #4e4e4e;
        }
      `}</style>
    </>
  );
};

export default QueryTabs;
