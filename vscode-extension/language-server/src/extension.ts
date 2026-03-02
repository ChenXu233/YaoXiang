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
let clientStartPromise: Promise<void> | undefined;
let outputChannel: vscode.OutputChannel | undefined;
let restartInProgress: Promise<void> | undefined;

function resolveServerCommand(): { command: string; args: string[] } {
  const executable = process.platform === 'win32' ? 'yaoxiang.exe' : 'yaoxiang';
  const folders = vscode.workspace.workspaceFolders ?? [];

  for (const folder of folders) {
    const root = folder.uri.fsPath;
    const candidates = [
      path.join(root, 'target', 'test', executable),
    ];

    for (const candidate of candidates) {
      if (fs.existsSync(candidate)) {
        console.log(`[client] Found YaoXiang language server executable at: ${candidate}`);
        return { command: candidate, args: ['lsp'] };
      }
    }
  }

  return { command: 'yaoxiang', args: ['lsp'] };
}

async function stopLanguageClient(): Promise<void> {
  if (!client) {
    return;
  }

  try {
    await client.stop();
  } finally {
    client = undefined;
    clientStartPromise = undefined;
  }
}

async function startLanguageClient(): Promise<void> {
  if (!outputChannel) {
    outputChannel = vscode.window.createOutputChannel('YaoXiang LSP');
  }

  if (client) {
    outputChannel.appendLine('[client] Language client already started');
    return;
  }

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

  clientStartPromise = Promise.resolve(client.start());

  await clientStartPromise.then(
    () => {
      outputChannel?.appendLine('[client] Language client started successfully');
    },
    (error: unknown) => {
      const message = error instanceof Error ? error.message : String(error);
      outputChannel?.appendLine(`[client] Failed to start language client: ${message}`);
      void vscode.window.showErrorMessage(`YaoXiang LSP 重启失败: ${message}`);
      outputChannel?.show(true);
    }
  );
}

async function restartLanguageClient(): Promise<void> {
  if (restartInProgress) {
    await restartInProgress;
    return;
  }

  restartInProgress = Promise.resolve(
    vscode.window.withProgress(
      { location: vscode.ProgressLocation.Notification, title: 'Restarting YaoXiang Language Server...' },
      async () => {
        outputChannel?.appendLine('[client] Restart requested');
        await stopLanguageClient();
        await startLanguageClient();
        outputChannel?.appendLine('[client] Restart completed');
      }
    )
  );

  try {
    await restartInProgress;
  } finally {
    restartInProgress = undefined;
  }
}

/**
 * YaoXiang Language Server
 */
export function activate(context: vscode.ExtensionContext): void {
  outputChannel = vscode.window.createOutputChannel('YaoXiang LSP');
  context.subscriptions.push(outputChannel);

  outputChannel.appendLine('[client] Activating YaoXiang language server extension');

  context.subscriptions.push(
    vscode.commands.registerCommand('yaoxiang.restartLanguageServer', async () => {
      await restartLanguageClient();
    })
  );

  context.subscriptions.push(
    new vscode.Disposable(() => {
      void stopLanguageClient();
    })
  );

  void startLanguageClient();

  const statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.text = '$(code) YaoXiang LSP';
  statusBarItem.command = 'yaoxiang.restartLanguageServer';
  statusBarItem.tooltip = 'Restart YaoXiang Language Server';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);
}

export async function deactivate(): Promise<void> {
  await stopLanguageClient();
}
