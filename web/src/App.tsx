import React, { useState, useEffect } from 'react';
import FileUploader from './components/FileUploader';
import DocumentList from './components/DocumentList';
import QueryEditor from './components/QueryEditor';
import ResultsViewer from './components/ResultsViewer';
import { Document, QueryResult } from './types';
import './App.css';

// This will be loaded from WASM
let XmlSqlDb: any = null;

function App() {
  const [db, setDb] = useState<any>(null);
  const [documents, setDocuments] = useState<Document[]>([]);
  const [queryResults, setQueryResults] = useState<QueryResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadWasm();
  }, []);

  const loadWasm = async () => {
    try {
      setLoading(true);
      // Import WASM module
      const wasm = await import('../public/wasm/xmlsql_wasm.js');
      // Add cache buster to force reload of WASM binary
      await wasm.default(`/wasm/xmlsql_wasm_bg.wasm?t=${Date.now()}`);

      XmlSqlDb = wasm.XmlSqlDb;

      // Initialize database
      const dbInstance = new XmlSqlDb();
      setDb(dbInstance);

      setLoading(false);
    } catch (err) {
      console.error('Failed to load WASM:', err);
      setError('Failed to initialize application. Please refresh the page.');
      setLoading(false);
    }
  };



  const handleExportDb = async () => {
    if (!db) return;
    try {
      setLoading(true);
      const data = await db.export_database();
      // data is Uint8Array
      const blob = new Blob([data], { type: 'application/x-sqlite3' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `xmlsql-${new Date().toISOString().slice(0, 10)}.db`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      setLoading(false);
    } catch (err: any) {
      console.error('Export failed:', err);
      setError('Failed to export database: ' + err.toString());
      setLoading(false);
    }
  };

  const handleImportDb = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!db || !e.target.files || e.target.files.length === 0) return;

    const file = e.target.files[0];
    try {
      setLoading(true);
      const buffer = await file.arrayBuffer();
      const data = new Uint8Array(buffer);
      await db.import_database(data);

      // Refresh documents after loading new DB
      await refreshDocuments();
      setLoading(false);

      // Reset input
      e.target.value = '';
    } catch (err: any) {
      console.error('Import failed:', err);
      setError('Failed to import database: ' + err.toString());
      setLoading(false);
    }
  };

  const handleFileLoad = async (content: string, filename: string, type: 'xml' | 'html') => {
    if (!db) {
      setError('Database not initialized');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      if (type === 'xml') {
        await db.load_xml(content, filename);
      } else {
        await db.load_html(content, filename);
      }

      // Refresh document list
      await refreshDocuments();

      setLoading(false);
    } catch (err: any) {
      console.error('File load error:', err);
      setError(`Failed to load ${type.toUpperCase()}: ${err.toString()}`);
      setLoading(false);
    }
  };

  const refreshDocuments = async () => {
    if (!db) return;

    try {
      const result = await db.get_documents();
      const docs: Document[] = result.rows.map((row: any[]) => ({
        id: row[0],
        name: row[1],
        created_at: row[2]
      }));
      setDocuments(docs);
    } catch (err) {
      console.error('Failed to refresh documents:', err);
    }
  };

  const handleQueryExecute = async (query: string, mode: 'css' | 'sql') => {
    if (!db) {
      setError('Database not initialized');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const result = mode === 'css'
        ? await db.query_selector(query)
        : await db.execute_sql(query);

      setQueryResults(result);
      setLoading(false);
    } catch (err: any) {
      console.error('Query execution error:', err);
      setError(`Query failed: ${err.toString()}`);
      setLoading(false);
    }
  };

  if (loading && !db) {
    return (
      <div className="app-container loading">
        <div className="loading-spinner">
          <div className="spinner"></div>
          <p>Loading application...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="app-container">
      <header className="app-header">
        <h1>XML/HTML to SQLite Query System</h1>
        <p>Parse XML/HTML documents and query them using CSS selectors or SQL</p>
      </header>

      {error && (
        <div className="error-banner">
          <span className="error-icon">⚠️</span>
          <span>{error}</span>
          <button onClick={() => setError(null)}>×</button>
        </div>
      )}

      <div className="main-content">
        <div className="sidebar">
          <section className="section">
            <h2>Load Document</h2>
            <FileUploader onFileLoad={handleFileLoad} disabled={loading} />

            <div className="db-actions" style={{ marginTop: '1rem', display: 'flex', gap: '0.5rem', flexDirection: 'column' }}>
              <h3>Database Storage</h3>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <button
                  onClick={handleExportDb}
                  className="action-button"
                  title="Save current database to file"
                  style={{ flex: 1, padding: '0.5rem' }}
                >
                  💾 Save DB
                </button>
                <label className="action-button" style={{ flex: 1, padding: '0.5rem', textAlign: 'center', cursor: 'pointer', backgroundColor: '#f0f0f0', border: '1px solid #ccc', borderRadius: '4px' }}>
                  📂 Load DB
                  <input
                    type="file"
                    accept=".db,.sqlite,.sqlite3"
                    style={{ display: 'none' }}
                    onChange={handleImportDb}
                  />
                </label>
              </div>
            </div>
          </section>

          <section className="section">
            <h2>Documents</h2>
            <DocumentList documents={documents} />
          </section>
        </div>

        <div className="content-area">
          <section className="section">
            <h2>Query</h2>
            <QueryEditor
              onExecute={handleQueryExecute}
              disabled={loading || documents.length === 0}
            />
          </section>

          {queryResults && (
            <section className="section">
              <h2>Results ({queryResults.rows.length} rows)</h2>
              <ResultsViewer results={queryResults} />
            </section>
          )}
        </div>
      </div>
    </div>
  );
}

export default App;
