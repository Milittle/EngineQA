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

const API_BASE_URL = (import.meta.env.VITE_API_BASE_URL || '').trim();

export class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl.replace(/\/+$/, '');
  }

  private buildUrl(path: string): string {
    if (!this.baseUrl) {
      return path;
    }
    if (path.startsWith('/')) {
      return `${this.baseUrl}${path}`;
    }
    return `${this.baseUrl}/${path}`;
  }

  private async request(path: string, init?: RequestInit): Promise<Response> {
    const url = this.buildUrl(path);

    try {
      return await fetch(url, init);
    } catch (err) {
      if (err instanceof TypeError) {
        throw new Error(
          `Network error: cannot reach backend at ${url}. Check backend process and VITE_API_BASE_URL.`
        );
      }
      throw err;
    }
  }

  async query(request: QueryRequest): Promise<QueryResponse> {
    const response = await this.request('/api/query', {
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
    const response = await this.request('/api/feedback', {
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
    const response = await this.request('/api/status');

    if (!response.ok) {
      throw new Error(`Status check failed: ${response.statusText}`);
    }

    return response.json();
  }

  async reindex(): Promise<ReindexResponse> {
    const response = await this.request('/api/reindex', {
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
    const response = await this.request('/api/reindex');

    if (!response.ok) {
      throw new Error(`Reindex status check failed: ${response.statusText}`);
    }

    return response.json();
  }

  async health(): Promise<{ status: string }> {
    const response = await this.request('/health');

    if (!response.ok) {
      throw new Error(`Health check failed: ${response.statusText}`);
    }

    return response.json();
  }
}

export const apiClient = new ApiClient();
