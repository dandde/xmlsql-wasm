import React, { useState } from 'react';
import { QueryMode } from '../types';

interface QueryEditorProps {
  onExecute: (query: string, mode: QueryMode) => void;
  disabled?: boolean;
}

const cssExamples = [
  { label: 'All divs', value: 'div' },
  { label: 'By class', value: '.container' },
  { label: 'By ID', value: '#main' },
  { label: 'Attribute', value: '[data-id]' },
  { label: 'Child combinator', value: 'div > p' },
  { label: 'Descendant', value: 'article p' },
  { label: 'Complex', value: 'div.container > p#intro[data-section="1"]' },
];

const sqlExamples = [
  { label: 'All nodes', value: 'SELECT * FROM nodes' },
  { label: 'By tag', value: "SELECT * FROM nodes WHERE tag_name = 'div'" },
  { label: 'With text', value: "SELECT * FROM nodes WHERE text_content IS NOT NULL" },
  { label: 'Count by tag', value: 'SELECT tag_name, COUNT(*) as count FROM nodes GROUP BY tag_name' },
  { label: 'Nodes with attrs', value: `SELECT DISTINCT n.* 
FROM nodes n 
JOIN attributes a ON a.node_id = n.id` },
];

const QueryEditor: React.FC<QueryEditorProps> = ({ onExecute, disabled }) => {
  const [mode, setMode] = useState<QueryMode>('css');
  const [query, setQuery] = useState('');

  const handleExecute = () => {
    if (query.trim()) {
      onExecute(query.trim(), mode);
    }
  };

  const handleExampleClick = (example: string) => {
    setQuery(example);
  };

  const examples = mode === 'css' ? cssExamples : sqlExamples;

  return (
    <div className="query-editor">
      <div className="mode-selector">
        <button
          className={`mode-button ${mode === 'css' ? 'active' : ''}`}
          onClick={() => setMode('css')}
        >
          CSS Selector
        </button>
        <button
          className={`mode-button ${mode === 'sql' ? 'active' : ''}`}
          onClick={() => setMode('sql')}
        >
          SQL Query
        </button>
      </div>

      <div className="examples">
        <span className="examples-label">Examples:</span>
        {examples.map((ex, idx) => (
          <button
            key={idx}
            className="example-button"
            onClick={() => handleExampleClick(ex.value)}
            disabled={disabled}
          >
            {ex.label}
          </button>
        ))}
      </div>

      <textarea
        className="query-input"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={
          mode === 'css'
            ? 'Enter CSS selector (e.g., div.container > p.intro)'
            : 'Enter SQL query (e.g., SELECT * FROM nodes WHERE tag_name = "div")'
        }
        rows={6}
        disabled={disabled}
      />

      <div className="editor-actions">
        <button
          className="execute-button"
          onClick={handleExecute}
          disabled={disabled || !query.trim()}
        >
          {mode === 'css' ? '🔍 Query Selector' : '▶️ Execute SQL'}
        </button>
        <button
          className="clear-button"
          onClick={() => setQuery('')}
          disabled={disabled || !query}
        >
          Clear
        </button>
      </div>
    </div>
  );
};

export default QueryEditor;
