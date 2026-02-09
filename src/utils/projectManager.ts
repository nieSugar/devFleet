import { Project, NpmScript, ProjectConfig, AppSettings } from '../types/project';
import * as fs from 'fs';
import * as path from 'path';

const CONFIG_FILE = 'devfleet-config.json';

/**
 * 读取项目的 package.json 文件并提取 npm 脚本
 */
export function getPackageScripts(projectPath: string): NpmScript[] {
  try {
    const packageJsonPath = path.join(projectPath, 'package.json');
    
    try {
      fs.accessSync(packageJsonPath, fs.constants.F_OK);
    } catch {
      return [];
    }

    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const scripts = packageJson.scripts || {};

    return Object.entries(scripts).map(([name, command]) => ({
      name,
      command: command as string
    }));
  } catch (error) {
    console.error('Error reading package.json:', error);
    return [];
  }
}

/**
 * 获取项目名称（从 package.json 或文件夹名称）
 */
export function getProjectName(projectPath: string): string {
  try {
    // 采用package.json的名称
    // const packageJsonPath = path.join(projectPath, 'package.json');
    
    // try {
    //   fs.accessSync(packageJsonPath, fs.constants.F_OK);
    //   const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    //   if (packageJson.name) {
    //     return packageJson.name;
    //   }
    // } catch {
    //   // package.json 不存在或无法读取
    // }
    
    return path.basename(projectPath);
  } catch (error) {
    console.error('Error getting project name:', error);
    return path.basename(projectPath);
  }
}

/**
 * 验证路径是否是有效的项目目录
 */
export function isValidProjectPath(projectPath: string): boolean {
  try {
    // 检查路径是否存在
    try {
      fs.accessSync(projectPath, fs.constants.F_OK);
    } catch {
      return false;
    }

    // 检查是否是目录
    const stats = fs.statSync(projectPath);
    if (!stats.isDirectory()) {
      return false;
    }

    // 检查是否包含 package.json
    const packageJsonPath = path.join(projectPath, 'package.json');
    try {
      fs.accessSync(packageJsonPath, fs.constants.F_OK);
      return true;
    } catch {
      return false;
    }
  } catch (error) {
    console.error('Error validating project path:', error);
    return false;
  }
}

/**
 * 创建新项目对象
 */
export function createProject(projectPath: string): Project | null {
  if (!isValidProjectPath(projectPath)) {
    return null;
  }

  const id = generateProjectId();
  const name = getProjectName(projectPath);
  const scripts = getPackageScripts(projectPath);
  const selectedScript = scripts[0]?.name;
  const nodeVersion = getProjectNodeVersion(projectPath);

  return {
    id,
    name,
    path: projectPath,
    scripts,
    selectedScript,
    isRunning: false,
    nodeVersion: nodeVersion || undefined
  };
}

/**
 * 生成唯一的项目ID
 */
function generateProjectId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substr(2);
}

/**
 * 加载项目配置
 */
export function loadProjectConfig(): ProjectConfig {
  try {
    const configPath = getConfigPath();
    
    try {
      fs.accessSync(configPath, fs.constants.F_OK);
      const configData = fs.readFileSync(configPath, 'utf-8');
      const config = JSON.parse(configData);
      
      // 验证并更新项目信息
      const validProjects = config.projects.filter((project: Project) => {
        if (isValidProjectPath(project.path)) {
          // 更新脚本信息
          project.scripts = getPackageScripts(project.path);
          // 如果项目没有配置 nodeVersion，自动检测并设置
          if (!project.nodeVersion) {
            const detectedVersion = getProjectNodeVersion(project.path);
            if (detectedVersion) {
              project.nodeVersion = detectedVersion;
            }
          }
          // 兼容从 JSON 恢复的时间字段
          if ((project as any).lastRunTime) {
            try { (project as any).lastRunTime = new Date((project as any).lastRunTime); } catch (_e) { /* ignore invalid date */ }
          }
          return true;
        }
        return false;
      });

      return {
        projects: validProjects,
        lastUpdated: new Date(config.lastUpdated || Date.now())
      };
    } catch {
      // 配置文件不存在，返回默认配置
    }
  } catch (error) {
    console.error('Error loading project config:', error);
  }

  return {
    projects: [],
    lastUpdated: new Date(),
    settings: { runInExternalTerminal: true }
  };
}

/**
 * 保存项目配置
 */
export function saveProjectConfig(config: ProjectConfig): boolean {
  try {
    const configPath = getConfigPath();
    // 合并默认设置
    const defaults: AppSettings = { runInExternalTerminal: false };
    const merged: ProjectConfig = {
      ...config,
      settings: { ...defaults, ...(config.settings || {}) },
      lastUpdated: new Date()
    };

    const configData = JSON.stringify(merged, null, 2);
    
    fs.writeFileSync(configPath, configData, 'utf-8');
    return true;
  } catch (error) {
    console.error('Error saving project config:', error);
    return false;
  }
}

/**
 * 获取配置文件路径
 */
function getConfigPath(): string {
  const userDataPath = process.env.APPDATA || 
    (process.platform === 'darwin' ? process.env.HOME + '/Library/Application Support' : process.env.HOME + '/.local/share');
  
  const appDataPath = path.join(userDataPath, 'devfleet');
  
  // 确保目录存在
  try {
    fs.accessSync(appDataPath, fs.constants.F_OK);
  } catch {
    fs.mkdirSync(appDataPath, { recursive: true });
  }
  
  return path.join(appDataPath, CONFIG_FILE);
}

/**
 * 添加项目到配置
 */
export function addProjectToConfig(projectPath: string): Project | null {
  const project = createProject(projectPath);
  if (!project) {
    return null;
  }

  const config = loadProjectConfig();
  
  // 检查项目是否已存在
  const existingProject = config.projects.find(p => p.path === projectPath);
  if (existingProject) {
    return existingProject;
  }

  config.projects.push(project);
  saveProjectConfig(config);
  
  return project;
}

/**
 * 从配置中移除项目
 */
export function removeProjectFromConfig(projectId: string): boolean {
  const config = loadProjectConfig();
  const initialLength = config.projects.length;

  config.projects = config.projects.filter(p => p.id !== projectId);

  if (config.projects.length < initialLength) {
    saveProjectConfig(config);
    return true;
  }

  return false;
}

/**
 * 获取项目推荐的 Node 版本
 * 从 .nvmdrc、.node-version、.nvmrc 或 package.json 的 engines.node 字段读取
 */
export function getProjectNodeVersion(projectPath: string): string | null {
  try {
    // 1. 尝试读取 .nvmdrc 文件（nvmd 专用）
    const nvmdrcPath = path.join(projectPath, '.nvmdrc');
    try {
      fs.accessSync(nvmdrcPath, fs.constants.F_OK);
      const nvmdrcContent = fs.readFileSync(nvmdrcPath, 'utf-8').trim();
      if (nvmdrcContent) {
        // 移除可能的 'v' 前缀
        return nvmdrcContent.replace(/^v/, '');
      }
    } catch {
      // .nvmdrc 不存在，继续尝试其他方法
    }

    // 2. 尝试读取 .node-version 文件（nvs 首选）
    const nodeVersionPath = path.join(projectPath, '.node-version');
    try {
      fs.accessSync(nodeVersionPath, fs.constants.F_OK);
      const nodeVersionContent = fs.readFileSync(nodeVersionPath, 'utf-8').trim();
      if (nodeVersionContent) {
        // 移除可能的 'v' 前缀
        return nodeVersionContent.replace(/^v/, '');
      }
    } catch {
      // .node-version 不存在，继续尝试其他方法
    }

    // 3. 尝试读取 .nvmrc 文件
    const nvmrcPath = path.join(projectPath, '.nvmrc');
    try {
      fs.accessSync(nvmrcPath, fs.constants.F_OK);
      const nvmrcContent = fs.readFileSync(nvmrcPath, 'utf-8').trim();
      if (nvmrcContent) {
        // 移除可能的 'v' 前缀
        return nvmrcContent.replace(/^v/, '');
      }
    } catch {
      // .nvmrc 不存在，继续尝试其他方法
    }

    // 4. 尝试从 package.json 的 engines.node 读取
    const packageJsonPath = path.join(projectPath, 'package.json');
    try {
      fs.accessSync(packageJsonPath, fs.constants.F_OK);
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

      if (packageJson.engines && packageJson.engines.node) {
        const nodeVersion = packageJson.engines.node;
        // 尝试提取具体版本号（例如 ">=18.0.0" -> "18.0.0"）
        const match = nodeVersion.match(/(\d+\.\d+\.\d+)/);
        if (match) {
          return match[1];
        }
        // 如果是范围表达式，尝试提取主版本号
        const majorMatch = nodeVersion.match(/(\d+)/);
        if (majorMatch) {
          return majorMatch[1];
        }
      }
    } catch {
      // package.json 不存在或读取失败
    }

    return null;
  } catch (error) {
    console.error('获取项目 Node 版本失败:', error);
    return null;
  }
}

/**
 * 创建、更新或删除项目的 Node 版本配置文件
 * @param projectPath 项目路径
 * @param nodeVersion Node 版本号（如 "18.0.0"），传入空字符串或 null 时删除配置文件
 * @param versionManager 版本管理器类型
 * @returns 成功返回 true，失败返回 false
 */
export function setProjectNodeVersionFile(
  projectPath: string,
  nodeVersion: string | null,
  versionManager: 'nvmd' | 'nvm-windows' | 'nvm' | 'nvs'
): boolean {
  try {
    // 根据版本管理器类型决定文件名
    // nvmd 使用 .nvmdrc
    // nvs 优先使用 .node-version (也支持 .nvmrc)
    // nvm/nvm-windows 使用 .nvmrc
    let fileName: string;
    if (versionManager === 'nvmd') {
      fileName = '.nvmdrc';
    } else if (versionManager === 'nvs') {
      fileName = '.node-version';
    } else {
      fileName = '.nvmrc';
    }

    const filePath = path.join(projectPath, fileName);

    // 如果 nodeVersion 为空，删除配置文件
    if (!nodeVersion || nodeVersion.trim() === '') {
      try {
        fs.accessSync(filePath, fs.constants.F_OK);
        fs.unlinkSync(filePath);
        console.log(`已删除 ${fileName} 文件`);
        return true;
      } catch {
        // 文件不存在，视为成功
        console.log(`${fileName} 文件不存在，无需删除`);
        return true;
      }
    }

    // 写入版本号（不带 'v' 前缀）
    const content = nodeVersion.replace(/^v/, '');
    fs.writeFileSync(filePath, content, 'utf-8');

    console.log(`已创建/更新 ${fileName} 文件，版本: ${content}`);
    return true;
  } catch (error) {
    console.error('操作版本配置文件失败:', error);
    return false;
  }
}
