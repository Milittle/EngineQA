export interface QueryRequest {
  question: string;
  top_k?: number;
}

export interface QuerySource {
  title: string;
  path: string;
  snippet: string;
  score: number;
}

export interface QueryResponse {
  answer: string;
  sources: QuerySource[];
  degraded: boolean;
  error_code?: string;
  trace_id: string;
}

export interface FeedbackRequest {
  question: string;
  answer: string;
  rating: 'useful' | 'useless';
  comment?: string;
  error_code?: string;
  trace_id: string;
}

export interface FeedbackResponse {
  ok: boolean;
  id: string;
}

export interface StatusResponse {
  provider: string;
  model: string;
  index_size: number;
  last_index_time?: string;
  upstream_health: 'ok' | 'degraded' | 'unavailable';
  rate_limit_state: {
    rpm_limit: number;
    current_rpm: number;
  };
  qdrant_connected: boolean;
}

export interface ReindexRequest {}

export interface ReindexResponse {
  job_id: string;
  message: string;
}

export interface JobStatus {
  running: boolean;
  completed: boolean;
  failed: boolean;
}

export interface JobInfo {
  job_id: string;
  status: 'running' | 'completed' | 'failed';
  started_at: string;
  ended_at?: string;
  result?: {
    total_files: number;
    indexed_files: number;
    skipped_files: number;
    failed_files: number;
    total_chunks: number;
    successful_chunks: number;
    failed_chunks: number;
    deleted_chunks: number;
    duration_ms: number;
  };
  error?: string;
}

export interface ReindexStatusResponse {
  job?: JobInfo;
}
