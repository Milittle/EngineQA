import { useState } from 'react';
import { QueryPage } from './pages/QueryPage';
import { StatusPage } from './pages/StatusPage';
import { HistoryPage } from './pages/HistoryPage';

type Page = 'query' | 'status' | 'history';

function App() {
  const [currentPage, setCurrentPage] = useState<Page>('query');

  return (
    <div>
      {/* Navigation */}
      <nav className="bg-white border-b border-gray-200">
        <div className="max-w-4xl mx-auto px-4">
          <div className="flex space-x-8">
            <button
              onClick={() => setCurrentPage('query')}
              className={`py-4 px-1 inline-flex items-center text-sm font-medium border-b-2 transition-colors ${
                currentPage === 'query'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              问答
            </button>
            <button
              onClick={() => setCurrentPage('history')}
              className={`py-4 px-1 inline-flex items-center text-sm font-medium border-b-2 transition-colors ${
                currentPage === 'history'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              历史
            </button>
            <button
              onClick={() => setCurrentPage('status')}
              className={`py-4 px-1 inline-flex items-center text-sm font-medium border-b-2 transition-colors ${
                currentPage === 'status'
                  ? 'border-blue-500 text-blue-600'
                  : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
              }`}
            >
              状态
            </button>
          </div>
        </div>
      </nav>

      {/* Page Content */}
      {currentPage === 'query' && <QueryPage />}
      {currentPage === 'history' && <HistoryPage />}
      {currentPage === 'status' && <StatusPage />}
    </div>
  );
}

export default App;
