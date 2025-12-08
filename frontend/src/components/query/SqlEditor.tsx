import React, { useRef, useCallback, useEffect } from 'react';
import Editor, { OnMount, Monaco } from '@monaco-editor/react';
import { editor } from 'monaco-editor';
import { useQueryStore } from '../../stores/queryStore';
import { useSchemaMetadata } from '../../hooks/useQuery';
import { format } from 'sql-formatter';

interface SqlEditorProps {
  value: string;
  onChange: (value: string) => void;
  onExecute?: () => void;
  onFormat?: () => void;
  readOnly?: boolean;
  height?: string;
}

export const SqlEditor: React.FC<SqlEditorProps> = ({
  value,
  onChange,
  onExecute,
  onFormat,
  readOnly = false,
  height = '400px',
}) => {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monacoRef = useRef<Monaco | null>(null);
  const { preferences } = useQueryStore();
  const { metadata } = useSchemaMetadata();

  const handleEditorDidMount: OnMount = useCallback(
    (editor, monaco) => {
      editorRef.current = editor;
      monacoRef.current = monaco;

      // Register SQL language configuration
      monaco.languages.setLanguageConfiguration('sql', {
        comments: {
          lineComment: '--',
          blockComment: ['/*', '*/'],
        },
        brackets: [
          ['{', '}'],
          ['[', ']'],
          ['(', ')'],
        ],
        autoClosingPairs: [
          { open: '{', close: '}' },
          { open: '[', close: ']' },
          { open: '(', close: ')' },
          { open: '"', close: '"' },
          { open: "'", close: "'" },
        ],
        surroundingPairs: [
          { open: '{', close: '}' },
          { open: '[', close: ']' },
          { open: '(', close: ')' },
          { open: '"', close: '"' },
          { open: "'", close: "'" },
        ],
      });

      // Add keyboard shortcuts
      editor.addAction({
        id: 'execute-query',
        label: 'Execute Query',
        keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
        run: () => {
          if (onExecute) {
            onExecute();
          }
        },
      });

      editor.addAction({
        id: 'format-sql',
        label: 'Format SQL',
        keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyF],
        run: () => {
          if (onFormat) {
            onFormat();
          } else {
            formatSql();
          }
        },
      });

      editor.addAction({
        id: 'comment-line',
        label: 'Toggle Line Comment',
        keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Slash],
        run: (ed) => {
          ed.trigger('keyboard', 'editor.action.commentLine', {});
        },
      });

      // Focus editor
      editor.focus();
    },
    [onExecute, onFormat]
  );

  // Register autocomplete provider
  useEffect(() => {
    if (!monacoRef.current || !metadata) return;

    const monaco = monacoRef.current;

    // Register completion provider
    const disposable = monaco.languages.registerCompletionItemProvider('sql', {
      provideCompletionItems: (model, position) => {
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
        };

        const suggestions: any[] = [];

        // Add SQL keywords
        const keywords = [
          'SELECT', 'FROM', 'WHERE', 'INSERT', 'UPDATE', 'DELETE', 'CREATE', 'DROP',
          'ALTER', 'TABLE', 'INDEX', 'VIEW', 'JOIN', 'INNER', 'LEFT', 'RIGHT', 'OUTER',
          'ON', 'AND', 'OR', 'NOT', 'IN', 'EXISTS', 'BETWEEN', 'LIKE', 'IS', 'NULL',
          'ORDER', 'BY', 'GROUP', 'HAVING', 'LIMIT', 'OFFSET', 'UNION', 'DISTINCT',
          'AS', 'CASE', 'WHEN', 'THEN', 'ELSE', 'END', 'COUNT', 'SUM', 'AVG', 'MIN', 'MAX',
        ];

        keywords.forEach((keyword) => {
          suggestions.push({
            label: keyword,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: keyword,
            range,
          });
        });

        // Add table names
        metadata.tables.forEach((table) => {
          suggestions.push({
            label: table.name,
            kind: monaco.languages.CompletionItemKind.Class,
            insertText: table.name,
            detail: `Table in ${table.schema}`,
            range,
          });

          // Add columns for each table
          table.columns.forEach((column) => {
            suggestions.push({
              label: `${table.name}.${column.name}`,
              kind: monaco.languages.CompletionItemKind.Field,
              insertText: `${table.name}.${column.name}`,
              detail: `${column.type} - ${table.name}`,
              range,
            });
          });
        });

        return { suggestions };
      },
    });

    return () => disposable.dispose();
  }, [metadata]);

  const formatSql = useCallback(() => {
    if (!editorRef.current) return;

    const currentValue = editorRef.current.getValue();
    if (!currentValue.trim()) return;

    try {
      const formatted = format(currentValue, {
        language: 'sql',
        uppercase: true,
        linesBetweenQueries: 2,
      });
      editorRef.current.setValue(formatted);
      onChange(formatted);
    } catch (error) {
      console.error('Failed to format SQL:', error);
    }
  }, [onChange]);

  const handleChange = useCallback(
    (newValue: string | undefined) => {
      onChange(newValue || '');
    },
    [onChange]
  );

  return (
    <div className="sql-editor-wrapper" style={{ height, border: '1px solid #333' }}>
      <Editor
        height="100%"
        defaultLanguage="sql"
        value={value}
        onChange={handleChange}
        onMount={handleEditorDidMount}
        theme={preferences.theme}
        options={{
          fontSize: preferences.fontSize,
          lineNumbers: preferences.lineNumbers ? 'on' : 'off',
          minimap: { enabled: preferences.minimap },
          wordWrap: preferences.wordWrap ? 'on' : 'off',
          readOnly,
          automaticLayout: true,
          scrollBeyondLastLine: false,
          formatOnPaste: true,
          formatOnType: preferences.autoComplete,
          suggestOnTriggerCharacters: preferences.autoComplete,
          quickSuggestions: preferences.autoComplete,
          tabSize: 2,
          insertSpaces: true,
          folding: true,
          foldingStrategy: 'indentation',
          showFoldingControls: 'always',
          rulers: [80, 120],
          renderWhitespace: 'selection',
          cursorBlinking: 'smooth',
          cursorSmoothCaretAnimation: 'on',
          smoothScrolling: true,
          contextmenu: true,
          mouseWheelZoom: true,
          links: true,
          colorDecorators: true,
          padding: { top: 10, bottom: 10 },
        }}
      />
    </div>
  );
};

export default SqlEditor;
