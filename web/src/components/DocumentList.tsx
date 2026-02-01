import React from 'react';
import { Document } from '../types';

interface DocumentListProps {
  documents: Document[];
}

const DocumentList: React.FC<DocumentListProps> = ({ documents }) => {
  if (documents.length === 0) {
    return (
      <div className="empty-state">
        <p>No documents loaded yet</p>
      </div>
    );
  }

  return (
    <div className="document-list">
      {documents.map((doc) => (
        <div key={doc.id} className="document-item">
          <div className="document-icon">
            {doc.name.endsWith('.xml') ? '📄' : '🌐'}
          </div>
          <div className="document-info">
            <div className="document-name">{doc.name}</div>
            <div className="document-meta">
              ID: {doc.id} • {new Date(doc.created_at).toLocaleString()}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
};

export default DocumentList;
