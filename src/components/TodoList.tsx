import React, { useState, useEffect } from 'react';
import './TodoList.css';

interface Todo {
  id: number;
  text: string;
  completed: boolean;
  createdAt: Date;
}

const TodoList: React.FC = () => {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [filter, setFilter] = useState<'all' | 'active' | 'completed'>('all');

  // 从 localStorage 加载数据
  useEffect(() => {
    const savedTodos = localStorage.getItem('electron-todos');
    if (savedTodos) {
      const parsedTodos = JSON.parse(savedTodos).map((todo: any) => ({
        ...todo,
        createdAt: new Date(todo.createdAt)
      }));
      setTodos(parsedTodos);
    }
  }, []);

  // 保存到 localStorage
  useEffect(() => {
    localStorage.setItem('electron-todos', JSON.stringify(todos));
  }, [todos]);

  const addTodo = () => {
    if (inputValue.trim()) {
      const newTodo: Todo = {
        id: Date.now(),
        text: inputValue.trim(),
        completed: false,
        createdAt: new Date()
      };
      setTodos([...todos, newTodo]);
      setInputValue('');
    }
  };

  const toggleTodo = (id: number) => {
    setTodos(todos.map(todo =>
      todo.id === id ? { ...todo, completed: !todo.completed } : todo
    ));
  };

  const deleteTodo = (id: number) => {
    setTodos(todos.filter(todo => todo.id !== id));
  };

  const clearCompleted = () => {
    setTodos(todos.filter(todo => !todo.completed));
  };

  const filteredTodos = todos.filter(todo => {
    switch (filter) {
      case 'active':
        return !todo.completed;
      case 'completed':
        return todo.completed;
      default:
        return true;
    }
  });

  const completedCount = todos.filter(todo => todo.completed).length;
  const activeCount = todos.length - completedCount;

  return (
    <div className="todo-container">
      <h2>📝 待办事项</h2>
      
      <div className="todo-input-section">
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyPress={(e) => e.key === 'Enter' && addTodo()}
          placeholder="添加新的待办事项..."
          className="todo-input"
        />
        <button onClick={addTodo} className="add-btn">
          添加
        </button>
      </div>

      <div className="todo-filters">
        <button
          className={filter === 'all' ? 'filter-btn active' : 'filter-btn'}
          onClick={() => setFilter('all')}
        >
          全部 ({todos.length})
        </button>
        <button
          className={filter === 'active' ? 'filter-btn active' : 'filter-btn'}
          onClick={() => setFilter('active')}
        >
          未完成 ({activeCount})
        </button>
        <button
          className={filter === 'completed' ? 'filter-btn active' : 'filter-btn'}
          onClick={() => setFilter('completed')}
        >
          已完成 ({completedCount})
        </button>
      </div>

      <div className="todo-list">
        {filteredTodos.length === 0 ? (
          <div className="empty-state">
            {filter === 'all' ? '暂无待办事项' : 
             filter === 'active' ? '没有未完成的事项' : '没有已完成的事项'}
          </div>
        ) : (
          filteredTodos.map(todo => (
            <div key={todo.id} className={`todo-item ${todo.completed ? 'completed' : ''}`}>
              <input
                type="checkbox"
                checked={todo.completed}
                onChange={() => toggleTodo(todo.id)}
                className="todo-checkbox"
              />
              <span className="todo-text">{todo.text}</span>
              <span className="todo-date">
                {todo.createdAt.toLocaleDateString('zh-CN')}
              </span>
              <button
                onClick={() => deleteTodo(todo.id)}
                className="delete-btn"
              >
                删除
              </button>
            </div>
          ))
        )}
      </div>

      {completedCount > 0 && (
        <div className="todo-actions">
          <button onClick={clearCompleted} className="clear-btn">
            清除已完成 ({completedCount})
          </button>
        </div>
      )}
    </div>
  );
};

export default TodoList;
