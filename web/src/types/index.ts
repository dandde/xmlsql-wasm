export interface NodeData {
  id: number;
  tag_name: string;
  text_content: string | null;
  attributes: Record<string, string>;
  parent_id: number | null;
  depth: number;
}

export interface QueryResult {
  columns: string[];
  rows: any[][];
}

export interface Document {
  id: number;
  name: string;
  created_at: string;
}

export type QueryMode = 'css' | 'sql';

export type ExportFormat = 'json' | 'csv' | 'sqlite';
