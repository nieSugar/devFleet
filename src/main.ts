import { app, BrowserWindow, Menu, ipcMain, dialog } from 'electron';
import path from 'node:path';
import started from 'electron-squirrel-startup';
import { spawn, spawnSync, ChildProcess } from 'child_process';
import {
  getPackageScripts,
  isValidProjectPath,
  loadProjectConfig,
  saveProjectConfig,
  addProjectToConfig,
  removeProjectFromConfig,
} from './utils/projectManager';

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
      // 禁用 Node.js 集成以提高安全性
      nodeIntegration: false,
      // 启用上下文隔离
      contextIsolation: true,
      // 在生产环境中启用网页安全性
      webSecurity: true,
      // 禁用不安全内容运行
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

 // 判断命令是否可用
 function isCommandAvailable(cmd: string, args: string[] = ['--version']): boolean {
   try {
     const res = spawnSync(cmd, args, { stdio: 'ignore',shell: true });
     return res.status === 0;
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
         spawn('open', ['-a', 'Visual Studio Code', projectPath], { shell: true });
       } else if (isWin) {
         // 通过 cmd start 调用 code
         spawn('code', [projectPath], {  shell: true });
       } else {
         spawn('code', [projectPath], { shell: true  });
       }
       return true;
     }
     if (editor === 'cursor') {
       if (isMac) {
         spawn('open', ['-a', 'Cursor', projectPath], { shell: true });
       } else if (isWin) {
         spawn('cursor', [ projectPath], { shell: true });
       } else {
         spawn('cursor', [projectPath], { shell: true });
       }
       return true;
     }
     if (editor === 'webstorm') {
       if (isMac) {
         spawn('open', ['-a', 'WebStorm', projectPath], { detached: true });
         return true;
       }
       if (isWin) {
         const exe = isCommandAvailable('webstorm64.exe') ? 'webstorm64.exe' : (isCommandAvailable('webstorm.exe') ? 'webstorm.exe' : null);
         if (!exe) return false;
         spawn('webstorm', [projectPath], { shell: true });
         return true;
       }
       const cmd = isCommandAvailable('webstorm') ? 'webstorm' : (isCommandAvailable('jetbrains-webstorm') ? 'jetbrains-webstorm' : null);
       if (!cmd) return false;
       spawn(cmd, [projectPath], { detached: true });
       return true;
     }
   } catch {
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

      return {
        success: true,
        data: {
          path: selectedPath,
          scripts
        }
      };
    } catch (error) {
      return {
        success: false,
        error: `选择文件夹时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 获取项目的 npm 脚本
  ipcMain.handle('get-package-scripts', async (_, projectPath: string) => {
    try {
      if (!isValidProjectPath(projectPath)) {
        return {
          success: false,
          error: '无效的项目路径'
        };
      }

      const scripts = getPackageScripts(projectPath);
      return {
        success: true,
        data: scripts
      };
    } catch (error) {
      return {
        success: false,
        error: `获取脚本时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 运行 npm 脚本
  ipcMain.handle('run-script', async (_, { projectPath, scriptName, projectId }) => {
    try {
      const isWindows = process.platform === 'win32';

        if (isWindows) {
          // 用新的 PowerShell 窗口
          spawn('cmd.exe', ['/c', 'start', '""', 'powershell',
            '-NoExit', '-NoProfile', '-ExecutionPolicy', 'Bypass',
            '-Command', `npm run ${scriptName}`
          ], { cwd: projectPath, windowsHide: false });
        } else if (process.platform === 'darwin') {
          // macOS 使用 Terminal.app 打开
          const osa = `tell application "Terminal"
  activate
  do script "cd ${projectPath.replace(/"/g, '\\"')} && npm run ${scriptName}"
end tell`;
          spawn('osascript', ['-e', osa]);
        } else {
          // Linux: 尝试常见终端
          const terms = [
            ['gnome-terminal', ['--', 'bash', '-lc', `npm run ${scriptName}; exec bash`]],
            ['konsole', ['-e', `bash -lc "npm run ${scriptName}; exec bash"`]],
            ['xterm', ['-e', `bash -lc "npm run ${scriptName}; exec bash"`]],
            ['alacritty', ['-e', 'bash', '-lc', `npm run ${scriptName}; exec bash`]]
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
        }
        return { success: true, data: { message: '已在外部终端启动' } };

    } catch (error) {
      return {
        success: false,
        error: `启动脚本时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 检测编辑器是否已安装
  ipcMain.handle('detect-editors', async () => {
    console.log('detect-editors');
    
    try {
      const isWin = process.platform === 'win32';
      const isMac = process.platform === 'darwin';
      // VS Code 可以通过 command line 工具 code 检测
      const vscode = isMac ? true : isCommandAvailable('code');
      // Cursor
      const cursor = isMac ? true : isCommandAvailable('cursor');
      // WebStorm
      let webstorm = false;
      if (isMac) {
        webstorm = true;
      } else if (isWin) {
        webstorm = isCommandAvailable('webstorm64.exe') || isCommandAvailable('webstorm.exe');
      } else {
        webstorm = isCommandAvailable('webstorm') || isCommandAvailable('jetbrains-webstorm');
      }
      return { success: true, data: { vscode, cursor, webstorm } };
    } catch (error) {
      return { success: false, error: `检测编辑器失败: ${error instanceof Error ? error.message : String(error)}` };
    }
  });

  // 用指定编辑器打开项目
  ipcMain.handle('open-in-editor', async (_evt, params: { editor: EditorId; projectPath: string }) => {
    const { editor, projectPath } = params;
    try {
      const ok = openWithEditor(editor, projectPath);
      if (!ok) return { success: false, error: '未找到对应编辑器或命令不可用' };
      return { success: true };
    } catch (error) {
      return { success: false, error: `打开编辑器失败: ${error instanceof Error ? error.message : String(error)}` };
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
};

// 当 Electron 完成初始化并准备创建浏览器窗口时，将调用此方法
// 某些 API 只能在此事件发生后使用
app.on('ready', () => {
  createWindow();
  setupIpcHandlers();
});

// 当所有窗口都关闭时退出，除了在 macOS 上。在 macOS 上，应用程序和它们的菜单栏
// 通常保持活动状态，直到用户使用 Cmd + Q 明确退出
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

// 在此文件中，您可以包含应用程序特定主进程的其余代码
// 您也可以将它们放在单独的文件中并在此处导入
