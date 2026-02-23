import * as vscode from 'vscode';

/**
 * YaoXiang Language Server 扩展入口
 *
 * 注意：此扩展使用 VS Code 的内置 LSP 客户端。
 * 通过 package.json 中的 "server" 配置，VS Code 会自动启动 yaoxiang lsp 命令。
 *
 * 如果 yaoxiang 命令不在 PATH 中，VS Code 会显示错误提示。
 */
export function activate(context: vscode.ExtensionContext): void {
  // Language Server 通过 package.json 的 server 字段自动启动
  // 此处无需额外代码

  // 可选：显示状态栏信息
  const statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.text = '$(code) YaoXiang LSP';
  statusBarItem.command = 'yaoxiang.showLspStatus';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);
}

export function deactivate(): void {
  // 清理资源
}
