import { app, BrowserWindow, Menu, ipcMain, dialog } from 'electron';
import path from 'node:path';
import started from 'electron-squirrel-startup';
import { spawn, ChildProcess } from 'child_process';
import {
  getPackageScripts,
  isValidProjectPath,
  loadProjectConfig,
  saveProjectConfig,
  addProjectToConfig,
  removeProjectFromConfig,
  createProject
} from './utils/projectManager';

// 处理在 Windows 上安装/卸载时创建/删除快捷方式
if (started) {
  app.quit();
}

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
console.log('🚀 React + Electron 应用启动中...');

// 创建中文菜单栏
const createMenu = () => {
  const template: any[] = [
    {
      label: '文件',
      submenu: [
        {
          label: '新建',
          accelerator: 'CmdOrCtrl+N',
          click: () => {
            console.log('新建文件');
          }
        },
        {
          label: '打开',
          accelerator: 'CmdOrCtrl+O',
          click: () => {
            console.log('打开文件');
          }
        },
        { type: 'separator' },
        {
          label: '退出',
          accelerator: process.platform === 'darwin' ? 'Cmd+Q' : 'Ctrl+Q',
          click: () => {
            app.quit();
          }
        }
      ]
    },
    {
      label: '编辑',
      submenu: [
        {
          label: '撤销',
          accelerator: 'CmdOrCtrl+Z',
          role: 'undo'
        },
        {
          label: '重做',
          accelerator: 'Shift+CmdOrCtrl+Z',
          role: 'redo'
        },
        { type: 'separator' },
        {
          label: '剪切',
          accelerator: 'CmdOrCtrl+X',
          role: 'cut'
        },
        {
          label: '复制',
          accelerator: 'CmdOrCtrl+C',
          role: 'copy'
        },
        {
          label: '粘贴',
          accelerator: 'CmdOrCtrl+V',
          role: 'paste'
        }
      ]
    },
    {
      label: '查看',
      submenu: [
        {
          label: '重新加载',
          accelerator: 'CmdOrCtrl+R',
          click: (_item: any, focusedWindow: any) => {
            if (focusedWindow) focusedWindow.reload();
          }
        },
        {
          label: '强制重新加载',
          accelerator: 'CmdOrCtrl+Shift+R',
          click: (_item: any, focusedWindow: any) => {
            if (focusedWindow) focusedWindow.webContents.reloadIgnoringCache();
          }
        },
        {
          label: '切换开发者工具',
          accelerator: process.platform === 'darwin' ? 'Alt+Cmd+I' : 'Ctrl+Shift+I',
          click: (_item: any, focusedWindow: any) => {
            if (focusedWindow) focusedWindow.webContents.toggleDevTools();
          }
        },
        { type: 'separator' },
        {
          label: '实际大小',
          accelerator: 'CmdOrCtrl+0',
          role: 'resetZoom'
        },
        {
          label: '放大',
          accelerator: 'CmdOrCtrl+Plus',
          role: 'zoomIn'
        },
        {
          label: '缩小',
          accelerator: 'CmdOrCtrl+-',
          role: 'zoomOut'
        },
        { type: 'separator' },
        {
          label: '切换全屏',
          accelerator: process.platform === 'darwin' ? 'Ctrl+Cmd+F' : 'F11',
          click: (_item: any, focusedWindow: any) => {
            if (focusedWindow) focusedWindow.setFullScreen(!focusedWindow.isFullScreen());
          }
        }
      ]
    },
    {
      label: '窗口',
      submenu: [
        {
          label: '最小化',
          accelerator: 'CmdOrCtrl+M',
          role: 'minimize'
        },
        {
          label: '关闭',
          accelerator: 'CmdOrCtrl+W',
          role: 'close'
        }
      ]
    },
    {
      label: '帮助',
      submenu: [
        {
          label: '关于',
          click: () => {
            console.log('关于应用程序');
          }
        }
      ]
    }
  ];

  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);
};

const createWindow = () => {
  // 创建中文菜单栏
  createMenu();

  // 创建浏览器窗口
  const mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
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
      // 检查是否已有进程在运行
      if (runningProcesses.has(projectId)) {
        return {
          success: false,
          error: '该项目已有脚本在运行中'
        };
      }

      // 使用 npm run 命令
      const isWindows = process.platform === 'win32';
      const command = isWindows ? 'npm.cmd' : 'npm';
      const args = ['run', scriptName];

      const childProcess = spawn(command, args, {
        cwd: projectPath,
        stdio: ['pipe', 'pipe', 'pipe'],
        shell: isWindows
      });

      // 存储进程引用
      runningProcesses.set(projectId, childProcess);

      // 处理进程输出
      childProcess.stdout?.on('data', (data) => {
        console.log(`[${projectId}] ${data.toString()}`);
      });

      childProcess.stderr?.on('data', (data) => {
        console.error(`[${projectId}] ${data.toString()}`);
      });

      // 处理进程结束
      childProcess.on('close', (code) => {
        console.log(`[${projectId}] 进程结束，退出码: ${code}`);
        runningProcesses.delete(projectId);
      });

      childProcess.on('error', (error) => {
        console.error(`[${projectId}] 进程错误:`, error);
        runningProcesses.delete(projectId);
      });

      return {
        success: true,
        data: {
          pid: childProcess.pid,
          message: `脚本 "${scriptName}" 已启动`
        }
      };
    } catch (error) {
      return {
        success: false,
        error: `启动脚本时出错: ${error instanceof Error ? error.message : String(error)}`
      };
    }
  });

  // 停止脚本
  ipcMain.handle('stop-script', async (_, projectId: string) => {
    try {
      const process = runningProcesses.get(projectId);

      if (!process) {
        return {
          success: false,
          error: '未找到运行中的进程'
        };
      }

      // 终止进程
      const isWindows = process.platform === 'win32';
      if (isWindows && process.pid) {
        // Windows 上使用 taskkill
        spawn('taskkill', ['/pid', process.pid.toString(), '/t', '/f']);
      } else {
        // Unix 系统使用 SIGTERM
        process.kill('SIGTERM');
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
