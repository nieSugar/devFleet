import { app, BrowserWindow, Menu } from 'electron';
import path from 'node:path';
import started from 'electron-squirrel-startup';

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

// 当 Electron 完成初始化并准备创建浏览器窗口时，将调用此方法
// 某些 API 只能在此事件发生后使用
app.on('ready', createWindow);

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
