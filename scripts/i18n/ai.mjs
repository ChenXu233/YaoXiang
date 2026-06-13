/**
 * 构建翻译 prompt
 */
function buildPrompt({ keys, sourceLang, targetLang, languagePrompt, glossary }) {
  const glossaryText = Object.entries(glossary)
    .map(([k, v]) => `${k} → ${v}`)
    .join('\n');

  return `你是一个专业的翻译器。

${languagePrompt}

术语表：
${glossaryText}

请将以下 JSON 中的值从${sourceLang}翻译成${targetLang}，保持 JSON 格式不变：
${JSON.stringify(keys, null, 2)}

注意：这是一个包含多个 key-value 对的 JSON 对象，需要批量翻译所有 value。

要求：
1. 保持专业术语一致性
2. {0}、{1}、{char}、{literal} 等占位符保持不变
3. 保持原有的语气和风格
4. 只返回 JSON，不要添加额外说明`;
}

/**
 * 批量翻译 key-value 对
 */
export async function translateBatch({
  keys,
  sourceLang,
  targetLang,
  languagePrompt,
  glossary,
  apiKey,
  baseUrl,
  model,
  fetchFn = fetch,
  maxRetries = 3
}) {
  const prompt = buildPrompt({ keys, sourceLang, targetLang, languagePrompt, glossary });

  let lastError;
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      const response = await fetchFn(`${baseUrl}/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`
        },
        body: JSON.stringify({
          model,
          messages: [{ role: 'user', content: prompt }],
          temperature: 0.3
        })
      });

      if (!response.ok) {
        throw new Error(`API error: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();
      const content = data.choices[0].message.content;

      // 尝试从响应中提取 JSON
      const jsonMatch = content.match(/\{[\s\S]*\}/);
      if (!jsonMatch) {
        throw new Error('No JSON found in response');
      }

      return JSON.parse(jsonMatch[0]);
    } catch (error) {
      lastError = error;
      if (attempt < maxRetries - 1) {
        // 指数退避
        await new Promise(r => setTimeout(r, Math.pow(2, attempt) * 1000));
      }
    }
  }

  throw lastError;
}
