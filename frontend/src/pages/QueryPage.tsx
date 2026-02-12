import { useState, FormEvent, useEffect } from 'react';
import { apiClient } from '../api/client';
import type { QueryResponse, QuerySource } from '../api/types';

interface HistoryItem {
  id: string;
  question: string;
  answer: string;
  timestamp: number;
  degraded: boolean;
}

export function QueryPage() {
  const [question, setQuestion] = useState('');
  const [response, setResponse] = useState<QueryResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();

    if (!question.trim()) {
      setError('Please enter a question');
      return;
    }

    setLoading(true);
    setError(null);
    setResponse(null);

    try {
      const result = await apiClient.query({ question });
      setResponse(result);

      // Save to history
      saveToHistory(question, result);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Query failed');
    } finally {
      setLoading(false);
    }
  };

  const saveToHistory = (question: string, response: QueryResponse) => {
    try {
      const historyItem: HistoryItem = {
        id: `${Date.now()}-${Math.random()}`,
        question,
        answer: response.answer,
        timestamp: Date.now(),
        degraded: response.degraded,
      };

      const existing = localStorage.getItem('qa_history');
      let history: HistoryItem[] = [];

      if (existing) {
        history = JSON.parse(existing);
      }

      // Add new item to beginning
      history.unshift(historyItem);

      // Keep only last 100 items
      if (history.length > 100) {
        history = history.slice(0, 100);
      }

      localStorage.setItem('qa_history', JSON.stringify(history));
    } catch (err) {
      console.error('Failed to save to history:', err);
    }
  };

  const handleFeedback = async (useful: boolean) => {
    if (!response) return;

    try {
      await apiClient.feedback({
        question,
        answer: response.answer,
        rating: useful ? 'useful' : 'useless',
        trace_id: response.trace_id,
        error_code: response.error_code,
      });
      alert('Feedback submitted. Thank you!');
    } catch (err) {
      alert('Failed to submit feedback');
    }
  };

  return (
    <div className="min-h-screen bg-gray-50 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <h1 className="text-3xl font-bold text-gray-900 mb-8 text-center">
          广告引擎维优问答系统
        </h1>

        <form onSubmit={handleSubmit} className="mb-8">
          <div className="flex gap-4">
            <input
              type="text"
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
              placeholder="请输入您的问题..."
              className="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none text-gray-900"
              disabled={loading}
            />
            <button
              type="submit"
              disabled={loading}
              className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed font-medium"
            >
              {loading ? '查询中...' : '提问'}
            </button>
          </div>
        </form>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6">
            {error}
          </div>
        )}

        {response && (
          <div className="space-y-6">
            {/* Answer Section */}
            <div
              className={`bg-white rounded-lg shadow-sm p-6 border ${
                response.degraded ? 'border-yellow-300' : 'border-gray-200'
              }`}
            >
              {response.degraded && response.error_code && (
                <div className="bg-yellow-50 border border-yellow-200 text-yellow-700 px-4 py-2 rounded-lg mb-4">
                  <strong>降级模式：</strong> {response.error_code}
                </div>
              )}

              <h2 className="text-xl font-semibold text-gray-900 mb-4">
                回答
              </h2>

              <div className="prose prose-sm max-w-none text-gray-700 whitespace-pre-wrap">
                {response.answer}
              </div>

              {/* Feedback Buttons */}
              <div className="mt-6 pt-4 border-t border-gray-200">
                <p className="text-sm text-gray-600 mb-3">
                  这个回答有帮助吗？
                </p>
                <div className="flex gap-3">
                  <button
                    onClick={() => handleFeedback(true)}
                    className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm font-medium"
                  >
                    👍 有用
                  </button>
                  <button
                    onClick={() => handleFeedback(false)}
                    className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 text-sm font-medium"
                  >
                    👎 无用
                  </button>
                </div>
              </div>
            </div>

            {/* Sources Section */}
            {response.sources.length > 0 && (
              <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200">
                <h2 className="text-xl font-semibold text-gray-900 mb-4">
                  参考来源 ({response.sources.length})
                </h2>

                <div className="space-y-4">
                  {response.sources.map((source: QuerySource, index: number) => (
                    <div
                      key={index}
                      className="border border-gray-200 rounded-lg p-4 hover:bg-gray-50 transition-colors"
                    >
                      <div className="flex justify-between items-start mb-2">
                        <h3 className="font-medium text-gray-900">
                          {source.title}
                        </h3>
                        <span className="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                          相关度: {(source.score * 100).toFixed(0)}%
                        </span>
                      </div>

                      <p className="text-sm text-gray-500 mb-3 font-mono">
                        {source.path}
                      </p>

                      <p className="text-sm text-gray-700 line-clamp-3">
                        {source.snippet}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Trace ID */}
            <div className="text-xs text-gray-400 text-center">
              Trace ID: {response.trace_id}
            </div>
          </div>
        )}

        {!response && !loading && !error && (
          <div className="text-center py-12 text-gray-500">
            <svg
              className="mx-auto h-16 w-16 text-gray-300 mb-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <p className="text-lg">开始提问，获取答案</p>
            <p className="text-sm mt-2">
              输入广告引擎维优相关的问题，系统将基于知识库为您解答
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
