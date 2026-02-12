import { useState, useEffect } from 'react';

interface HistoryItem {
  id: string;
  question: string;
  answer: string;
  timestamp: number;
  degraded: boolean;
}

export function HistoryPage() {
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [selectedItem, setSelectedItem] = useState<HistoryItem | null>(null);

  useEffect(() => {
    loadHistory();
  }, []);

  const loadHistory = () => {
    try {
      const saved = localStorage.getItem('qa_history');
      if (saved) {
        setHistory(JSON.parse(saved));
      }
    } catch (err) {
      console.error('Failed to load history:', err);
    }
  };

  const clearHistory = () => {
    if (!confirm('确定要清空历史记录吗？')) {
      return;
    }
    localStorage.removeItem('qa_history');
    setHistory([]);
  };

  const deleteItem = (id: string) => {
    const updated = history.filter((item) => item.id !== id);
    localStorage.setItem('qa_history', JSON.stringify(updated));
    setHistory(updated);
    if (selectedItem?.id === id) {
      setSelectedItem(null);
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className="min-h-screen bg-gray-50 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900">
            历史记录
          </h1>
          {history.length > 0 && (
            <button
              onClick={clearHistory}
              className="px-4 py-2 text-sm text-red-600 hover:text-red-800 font-medium"
            >
              清空记录
            </button>
          )}
        </div>

        {history.length === 0 ? (
          <div className="text-center py-12 text-gray-500 bg-white rounded-lg shadow-sm border border-gray-200">
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
                d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <p className="text-lg">暂无历史记录</p>
            <p className="text-sm mt-2">
              开始提问后，您的对话历史会显示在这里
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {history.map((item) => (
              <div
                key={item.id}
                className="bg-white rounded-lg shadow-sm border border-gray-200 hover:shadow-md transition-shadow cursor-pointer"
                onClick={() => setSelectedItem(item)}
              >
                <div className="p-4">
                  <div className="flex justify-between items-start mb-2">
                    <h3 className="font-medium text-gray-900 flex-1 pr-4">
                      {item.question}
                    </h3>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteItem(item.id);
                      }}
                      className="text-gray-400 hover:text-red-600 transition-colors"
                      title="删除"
                    >
                      <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </button>
                  </div>

                  <div className="flex items-center gap-2 text-sm text-gray-500">
                    <time>{formatDate(item.timestamp)}</time>
                    {item.degraded && (
                      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-800">
                        降级模式
                      </span>
                    )}
                  </div>
                </div>

                {selectedItem?.id === item.id && (
                  <div className="px-4 pb-4 pt-0 border-t border-gray-100 mt-2">
                    <div className="pt-4">
                      <h4 className="text-sm font-medium text-gray-900 mb-2">
                        回答
                      </h4>
                      <p className="text-sm text-gray-700 whitespace-pre-wrap">
                        {item.answer}
                      </p>
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
