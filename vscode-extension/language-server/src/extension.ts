import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import {
  LanguageClient,
  LanguageClientOptions,
  RevealOutputChannelOn,
  ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;

function resolveServerCommand(): { command: string; args: string[] } {
  const executable = process.platform === 'win32' ? 'yaoxiang.exe' : 'yaoxiang';
  const folders = vscode.workspace.workspaceFolders ?? [];

  for (const folder of folders) {
    const root = folder.uri.fsPath;
    const candidates = [
      path.join(root, 'target', 'debug', executable),
      path.join(root, 'target', 'release', executable),
    ];

    for (const candidate of candidates) {
      if (fs.existsSync(candidate)) {
        return { command: candidate, args: ['lsp'] };
      }
    }
  }

  return { command: 'yaoxiang', args: ['lsp'] };
}

/**
 * YaoXiang Language Server 扩展入口
 */
export function activate(context: vscode.ExtensionContext): void {
  const outputChannel = vscode.window.createOutputChannel('YaoXiang LSP');
  outputChannel.appendLine('[client] Activating YaoXiang language server extension');

  const resolved = resolveServerCommand();

  const serverOptions: ServerOptions = {
    command: resolved.command,
    args: resolved.args,
  };

  outputChannel.appendLine(`[client] Server command: ${resolved.command} ${resolved.args.join(' ')}`);

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'yaoxiang' },
      { scheme: 'untitled', language: 'yaoxiang' },
    ],
    outputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Error,
  };

  client = new LanguageClient(
    'yaoxiang-lsp',
    'YaoXiang Language Server',
    serverOptions,
    clientOptions
  );

  context.subscriptions.push(client);
  void client.start().then(
    () => {
      outputChannel.appendLine('[client] Language client started successfully');
    },
    (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
      outputChannel.appendLine(`[client] Failed to start language client: ${message}`);
      void vscode.window.showErrorMessage(`YaoXiang LSP 启动失败: ${message}`);
      outputChannel.show(true);
    }
  );

  // 可选：显示状态栏信息
  const statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.text = '$(code) YaoXiang LSP';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
