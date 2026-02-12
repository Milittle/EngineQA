import { useState, useEffect } from 'react';
import { apiClient } from '../api/client';
import type { StatusResponse, JobInfo } from '../api/types';

export function StatusPage() {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [job, setJob] = useState<JobInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reindexing, setReindexing] = useState(false);

  useEffect(() => {
    loadStatus();
    loadJobStatus();
  }, []);

  const loadStatus = async () => {
    try {
      const data = await apiClient.status();
      setStatus(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load status');
    } finally {
      setLoading(false);
    }
  };

  const loadJobStatus = async () => {
    try {
      const data = await apiClient.reindexStatus();
      setJob(data.job || null);
    } catch (err) {
      // Job status endpoint might not exist yet
      setJob(null);
    }
  };

  const handleReindex = async () => {
    if (!confirm('确定要重新索引知识库吗？这可能需要几分钟时间。')) {
      return;
    }

    setReindexing(true);

    try {
      const result = await apiClient.reindex();
      alert(`索引任务已启动: ${result.job_id}`);
      setJob({
        job_id: result.job_id,
        status: 'running',
        started_at: new Date().toISOString(),
      });

      // Poll job status
      const pollInterval = setInterval(async () => {
        try {
          const data = await apiClient.reindexStatus();
          if (data.job) {
            setJob(data.job);
            if (data.job.status !== 'running') {
              clearInterval(pollInterval);
              loadStatus();
            }
          }
        } catch (err) {
          clearInterval(pollInterval);
        }
      }, 2000);

      // Auto-stop polling after 5 minutes
      setTimeout(() => clearInterval(pollInterval), 300000);
    } catch (err) {
      alert('启动索引任务失败: ' + (err instanceof Error ? err.message : 'Unknown error'));
    } finally {
      setReindexing(false);
    }
  };

  const getHealthBadge = (health: string) => {
    switch (health) {
      case 'ok':
        return (
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
            正常
          </span>
        );
      case 'degraded':
        return (
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-yellow-100 text-yellow-800">
            降级
          </span>
        );
      case 'unavailable':
        return (
          <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800">
            不可用
          </span>
        );
      default:
        return null;
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 py-8 px-4">
        <div className="max-w-4xl mx-auto text-center py-12">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <p className="mt-4 text-gray-600">加载中...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 py-8 px-4">
      <div className="max-w-4xl mx-auto">
        <div className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-bold text-gray-900">
            系统状态
          </h1>
          <button
            onClick={loadStatus}
            className="px-4 py-2 text-sm text-blue-600 hover:text-blue-800 font-medium"
          >
            刷新
          </button>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6">
            {error}
          </div>
        )}

        {status && (
          <div className="space-y-6">
            {/* Provider Info */}
            <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                推理服务
              </h2>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <dt className="text-sm font-medium text-gray-500">提供方</dt>
                  <dd className="mt-1 text-sm text-gray-900">{status.provider}</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-gray-500">模型</dt>
                  <dd className="mt-1 text-sm text-gray-900">{status.model}</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-gray-500">健康状态</dt>
                  <dd className="mt-1">{getHealthBadge(status.upstream_health)}</dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-gray-500">Qdrant 连接</dt>
                  <dd className="mt-1">
                    {status.qdrant_connected ? (
                      <span className="text-green-600">已连接</span>
                    ) : (
                      <span className="text-red-600">未连接</span>
                    )}
                  </dd>
                </div>
              </div>
            </div>

            {/* Index Info */}
            <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200">
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-lg font-semibold text-gray-900">
                  知识库索引
                </h2>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <dt className="text-sm font-medium text-gray-500">索引规模</dt>
                  <dd className="mt-1 text-2xl font-bold text-gray-900">
                    {status.index_size.toLocaleString()}
                  </dd>
                </div>
                <div>
                  <dt className="text-sm font-medium text-gray-500">最后索引时间</dt>
                  <dd className="mt-1 text-sm text-gray-900">
                    {status.last_index_time
                      ? new Date(status.last_index_time).toLocaleString('zh-CN')
                      : '未索引'}
                  </dd>
                </div>
              </div>
            </div>

            {/* Rate Limit Info */}
            <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200">
              <h2 className="text-lg font-semibold text-gray-900 mb-4">
                速率限制
              </h2>

              <div className="flex items-center gap-4">
                <div className="flex-1">
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-gray-600">当前 RPM</span>
                    <span className="text-gray-900 font-medium">
                      {status.rate_limit_state.current_rpm} / {status.rate_limit_state.rpm_limit}
                    </span>
                  </div>
                  <div className="w-full bg-gray-200 rounded-full h-2">
                    <div
                      className="bg-blue-600 h-2 rounded-full"
                      style={{
                        width: `${(status.rate_limit_state.current_rpm / status.rate_limit_state.rpm_limit) * 100}%`,
                      }}
                    ></div>
                  </div>
                </div>
              </div>
            </div>

            {/* Reindex Job */}
            <div className="bg-white rounded-lg shadow-sm p-6 border border-gray-200">
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-lg font-semibold text-gray-900">
                  索引任务
                </h2>
                <button
                  onClick={handleReindex}
                  disabled={reindexing || job?.status === 'running'}
                  className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-sm font-medium"
                >
                  {reindexing ? '启动中...' : '重新索引'}
                </button>
              </div>

              {job ? (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-sm">
                    <span className="font-medium text-gray-900">任务 ID:</span>
                    <span className="text-gray-600 font-mono">{job.job_id}</span>
                  </div>

                  <div className="flex items-center gap-2 text-sm">
                    <span className="font-medium text-gray-900">状态:</span>
                    {job.status === 'running' && (
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                        运行中
                      </span>
                    )}
                    {job.status === 'completed' && (
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                        已完成
                      </span>
                    )}
                    {job.status === 'failed' && (
                      <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800">
                        失败
                      </span>
                    )}
                  </div>

                  {job.result && (
                    <div className="mt-4 pt-4 border-t border-gray-200">
                      <h3 className="text-sm font-medium text-gray-900 mb-2">
                        任务结果
                      </h3>
                      <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                          <dt className="text-gray-500">总文件数</dt>
                          <dd className="text-gray-900">{job.result.total_files}</dd>
                        </div>
                        <div>
                          <dt className="text-gray-500">已索引文件</dt>
                          <dd className="text-gray-900">{job.result.indexed_files}</dd>
                        </div>
                        <div>
                          <dt className="text-gray-500">总 chunk 数</dt>
                          <dd className="text-gray-900">{job.result.total_chunks}</dd>
                        </div>
                        <div>
                          <dt className="text-gray-500">成功 chunk 数</dt>
                          <dd className="text-gray-900">{job.result.successful_chunks}</dd>
                        </div>
                        <div>
                          <dt className="text-gray-500">失败 chunk 数</dt>
                          <dd className="text-gray-900">{job.result.failed_chunks}</dd>
                        </div>
                        <div>
                          <dt className="text-gray-500">耗时</dt>
                          <dd className="text-gray-900">{(job.result.duration_ms / 1000).toFixed(2)}s</dd>
                        </div>
                      </div>
                    </div>
                  )}

                  {job.error && (
                    <div className="mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded-lg text-sm">
                      {job.error}
                    </div>
                  )}
                </div>
              ) : (
                <div className="text-sm text-gray-500">
                  暂无运行中的索引任务
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
