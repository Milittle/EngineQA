import type {
  QueryRequest,
  QueryResponse,
  FeedbackRequest,
  FeedbackResponse,
  StatusResponse,
  ReindexRequest,
  ReindexResponse,
  ReindexStatusResponse,
} from './types';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:8080';

export class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  async query(request: QueryRequest): Promise<QueryResponse> {
    const response = await fetch(`${this.baseUrl}/api/query`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        question: request.question,
        top_k: request.top_k || 6,
      }),
    });

    if (!response.ok) {
      throw new Error(`Query failed: ${response.statusText}`);
    }

    return response.json();
  }

  async feedback(request: FeedbackRequest): Promise<FeedbackResponse> {
    const response = await fetch(`${this.baseUrl}/api/feedback`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`Feedback failed: ${response.statusText}`);
    }

    return response.json();
  }

  async status(): Promise<StatusResponse> {
    const response = await fetch(`${this.baseUrl}/api/status`);

    if (!response.ok) {
      throw new Error(`Status check failed: ${response.statusText}`);
    }

    return response.json();
  }

  async reindex(): Promise<ReindexResponse> {
    const response = await fetch(`${this.baseUrl}/api/reindex`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({}),
    });

    if (!response.ok) {
      throw new Error(`Reindex failed: ${response.statusText}`);
    }

    return response.json();
  }

  async reindexStatus(): Promise<ReindexStatusResponse> {
    const response = await fetch(`${this.baseUrl}/api/reindex`);

    if (!response.ok) {
      throw new Error(`Reindex status check failed: ${response.statusText}`);
    }

    return response.json();
  }

  async health(): Promise<{ status: string }> {
    const response = await fetch(`${this.baseUrl}/health`);

    if (!response.ok) {
      throw new Error(`Health check failed: ${response.statusText}`);
    }

    return response.json();
  }
}

export const apiClient = new ApiClient();
