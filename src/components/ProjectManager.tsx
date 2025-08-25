import React, { useState, useEffect } from 'react';
import { Project, NpmScript } from '../types/project';
import './ProjectManager.css';

const ProjectManager: React.FC = () => {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // 加载项目配置
  useEffect(() => {
    loadProjects();
  }, []);

  const loadProjects = async () => {
    try {
      const result = await window.electronAPI.loadProjectConfig();
      if (result.success && result.data) {
        setProjects(result.data.projects);
      } else {
        showMessage('error', result.error || '加载项目配置失败');
      }
    } catch (error) {
      showMessage('error', '加载项目配置失败');
    }
  };

  const showMessage = (type: 'success' | 'error', text: string) => {
    setMessage({ type, text });
    setTimeout(() => setMessage(null), 3000);
  };

  // 添加项目
  const handleAddProject = async () => {
    setLoading(true);
    try {
      const result = await window.electronAPI.selectFolder();

      if (result.success && result.data) {
        const addResult = await window.electronAPI.addProjectToConfig(result.data.path);
        if (addResult.success && addResult.data) {
          setProjects(prev => [...prev, addResult.data]);
          showMessage('success', `项目 "${addResult.data.name}" 添加成功`);
        } else {
          showMessage('error', addResult.error || '添加项目失败');
        }
      } else {
        if (result.error && !result.error.includes('用户取消')) {
          showMessage('error', result.error);
        }
      }
    } catch (error) {
      showMessage('error', '添加项目时出错');
    } finally {
      setLoading(false);
    }
  };

  // 删除项目
  const handleRemoveProject = async (projectId: string) => {
    if (window.confirm('确定要删除这个项目吗？')) {
      try {
        const result = await window.electronAPI.removeProjectFromConfig(projectId);
        if (result.success) {
          setProjects(prev => prev.filter(p => p.id !== projectId));
          showMessage('success', '项目删除成功');
        } else {
          showMessage('error', result.error || '删除项目失败');
        }
      } catch (error) {
        showMessage('error', '删除项目时出错');
      }
    }
  };

  // 选择脚本
  const handleScriptChange = async (projectId: string, scriptName: string) => {
    // 更新本地状态
    const updatedProjects = projects.map(project =>
      project.id === projectId
        ? { ...project, selectedScript: scriptName }
        : project
    );
    setProjects(updatedProjects);

    // 保存配置到主进程
    try {
      const config = {
        projects: updatedProjects,
        lastUpdated: new Date()
      };
      await window.electronAPI.saveProjectConfig(config);
    } catch (error) {
      console.error('保存配置失败:', error);
    }
  };

  // 运行脚本
  const handleRunScript = async (project: Project) => {
    if (!project.selectedScript) {
      showMessage('error', '请先选择要运行的脚本');
      return;
    }

    setLoading(true);
    try {
      const result = await window.electronAPI.runScript({
        projectPath: project.path,
        scriptName: project.selectedScript,
        projectId: project.id
      });

      if (result.success) {
        setProjects(prev => prev.map(p => 
          p.id === project.id 
            ? { ...p, isRunning: true, lastRunTime: new Date() }
            : p
        ));
        showMessage('success', `脚本 "${project.selectedScript}" 启动成功`);
      } else {
        showMessage('error', result.error || '启动脚本失败');
      }
    } catch (error) {
      showMessage('error', '启动脚本时出错');
    } finally {
      setLoading(false);
    }
  };

  // 停止脚本
  const handleStopScript = async (project: Project) => {
    setLoading(true);
    try {
      const result = await window.electronAPI.stopScript(project.id);

      if (result.success) {
        setProjects(prev => prev.map(p => 
          p.id === project.id 
            ? { ...p, isRunning: false }
            : p
        ));
        showMessage('success', '脚本已停止');
      } else {
        showMessage('error', result.error || '停止脚本失败');
      }
    } catch (error) {
      showMessage('error', '停止脚本时出错');
    } finally {
      setLoading(false);
    }
  };

  // 检查脚本状态
  const checkScriptStatus = async (projectId: string) => {
    try {
      const result = await window.electronAPI.checkScriptStatus(projectId);
      if (result.success) {
        setProjects(prev => prev.map(p => 
          p.id === projectId 
            ? { ...p, isRunning: result.data.isRunning }
            : p
        ));
      }
    } catch (error) {
      console.error('检查脚本状态失败:', error);
    }
  };

  // 定期检查脚本状态
  useEffect(() => {
    const interval = setInterval(() => {
      projects.forEach(project => {
        if (project.isRunning) {
          checkScriptStatus(project.id);
        }
      });
    }, 5000);

    return () => clearInterval(interval);
  }, [projects]);

  return (
    <div className="project-manager">
      <div className="project-manager-header">
        <h2>项目管理</h2>
        <div className="header-actions">
          <button 
            className="btn btn-primary" 
            onClick={handleAddProject}
            disabled={loading}
          >
            {loading ? '添加中...' : '📁 添加项目'}
          </button>
          <button 
            className="btn btn-secondary" 
            onClick={loadProjects}
            disabled={loading}
          >
            🔄 刷新
          </button>
        </div>
      </div>

      {message && (
        <div className={`message ${message.type}`}>
          {message.text}
        </div>
      )}

      <div className="projects-container">
        {projects.length === 0 ? (
          <div className="empty-state">
            <p>还没有添加任何项目</p>
            <p>点击"添加项目"按钮开始管理你的项目</p>
          </div>
        ) : (
          <div className="projects-table">
            <div className="table-header">
              <div className="col-name">项目名称</div>
              <div className="col-path">项目路径</div>
              <div className="col-script">npm 脚本</div>
              <div className="col-actions">操作</div>
            </div>
            
            {projects.map(project => (
              <div key={project.id} className="table-row">
                <div className="col-name">
                  <div className="project-name">
                    <span className="name">{project.name}</span>
                    {project.isRunning && (
                      <span className="status running">运行中</span>
                    )}
                  </div>
                </div>
                
                <div className="col-path">
                  <span className="path" title={project.path}>
                    {project.path}
                  </span>
                </div>
                
                <div className="col-script">
                  <select
                    value={project.selectedScript || ''}
                    onChange={(e) => handleScriptChange(project.id, e.target.value)}
                    className="script-select"
                    disabled={project.scripts.length === 0}
                  >
                    <option value="">选择脚本</option>
                    {project.scripts.map(script => (
                      <option key={script.name} value={script.name}>
                        {script.name}
                      </option>
                    ))}
                  </select>
                </div>
                
                <div className="col-actions">
                  <div className="action-buttons">
                    {project.isRunning ? (
                      <button
                        className="btn btn-danger btn-sm"
                        onClick={() => handleStopScript(project)}
                        disabled={loading}
                      >
                        ⏹️ 停止
                      </button>
                    ) : (
                      <button
                        className="btn btn-success btn-sm"
                        onClick={() => handleRunScript(project)}
                        disabled={loading || !project.selectedScript}
                      >
                        ▶️ 运行
                      </button>
                    )}
                    
                    <button
                      className="btn btn-danger btn-sm"
                      onClick={() => handleRemoveProject(project.id)}
                      disabled={loading}
                    >
                      🗑️ 删除
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default ProjectManager;
