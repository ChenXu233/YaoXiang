import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
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

  // 首先尝试从当前扩展所在的目录向上查找 target/debug 目录
  let currentDir = __dirname;
  while (currentDir !== path.parse(currentDir).root) {
    const debugCandidate = path.join(currentDir, "target", "debug", executable);
    if (fs.existsSync(debugCandidate)) {
      console.log(`[client] Found YaoXiang language server at: ${debugCandidate}`);
      return { command: debugCandidate, args: ['lsp'] };
    }
    currentDir = path.dirname(currentDir);
  }

  for (const folder of folders) {
    const root = folder.uri.fsPath;
    const debugCandidate = path.join(root, "target", "debug", executable);
    if (fs.existsSync(debugCandidate)) {
      console.log(`[client] Found YaoXiang language server at: ${debugCandidate}`);
      return { command: debugCandidate, args: ['lsp'] };
    }
  }

  return { command: 'yaoxiang', args: ['lsp'] };
}

/**
 * 将可执行文件复制到临时目录并返回新路径
 * 每次调用都会重新复制，确保使用最新版本
 */
function ensureTempExecutable(originalPath: string): string {
  const isWin = process.platform === 'win32';
  const executableBase = isWin ? 'yaoxiang' : 'yaoxiang';
  const ext = isWin ? '.exe' : '';
  const tempDir = path.join(os.tmpdir(), 'yaoxiang-lsp');

  // 确保 temp 目录存在
  if (!fs.existsSync(tempDir)) {
    fs.mkdirSync(tempDir, { recursive: true });
  }

  // 尝试清理旧的临时可执行文件，防止垃圾堆积
  try {
    const files = fs.readdirSync(tempDir);
    for (const file of files) {
      if (file.startsWith(executableBase)) {
        try {
          fs.unlinkSync(path.join(tempDir, file));
        } catch {
          // 如果文件还在被占用（比如还有别的实例在运行），忽略即可
        }
      }
    }
  } catch (e) {
    console.error('Failed to clean up old temp executables', e);
  }

  // 加入时间戳避免覆盖被占用的文件
  const uniqueName = `${executableBase}-${Date.now()}${ext}`;
  const tempPath = path.join(tempDir, uniqueName);

  // 复制出最新的可执行文件
  fs.copyFileSync(originalPath, tempPath);
  
  // 在 Linux/macOS 下确保拥有执行权限
  if (!isWin) {
    fs.chmodSync(tempPath, 0o755);
  }

  return tempPath;
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
  if (client) {
    outputChannel.appendLine('[client] Language client already started');
    return;
  }

  const resolved = resolveServerCommand();
  outputChannel.appendLine('[client] Starting in debug mode (target/debug)');

  // 如果是本地文件路径，复制到 temp 目录
  let command = resolved.command;
  if (command !== 'yaoxiang' && fs.existsSync(command)) {
    command = ensureTempExecutable(command);
    outputChannel.appendLine(`[client] Copied executable to temp: ${command}`);
  }

  const serverOptions: ServerOptions = {
    command: command,
    args: resolved.args,
  };

  outputChannel.appendLine(`[client] Server command: ${command} ${resolved.args.join(' ')}`);

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
  // 检查 target/debug 是否存在
  const executable = process.platform === 'win32' ? 'yaoxiang.exe' : 'yaoxiang';
  const folders = vscode.workspace.workspaceFolders ?? [];
  let hasDebug = false;

  // 从当前扩展目录向上查找
  let currentDir = __dirname;
  while (currentDir !== path.parse(currentDir).root) {
    const debugCandidate = path.join(currentDir, "target", "debug", executable);
    if (fs.existsSync(debugCandidate)) {
      hasDebug = true;
      break;
    }
    currentDir = path.dirname(currentDir);
  }

  // 从工作区目录查找
  if (!hasDebug) {
    for (const folder of folders) {
      const root = folder.uri.fsPath;
      const debugCandidate = path.join(root, "target", "debug", executable);
      if (fs.existsSync(debugCandidate)) {
        hasDebug = true;
        break;
      }
    }
  }

  // 根据是否使用debug模式创建对应名称的output channel
  const channelName = hasDebug ? 'YaoXiang Language Server (Debug)' : 'YaoXiang Language Server';
  outputChannel = vscode.window.createOutputChannel(channelName);
  context.subscriptions.push(outputChannel);

  outputChannel.appendLine('[client] Activating YaoXiang language server extension');

  // 无论是否找到 target/debug，都启动 LSP
  // resolveServerCommand 中有 fallback 到全局 yaoxiang 的逻辑
  if (hasDebug) {
    outputChannel.appendLine('[client] target/debug found, using debug mode');
  } else {
    outputChannel.appendLine('[client] No target/debug found, using global yaoxiang');
  }

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
  statusBarItem.text = '$(bug) YaoXiang LSP (Debug)';
  statusBarItem.command = 'yaoxiang.restartLanguageServer';
  statusBarItem.tooltip = 'Restart YaoXiang Language Server (Debug Mode)';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);
}

export async function deactivate(): Promise<void> {
  await stopLanguageClient();
}
