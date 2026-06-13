import { describe, it, expect, vi } from 'vitest';
import { translateBatch } from './ai.mjs';

describe('translateBatch', () => {
  it('should call API and parse response', async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        choices: [{
          message: {
            content: JSON.stringify({
              cmd_received: 'Command received',
              run_file: 'Run file'
            })
          }
        }]
      })
    });

    const result = await translateBatch({
      keys: { cmd_received: '收到命令', run_file: '运行文件' },
      sourceLang: '中文',
      targetLang: 'English',
      languagePrompt: 'Professional translation',
      glossary: {},
      apiKey: 'test-key',
      baseUrl: 'http://test.com/v1',
      model: 'test-model',
      fetchFn: mockFetch
    });

    expect(result).toEqual({
      cmd_received: 'Command received',
      run_file: 'Run file'
    });
    expect(mockFetch).toHaveBeenCalledOnce();
  });

  it('should retry on failure', async () => {
    let callCount = 0;
    const mockFetch = vi.fn().mockImplementation(async () => {
      callCount++;
      if (callCount < 3) {
        throw new Error('Network error');
      }
      return {
        ok: true,
        json: async () => ({
          choices: [{
            message: { content: JSON.stringify({ key: 'value' }) }
          }]
        })
      };
    });

    const result = await translateBatch({
      keys: { key: '原始' },
      sourceLang: '中文',
      targetLang: 'English',
      languagePrompt: 'test',
      glossary: {},
      apiKey: 'test',
      baseUrl: 'http://test.com/v1',
      model: 'test',
      fetchFn: mockFetch,
      maxRetries: 3
    });

    expect(result).toEqual({ key: 'value' });
    expect(mockFetch).toHaveBeenCalledTimes(3);
  });

  it('should throw after max retries', async () => {
    const mockFetch = vi.fn().mockRejectedValue(new Error('Network error'));

    await expect(translateBatch({
      keys: { key: 'value' },
      sourceLang: '中文',
      targetLang: 'English',
      languagePrompt: 'test',
      glossary: {},
      apiKey: 'test',
      baseUrl: 'http://test.com/v1',
      model: 'test',
      fetchFn: mockFetch,
      maxRetries: 2
    })).rejects.toThrow('Network error');
  });
});
