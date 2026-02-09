import { app, BrowserWindow, ipcMain, dialog } from 'electron';
import path from 'node:path';
import started from 'electron-squirrel-startup';
import { spawn, spawnSync, ChildProcess } from 'child_process';
import fs from 'node:fs';
import {
  getPackageScripts,
  isValidProjectPath,
  loadProjectConfig,
  saveProjectConfig,
  addProjectToConfig,
  removeProjectFromConfig,
  getProjectNodeVersion,
  setProjectNodeVersionFile,
} from './utils/projectManager';
import { NodeVersion, NvmInfo, NodeVersionManager } from './types/project';

// 处理在 Windows 上安装/卸载时创建/删除快捷方式
if (started) {
  app.quit();
}
app.setName('devFleet');

// 设置控制台错误过滤器
const originalConsoleError = console.error;
console.error = (...args: any[]) => {
  const message = args.join(' ');
  // 过滤掉 Autofill 相关的错误
  if (message.includes('Autofill.enable') ||
      message.includes('Autofill.setAddresses') ||
      message.includes('devtools://devtools') ||
      message.includes('Request Autofill')) {
    return; // 不输出这些错误
  }
  originalConsoleError.apply(console, args);
};

// 应用启动日志
console.log('🚀 devFleet 启动中...');

const createWindow = () => {
  // 创建浏览器窗口
  const mainWindow = new BrowserWindow({
    width: 1000,
    height: 600,
    autoHideMenuBar: true, 
    title: 'devFleet',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
      allowRunningInsecureContent: false,
    },
  });

  // 加载应用的 index.html 文件
  if (MAIN_WINDOW_VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(MAIN_WINDOW_VITE_DEV_SERVER_URL);
  } else {
    mainWindow.loadFile(path.join(__dirname, `../renderer/${MAIN_WINDOW_VITE_NAME}/index.html`));
  }

  // 开发环境配置
  const isDevelopment = !!MAIN_WINDOW_VITE_DEV_SERVER_URL;

  if (isDevelopment) {
    // 开发环境：延迟打开开发者工具
    mainWindow.webContents.once('did-finish-load', () => {
      setTimeout(() => {
        mainWindow.webContents.openDevTools({ mode: 'detach' });
        console.log('🔧 开发者工具已打开');
        console.log('ℹ️  已知问题说明:');
        console.log('   - Autofill API 错误是 Electron 开发者工具的已知问题');
        console.log('   - 这些错误不会影响应用程序的正常功能');
        console.log('   - 在生产环境中不会出现这些错误');
      }, 1000);
    });
  }

  // 添加窗口事件监听
  mainWindow.on('ready-to-show', () => {
    console.log('✅ 应用程序窗口已准备就绪');
  });
};

// 存储运行中的进程
const runningProcesses = new Map<string, ChildProcess>();

// 支持的编辑器类型
type EditorId = 'vscode' | 'cursor' | 'webstorm';

// 支持的包管理器类型
type PackageManager = 'npm' | 'yarn' | 'pnpm' | 'bun';

// 判断命令是否可用
function isCommandAvailable(cmd: string, args: string[] = ['--version']): boolean {
  try {
    const res = spawnSync(cmd, args, { 
      stdio: 'ignore',
      shell: process.platform === 'win32' 
    });
    return res.status === 0;
  } catch {
    return false;
  }
}

// 检测项目使用的包管理器
function detectPackageManager(projectPath: string): PackageManager {
  try {
    // 检查锁文件来确定包管理器
    const hasPackageLock = fs.existsSync(path.join(projectPath, 'package-lock.json'));
    const hasYarnLock = fs.existsSync(path.join(projectPath, 'yarn.lock'));
    const hasPnpmLock = fs.existsSync(path.join(projectPath, 'pnpm-lock.yaml'));
    const hasBunLock = fs.existsSync(path.join(projectPath, 'bun.lockb'));
    
    // 根据锁文件判断
    if (hasBunLock) return 'bun';
    if (hasPnpmLock) return 'pnpm';
    if (hasYarnLock) return 'yarn';
    if (hasPackageLock) return 'npm';
    
    // 如果没有锁文件，检查 packageManager 字段
    const packageJsonPath = path.join(projectPath, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
      if (packageJson.packageManager) {
        const manager = packageJson.packageManager.split('@')[0];
        if (['npm', 'yarn', 'pnpm', 'bun'].includes(manager)) {
          return manager as PackageManager;
        }
      }
    }
    
    // 默认使用 npm
    return 'npm';
  } catch (error) {
    console.error('检测包管理器失败:', error);
    return 'npm';
  }
}

// 获取运行脚本的命令
function getRunCommand(packageManager: PackageManager, scriptName: string): string {
  switch (packageManager) {
    case 'pnpm':
      // pnpm 可以直接运行脚本，不需要 run
      return `pnpm ${scriptName}`;
    case 'yarn':
      // yarn 也可以直接运行脚本
      return `yarn ${scriptName}`;
    case 'bun':
      // bun 同样可以直接运行
      return `bun ${scriptName}`;
    case 'npm':
    default:
      // npm 需要 run 关键字
      return `npm run ${scriptName}`;
  }
}

// ============= NVM 相关功能函数 =============

// 检测系统安装的 Node 版本管理器
function detectNodeVersionManager(): NodeVersionManager {
  try {
    // 1. 优先检测 nvmd（跨平台）
    const nvmdResult = spawnSync('nvmd', ['--help'], {
      stdio: 'pipe',
      shell: process.platform === 'win32'
    });
    if (nvmdResult.status === 0) {
      return 'nvmd';
    }

    // 2. 检测 nvs（跨平台）
    const nvsResult = spawnSync('nvs', ['--version'], {
      stdio: 'pipe',
      shell: process.platform === 'win32'
    });
    if (nvsResult.status === 0) {
      return 'nvs';
    }

    // 3. 检测 nvm（Windows 或 Unix）
    const isWin = process.platform === 'win32';
    if (isWin) {
      // Windows: 检测 nvm-windows
      const nvmWinResult = spawnSync('nvm', ['version'], {
        stdio: 'pipe',
        shell: true
      });
      if (nvmWinResult.status === 0) {
        return 'nvm-windows';
      }
    } else {
      // macOS/Linux: 检测 nvm
      const nvmResult = spawnSync('bash', ['-c', 'command -v nvm'], {
        stdio: 'pipe'
      });
      if (nvmResult.status === 0) {
        return 'nvm';
      }
    }

    return 'none';
  } catch {
    return 'none';
  }
}

// 检查版本管理器是否已安装
function isVersionManagerInstalled(manager?: NodeVersionManager): boolean {
  const detectedManager = manager || detectNodeVersionManager();
  return detectedManager !== 'none';
}

// 获取当前系统使用的 Node 版本
function getCurrentNodeVersion(): string | null {
  try {
    const result = spawnSync('node', ['--version'], {
      encoding: 'utf8',
      shell: process.platform === 'win32'
    });
    if (result.status === 0 && result.stdout) {
      return result.stdout.trim().replace('v', '');
    }
    return null;
  } catch {
    return null;
  }
}

// 获取所有已安装的 Node 版本
function getNodeVersions(manager: NodeVersionManager): NodeVersion[] {
  try {
    const currentVersion = getCurrentNodeVersion();
    let result;

    switch (manager) {
      case 'nvmd':
        // nvmd: 使用 nvmd ls 或 nvmd list
        result = spawnSync('nvmd', ['ls'], {
          encoding: 'utf8',
          shell: true
        });
        break;

      case 'nvs':
        // nvs: 使用 nvs ls 列出所有已安装版本
        result = spawnSync('nvs', ['ls'], {
          encoding: 'utf8',
          shell: process.platform === 'win32'
        });
        break;

      case 'nvm-windows':
        // Windows: 使用 nvm list
        result = spawnSync('nvm', ['list'], {
          encoding: 'utf8',
          shell: true
        });
        break;

      case 'nvm':
        // macOS/Linux: 使用 bash 执行 nvm ls
        result = spawnSync('bash', ['-c', 'source ~/.nvm/nvm.sh && nvm ls'], {
          encoding: 'utf8'
        });
        break;

      default:
        return [];
    }

    if (result.status !== 0) {
      return [];
    }

    // nvmd 的输出在 stderr 中，其他版本管理器在 stdout 中
    const output = manager === 'nvmd' ? result.stderr : result.stdout;

    if (!output) {
      return [];
    }

    const lines = output.split('\n');
    const versions: NodeVersion[] = [];

    for (const line of lines) {
      // 匹配版本号：18.18.0, v18.18.0, v20.5.1 (currently) 等格式
      // nvs 格式：node/20.11.0/x64
      const match = line.match(/(?:node\/)?v?(\d+\.\d+\.\d+)/);
      if (match) {
        const version = match[1];
        const fullVersion = `v${version}`;
        // nvmd 格式：v20.5.1 (currently)
        // nvm 格式：当前版本带箭头或标记
        // nvs 格式：带 > 前缀表示当前版本
        const isCurrent = currentVersion === version ||
                         line.includes('(currently)') ||
                         line.includes('(current)') ||
                         line.trim().startsWith('>');

        versions.push({
          version,
          fullVersion,
          isCurrent
        });
      }
    }

    // 去重并排序
    const uniqueVersions = Array.from(
      new Map(versions.map(v => [v.version, v])).values()
    );

    return uniqueVersions.sort((a, b) => {
      // 按版本号降序排序
      const aParts = a.version.split('.').map(Number);
      const bParts = b.version.split('.').map(Number);
      for (let i = 0; i < 3; i++) {
        if (aParts[i] !== bParts[i]) {
          return bParts[i] - aParts[i];
        }
      }
      return 0;
    });
  } catch (error) {
    console.error('获取 Node 版本失败:', error);
    return [];
  }
}

// 获取版本管理器信息
function getNvmInfo(): NvmInfo {
  const manager = detectNodeVersionManager();
  const isInstalled = manager !== 'none';
  const currentVersion = getCurrentNodeVersion();
  const availableVersions = isInstalled ? getNodeVersions(manager) : [];

  return {
    isInstalled,
    manager,
    currentVersion: currentVersion || undefined,
    availableVersions
  };
}

// 检查 macOS 上的应用是否安装
function isMacAppInstalled(appName: string): boolean {
  try {
    const result = spawnSync('mdfind', [
      `kMDItemKind == "Application" && kMDItemDisplayName == "${appName}"`
    ], { encoding: 'utf8' });
    return result.status === 0 && result.stdout.trim().length > 0;
  } catch {
    return false;
  }
}

// 用指定编辑器打开项目目录
function openWithEditor(editor: EditorId, projectPath: string): boolean {
  const isWin = process.platform === 'win32';
  const isMac = process.platform === 'darwin';
  
  try {
    if (editor === 'vscode') {
      if (isMac) {
        spawn('open', ['-a', 'Visual Studio Code', projectPath], { shell: false });
      } else {
        // Windows 和 Linux 都使用 code 命令
        spawn('code', [projectPath], { shell: true });
      }
      return true;
    }
    
    if (editor === 'cursor') {
      if (isMac) {
        spawn('open', ['-a', 'Cursor', projectPath], { shell: false });
      } else {
        spawn('cursor', [projectPath], { shell: true });
      }
      return true;
    }
    
    if (editor === 'webstorm') {
      if (isMac) {
        spawn('open', ['-a', 'WebStorm', projectPath], { shell: false });
        return true;
      }
      if (isWin) {
        // Windows: 尝试多种可能的命令
        const commands = ['webstorm', 'webstorm64', 'webstorm.exe', 'webstorm64.exe'];
        for (const cmd of commands) {
          if (isCommandAvailable(cmd)) {
            spawn(cmd, [projectPath], { shell: true });
            return true;
          }
        }
        return false;
      }
      // Linux
      const linuxCommands = ['webstorm', 'jetbrains-webstorm', 'webstorm.sh'];
      for (const cmd of linuxCommands) {
        if (isCommandAvailable(cmd)) {
          spawn(cmd, [projectPath], { detached: true });
          return true;
        }
      }
      return false;
    }
  } catch (error) {
    console.error(`打开编辑器 ${editor} 失败:`, error);
    return false;
  }
  return false;
}

// IPC 处理程序
const setupIpcHandlers = () => {
  // 选择文件夹
  ipcMain.handle('select-folder', async () => {
    try {
      const result = await dialog.showOpenDialog({
        properties: ['openDirectory'],
        title: '选择项目文件夹'
      });

      if (result.canceled || result.filePaths.length === 0) {
        return { success: false, error: '用户取消选择' };
      }

      const selectedPath = result.filePaths[0];

      if (!isValidProjectPath(selectedPath)) {
        return {
          success: false,
          error: '所选文件夹不是有效的项目目录（缺少 package.json）'
        };
      }

      const scripts = getPackageScripts(selectedPath);
      const packageManager = detectPackageManager(selectedPath);

      return {
        success: true,
        data: {
          path: selectedPath,
          scripts,
          packageManager
        }
      };
    } catch (error) {
      return {
        success: false,
        error: `选择文件夹时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 获取项目的 npm 脚本和包管理器
  ipcMain.handle('get-package-scripts', async (_, projectPath: string) => {
    try {
      if (!isValidProjectPath(projectPath)) {
        return {
          success: false,
          error: '无效的项目路径'
        };
      }

      const scripts = getPackageScripts(projectPath);
      const packageManager = detectPackageManager(projectPath);
      
      return {
        success: true,
        data: {
          scripts,
          packageManager
        }
      };
    } catch (error) {
      return {
        success: false,
        error: `获取脚本时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 检测项目的包管理器
  ipcMain.handle('detect-package-manager', async (_, projectPath: string) => {
    try {
      const packageManager = detectPackageManager(projectPath);
      return {
        success: true,
        data: { packageManager }
      };
    } catch (error) {
      return {
        success: false,
        error: `检测包管理器失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 运行脚本
  ipcMain.handle('run-script', async (_, { projectPath, scriptName, projectId, packageManager, nodeVersion }) => {
    try {
      const isWindows = process.platform === 'win32';

      // 如果没有传入包管理器，则自动检测
      const pm = packageManager || detectPackageManager(projectPath);
      const runCommand = getRunCommand(pm, scriptName);

      // 注意：不再在命令中拼接版本切换命令
      // 版本管理器会自动读取项目目录下的 .nvmdrc 或 .nvmrc 文件

      if (isWindows) {
        // Windows: 用新的 PowerShell 窗口
        spawn('cmd.exe', ['/c', 'start', '""', 'powershell',
          '-NoExit', '-NoProfile', '-ExecutionPolicy', 'Bypass',
          '-Command', `cd "${projectPath}"; ${runCommand}`
        ], { cwd: projectPath, windowsHide: false });
      } else if (process.platform === 'darwin') {
        // macOS: 使用 Terminal.app 打开
        const osa = `tell application "Terminal"
  activate
  do script "cd ${projectPath.replace(/"/g, '\\"')} && ${runCommand}"
end tell`;
        spawn('osascript', ['-e', osa]);
      } else {
        // Linux: 尝试常见终端
        const terms = [
          ['gnome-terminal', ['--', 'bash', '-lc', `${runCommand}; exec bash`]],
          ['konsole', ['-e', `bash -lc "${runCommand}; exec bash"`]],
          ['xterm', ['-e', `bash -lc "${runCommand}; exec bash"`]],
          ['alacritty', ['-e', 'bash', '-lc', `${runCommand}; exec bash`]]
        ] as const;

        let started = false;
        for (const [cmd, args] of terms) {
          const p = spawn(cmd, args, { cwd: projectPath });
          p.on('error', () => { /* ignore */ });
          p.on('spawn', () => { started = true; });
          // 简单地尝试第一个能启动的
          await new Promise(r => setTimeout(r, 150));
          if (started) break;
        }

        if (!started) {
          return {
            success: false,
            error: '无法找到可用的终端程序'
          };
        }
      }

      return {
        success: true,
        data: {
          message: '已在外部终端启动',
          command: runCommand,
          packageManager: pm,
          nodeVersion
        }
      };

    } catch (error) {
      return {
        success: false,
        error: `启动脚本时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 检测编辑器是否已安装
  ipcMain.handle('detect-editors', async () => {
    console.log('开始检测编辑器...');
    
    try {
      const isWin = process.platform === 'win32';
      const isMac = process.platform === 'darwin';
      
      let vscode = false;
      let cursor = false;
      let webstorm = false;
      
      if (isMac) {
        // macOS: 检查应用是否安装
        vscode = isMacAppInstalled('Visual Studio Code') || isCommandAvailable('code');
        cursor = isMacAppInstalled('Cursor') || isCommandAvailable('cursor');
        webstorm = isMacAppInstalled('WebStorm');
      } else if (isWin) {
        // Windows: 检查命令是否可用
        vscode = isCommandAvailable('code');
        cursor = isCommandAvailable('cursor');
        webstorm = isCommandAvailable('webstorm') || 
                   isCommandAvailable('webstorm64') ||
                   isCommandAvailable('webstorm.exe') || 
                   isCommandAvailable('webstorm64.exe');
      } else {
        // Linux: 检查命令是否可用
        vscode = isCommandAvailable('code');
        cursor = isCommandAvailable('cursor');
        webstorm = isCommandAvailable('webstorm') || 
                   isCommandAvailable('jetbrains-webstorm') ||
                   isCommandAvailable('webstorm.sh');
      }
      
      console.log('编辑器检测结果:', { vscode, cursor, webstorm });
      
      return { 
        success: true, 
        data: { vscode, cursor, webstorm } 
      };
    } catch (error) {
      console.error('检测编辑器失败:', error);
      return { 
        success: false, 
        error: `检测编辑器失败: ${error instanceof Error ? error.message : String(error)}` 
      };
    }
  });

  // 用指定编辑器打开项目
  ipcMain.handle('open-in-editor', async (_evt, params: { editor: EditorId; projectPath: string }) => {
    const { editor, projectPath } = params;
    try {
      const ok = openWithEditor(editor, projectPath);
      if (!ok) {
        return { 
          success: false, 
          error: '未找到对应编辑器或命令不可用' 
        };
      }
      return { success: true };
    } catch (error) {
      return { 
        success: false, 
        error: `打开编辑器失败: ${error instanceof Error ? error.message : String(error)}` 
      };
    }
  });

  // 停止脚本
  ipcMain.handle('stop-script', async (_, projectId: string) => {
    try {
      const cp = runningProcesses.get(projectId);

      if (!cp) {
        return {
          success: false,
          error: '未找到运行中的进程'
        };
      }

      // 终止进程
      const isWindows = process.platform === 'win32';
      if (isWindows && cp.pid) {
        // Windows 上使用 taskkill
        spawn('taskkill', ['/pid', cp.pid.toString(), '/t', '/f']);
      } else {
        // Unix 系统使用 SIGTERM
        cp.kill('SIGTERM');
      }

      runningProcesses.delete(projectId);

      return {
        success: true,
        data: { message: '脚本已停止' }
      };
    } catch (error) {
      return {
        success: false,
        error: `停止脚本时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 检查脚本运行状态
  ipcMain.handle('check-script-status', async (_, projectId: string) => {
    const isRunning = runningProcesses.has(projectId);
    return {
      success: true,
      data: { isRunning }
    };
  });

  // 加载项目配置
  ipcMain.handle('load-project-config', async () => {
    try {
      const config = loadProjectConfig();
      // 为每个项目添加包管理器信息
      if (config.projects) {
        for (const project of config.projects) {
          if (!project.packageManager) {
            project.packageManager = detectPackageManager(project.path);
          }
        }
      }
      return {
        success: true,
        data: config
      };
    } catch (error) {
      return {
        success: false,
        error: `加载项目配置失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 保存项目配置
  ipcMain.handle('save-project-config', async (_, config) => {
    try {
      const success = saveProjectConfig(config);
      return {
        success,
        data: success ? { message: '配置保存成功' } : null,
        error: success ? null : '保存配置失败'
      };
    } catch (error) {
      return {
        success: false,
        error: `保存项目配置失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 添加项目到配置
  ipcMain.handle('add-project-to-config', async (_, projectPath: string) => {
    try {
      const project = addProjectToConfig(projectPath);
      if (project) {
        // 添加包管理器信息
        project.packageManager = detectPackageManager(projectPath);
      }
      return {
        success: !!project,
        data: project,
        error: project ? null : '添加项目失败'
      };
    } catch (error) {
      return {
        success: false,
        error: `添加项目失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 从配置中移除项目
  ipcMain.handle('remove-project-from-config', async (_, projectId: string) => {
    try {
      const success = removeProjectFromConfig(projectId);
      return {
        success,
        data: success ? { message: '项目删除成功' } : null,
        error: success ? null : '删除项目失败'
      };
    } catch (error) {
      return {
        success: false,
        error: `删除项目失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // ============= NVM 相关 IPC Handlers =============

  // 获取 NVM 信息
  ipcMain.handle('get-nvm-info', async () => {
    try {
      const nvmInfo = getNvmInfo();
      return {
        success: true,
        data: nvmInfo
      };
    } catch (error) {
      return {
        success: false,
        error: `获取 NVM 信息失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 检测项目推荐的 Node 版本
  ipcMain.handle('detect-project-node-version', async (_, projectPath: string) => {
    try {
      const version = getProjectNodeVersion(projectPath);
      return {
        success: true,
        data: { version }
      };
    } catch (error) {
      return {
        success: false,
        error: `检测项目 Node 版本失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 设置项目的 Node 版本
  ipcMain.handle('set-project-node-version', async (_, { projectId, nodeVersion }: { projectId: string; nodeVersion: string | null }) => {
    try {
      const config = loadProjectConfig();
      const project = config.projects.find(p => p.id === projectId);

      if (!project) {
        return {
          success: false,
          error: '项目不存在'
        };
      }

      // 检测版本管理器类型
      const versionManager = detectNodeVersionManager();
      if (versionManager === 'none') {
        return {
          success: false,
          error: '未检测到 Node 版本管理器（nvmd/nvm）'
        };
      }

      // 在项目目录下创建或删除版本配置文件
      const fileCreated = setProjectNodeVersionFile(project.path, nodeVersion, versionManager);
      if (!fileCreated) {
        return {
          success: false,
          error: '操作版本配置文件失败'
        };
      }

      // 更新配置
      project.nodeVersion = nodeVersion || undefined;
      const success = saveProjectConfig(config);

      const fileName = versionManager === 'nvmd' ? '.nvmdrc' :
                       versionManager === 'nvs' ? '.node-version' : '.nvmrc';

      const message = !nodeVersion || nodeVersion.trim() === ''
        ? `已删除 ${fileName} 文件`
        : `已创建 ${fileName} 文件并设置 Node 版本为 ${nodeVersion}`;

      return {
        success,
        data: success ? { message, project } : null,
        error: success ? null : '保存配置失败'
      };
    } catch (error) {
      return {
        success: false,
        error: `设置 Node 版本失败: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });
};

// 当 Electron 完成初始化并准备创建浏览器窗口时，将调用此方法
app.on('ready', () => {
  createWindow();
  setupIpcHandlers();
});

// 当所有窗口都关闭时退出，除了在 macOS 上
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  // 在 OS X 上，当点击 dock 图标且没有其他窗口打开时，
  // 通常会在应用程序中重新创建一个窗口
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

