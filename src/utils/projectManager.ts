import { Project, NpmScript, ProjectConfig } from '../types/project';
import * as fs from 'fs';
import * as path from 'path';

const CONFIG_FILE = 'devfleet-config.json';

/**
 * 读取项目的 package.json 文件并提取 npm 脚本
 */
export function getPackageScripts(projectPath: string): NpmScript[] {
  try {
    const packageJsonPath = path.join(projectPath, 'package.json');
    
    if (!fs.existsSync(packageJsonPath)) {
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
    const packageJsonPath = path.join(projectPath, 'package.json');
    
    if (fs.existsSync(packageJsonPath)) {
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
      if (packageJson.name) {
        return packageJson.name;
      }
    }
    
    // 如果没有 package.json 或没有 name 字段，使用文件夹名称
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
    if (!fs.existsSync(projectPath)) {
      return false;
    }

    // 检查是否是目录
    const stats = fs.statSync(projectPath);
    if (!stats.isDirectory()) {
      return false;
    }

    // 检查是否包含 package.json
    const packageJsonPath = path.join(projectPath, 'package.json');
    return fs.existsSync(packageJsonPath);
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

  return {
    id,
    name,
    path: projectPath,
    scripts,
    isRunning: false
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
    
    if (fs.existsSync(configPath)) {
      const configData = fs.readFileSync(configPath, 'utf-8');
      const config = JSON.parse(configData);
      
      // 验证并更新项目信息
      const validProjects = config.projects.filter((project: Project) => {
        if (isValidProjectPath(project.path)) {
          // 更新脚本信息
          project.scripts = getPackageScripts(project.path);
          return true;
        }
        return false;
      });

      return {
        projects: validProjects,
        lastUpdated: new Date(config.lastUpdated || Date.now())
      };
    }
  } catch (error) {
    console.error('Error loading project config:', error);
  }

  return {
    projects: [],
    lastUpdated: new Date()
  };
}

/**
 * 保存项目配置
 */
export function saveProjectConfig(config: ProjectConfig): boolean {
  try {
    const configPath = getConfigPath();
    const configData = JSON.stringify({
      ...config,
      lastUpdated: new Date()
    }, null, 2);
    
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
  if (!fs.existsSync(appDataPath)) {
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
