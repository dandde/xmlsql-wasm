import React, { useRef } from 'react';

interface FileUploaderProps {
  onFileLoad: (content: string, filename: string, type: 'xml' | 'html') => void;
  disabled?: boolean;
}

const FileUploader: React.FC<FileUploaderProps> = ({ onFileLoad, disabled }) => {
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const content = await file.text();
      const type = file.name.match(/\.html?$/i) ? 'html' : 'xml';
      onFileLoad(content, file.name, type);
      
      // Reset input
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    } catch (err) {
      console.error('File read error:', err);
      alert('Failed to read file');
    }
  };

  const handleDrop = async (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();

    const file = event.dataTransfer.files[0];
    if (!file) return;

    try {
      const content = await file.text();
      const type = file.name.match(/\.html?$/i) ? 'html' : 'xml';
      onFileLoad(content, file.name, type);
    } catch (err) {
      console.error('File read error:', err);
      alert('Failed to read file');
    }
  };

  const handleDragOver = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();
  };

  return (
    <div className="file-uploader">
      <div
        className={`drop-zone ${disabled ? 'disabled' : ''}`}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onClick={() => !disabled && fileInputRef.current?.click()}
      >
        <div className="drop-zone-content">
          <svg className="upload-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
          </svg>
          <p className="drop-zone-text">
            Drop XML/HTML file here or click to browse
          </p>
          <p className="drop-zone-hint">
            Supports .xml, .html files
          </p>
        </div>
      </div>
      <input
        ref={fileInputRef}
        type="file"
        accept=".xml,.html,.htm"
        onChange={handleFileChange}
        disabled={disabled}
        style={{ display: 'none' }}
      />
    </div>
  );
};

export default FileUploader;
