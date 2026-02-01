import React, { useState } from 'react';
import { QueryResult, ExportFormat } from '../types';

interface ResultsViewerProps {
  results: QueryResult;
}

const ResultsViewer: React.FC<ResultsViewerProps> = ({ results }) => {
  const [viewMode, setViewMode] = useState<'table' | 'json'>('table');

  const exportResults = (format: ExportFormat) => {
    let content: string;
    let filename: string;
    let mimeType: string;

    switch (format) {
      case 'json':
        content = JSON.stringify(results, null, 2);
        filename = 'query-results.json';
        mimeType = 'application/json';
        break;
      
      case 'csv':
        content = convertToCSV(results);
        filename = 'query-results.csv';
        mimeType = 'text/csv';
        break;
      
      default:
        alert(`Export format ${format} not yet implemented`);
        return;
    }

    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  const convertToCSV = (results: QueryResult): string => {
    const headers = results.columns.join(',');
    const rows = results.rows.map(row => 
      row.map(cell => {
        const str = String(cell ?? '');
        // Escape quotes and wrap in quotes if contains comma or newline
        if (str.includes(',') || str.includes('\n') || str.includes('"')) {
          return `"${str.replace(/"/g, '""')}"`;
        }
        return str;
      }).join(',')
    ).join('\n');
    return `${headers}\n${rows}`;
  };

  if (results.rows.length === 0) {
    return (
      <div className="empty-results">
        <p>No results found</p>
      </div>
    );
  }

  return (
    <div className="results-viewer">
      <div className="results-toolbar">
        <div className="view-mode-selector">
          <button
            className={`view-button ${viewMode === 'table' ? 'active' : ''}`}
            onClick={() => setViewMode('table')}
          >
            📊 Table
          </button>
          <button
            className={`view-button ${viewMode === 'json' ? 'active' : ''}`}
            onClick={() => setViewMode('json')}
          >
            📋 JSON
          </button>
        </div>
        
        <div className="export-buttons">
          <button onClick={() => exportResults('json')} className="export-button">
            Export JSON
          </button>
          <button onClick={() => exportResults('csv')} className="export-button">
            Export CSV
          </button>
        </div>
      </div>

      {viewMode === 'table' ? (
        <div className="table-container">
          <table className="results-table">
            <thead>
              <tr>
                {results.columns.map((col, idx) => (
                  <th key={idx}>{col}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {results.rows.map((row, rowIdx) => (
                <tr key={rowIdx}>
                  {row.map((cell, cellIdx) => (
                    <td key={cellIdx}>
                      {cell === null ? <span className="null-value">NULL</span> : String(cell)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <pre className="json-viewer">
          {JSON.stringify(results, null, 2)}
        </pre>
      )}
    </div>
  );
};

export default ResultsViewer;
