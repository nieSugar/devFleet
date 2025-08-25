import React, { useState } from 'react';
import TodoList from './components/TodoList';
import './App.css';

const App: React.FC = () => {
  const [count, setCount] = useState(0);
  const [currentTab, setCurrentTab] = useState<'counter' | 'todo'>('counter');

  return (
    <div className="app">
      <header className="app-header">
        <h1>💖 你好世界！</h1>
        <p>欢迎使用您的 React + Electron 应用程序</p>

        <div className="tab-buttons">
          <button
            className={`tab-btn ${currentTab === 'counter' ? 'active' : ''}`}
            onClick={() => setCurrentTab('counter')}
          >
            🔢 计数器
          </button>
          <button
            className={`tab-btn ${currentTab === 'todo' ? 'active' : ''}`}
            onClick={() => setCurrentTab('todo')}
          >
            📝 待办事项
          </button>
        </div>
      </header>

      <main className="app-main">
        {currentTab === 'counter' ? (
          <>
            <div className="counter-section">
              <h2>计数器示例</h2>
              <div className="counter">
                <button
                  className="counter-btn"
                  onClick={() => setCount(count - 1)}
                >
                  -
                </button>
                <span className="counter-value">{count}</span>
                <button
                  className="counter-btn"
                  onClick={() => setCount(count + 1)}
                >
                  +
                </button>
              </div>
              <button
                className="reset-btn"
                onClick={() => setCount(0)}
              >
                重置
              </button>
            </div>

            <div className="info-section">
              <h2>技术栈</h2>
              <ul>
                <li>⚛️ React 18</li>
                <li>⚡ Vite</li>
                <li>🖥️ Electron</li>
                <li>📘 TypeScript</li>
              </ul>
            </div>
          </>
        ) : (
          <TodoList />
        )}
      </main>
    </div>
  );
};

export default App;
