import { useState } from 'react';
import { X, Search, Sparkles, Bot, PenTool, Database, Code, Image as ImageIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface AgentSelectorModalProps {
  onClose: () => void;
  onSelect: (agent: any) => void;
}

const CATEGORIES = [
  { id: 'all', label: '全部智能体', icon: Sparkles },
  { id: 'writing', label: '写作辅助', icon: PenTool },
  { id: 'data', label: '数据分析', icon: Database },
  { id: 'code', label: '编程助手', icon: Code },
  { id: 'image', label: '图像处理', icon: ImageIcon },
];

const AGENTS = [
  { id: '1', category: 'writing', name: '文章润色专家', desc: '专业的文本润色，提升文章可读性、流畅度和专业语感。', icon: '✨' },
  { id: '2', category: 'writing', name: '总结助手', desc: '快速提取长文核心要点，生成简洁明了的摘要内容。', icon: '📝' },
  { id: '3', category: 'writing', name: '标题生成器', desc: '为你撰写吸引眼球的公众号文章标题和副标题。', icon: '📌' },
  { id: '4', category: 'code', name: '代码解释器', desc: '解释复杂的代码片段，发现漏洞并提供优化建议。', icon: '💻' },
  { id: '5', category: 'data', name: '数据洞察', desc: '对数据报表进行梳理分析，得出数据背后的含义。', icon: '📊' },
  { id: '6', category: 'image', name: '配图建议', desc: '根据文本内容，建议应该配什么样的插图。', icon: '🖼️' },
  { id: '7', category: 'all', name: '通用全能助手', desc: '解答你的各种日常问题与知识百科。', icon: '🤖' },
];

export function AgentSelectorModal({ onClose, onSelect }: AgentSelectorModalProps) {
void (useTranslation());
  const [activeCategory, setActiveCategory] = useState('all');
  const [searchQuery, setSearchQuery] = useState('');

  const filteredAgents = AGENTS.filter(agent => {
    const matchesCategory = activeCategory === 'all' || agent.category === activeCategory;
    const matchesSearch = agent.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
                          agent.desc.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  return (
    <div className="fixed inset-0 z-[400] bg-zinc-950/40 flex items-center justify-center backdrop-blur-md" onClick={onClose}>
      <div className="w-[800px] h-[550px] bg-[var(--color-kb-panel)] rounded-2xl shadow-2xl flex overflow-hidden border border-[var(--color-kb-panel-border)] animate-in zoom-in-95 duration-200" onClick={e => e.stopPropagation()}>
        
        {/* Left Sidebar */}
        <div className="w-[200px] border-r border-[var(--color-kb-panel-border)] bg-[var(--color-kb-editor)] flex flex-col">
          <div className="h-14 flex items-center px-4 border-b border-transparent text-[var(--color-kb-text-heading)] font-medium text-base">
            选择智能体
          </div>
          <div className="flex-1 overflow-y-auto p-2 space-y-1">
            {CATEGORIES.map(cat => {
              const Icon = cat.icon;
              return (
                <button
                  key={cat.id}
                  onClick={() => setActiveCategory(cat.id)}
                  className={`w-full flex items-center px-3 py-2.5 rounded-lg text-sm transition-colors ${
                    activeCategory === cat.id 
                      ? 'bg-[var(--color-kb-panel-active)] text-[var(--color-kb-panel-text)] font-medium' 
                      : 'text-[var(--color-kb-text)] hover:bg-[var(--color-kb-panel-hover)]'
                  }`}
                >
                  <Icon size={16} className={`mr-3 ${activeCategory === cat.id ? 'text-[var(--color-kb-accent)]' : 'text-[var(--color-kb-text-muted)]'}`} />
                  {cat.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Right Content */}
        <div className="flex-1 flex flex-col bg-[var(--color-kb-panel)]">
          <div className="h-14 border-b border-[var(--color-kb-panel-border)] flex items-center justify-between px-6">
            <div className="relative w-64">
              <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--color-kb-text-muted)]" />
              <input
                type="text"
                placeholder="搜索智能体..."
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                className="w-full bg-[var(--color-kb-input-bg)] text-[var(--color-kb-text)] text-sm rounded-full pl-9 pr-4 py-1.5 focus:outline-none focus:ring-1 focus:ring-[var(--color-kb-accent)]"
              />
            </div>
            <button onClick={onClose} className="p-1.5 rounded-full hover:bg-[var(--color-kb-panel-hover)] text-[var(--color-kb-text-muted)] transition-colors">
              <X size={18} />
            </button>
          </div>
          
          <div className="flex-1 overflow-y-auto p-6">
            <div className="grid grid-cols-2 gap-4">
              {filteredAgents.map(agent => (
                <div 
                  key={agent.id}
                  onClick={() => onSelect(agent)}
                  className="p-4 border border-[var(--color-kb-panel-border)] rounded-xl hover:border-[var(--color-kb-accent)] hover:shadow-sm bg-[var(--color-kb-editor)] cursor-pointer transition-all group relative overflow-hidden"
                >
                  <div className="flex items-start">
                    <div className="text-3xl mr-4">{agent.icon}</div>
                    <div>
                      <h4 className="text-[var(--color-kb-text-heading)] font-medium mb-1 group-hover:text-[var(--color-kb-accent)] transition-colors">{agent.name}</h4>
                      <p className="text-[13px] text-[var(--color-kb-text-muted)] leading-relaxed line-clamp-2">{agent.desc}</p>
                    </div>
                  </div>
                </div>
              ))}
              {filteredAgents.length === 0 && (
                <div className="col-span-2 py-12 flex flex-col items-center justify-center text-[var(--color-kb-text-muted)]">
                  <Bot size={32} className="mb-3 opacity-50" />
                  <p>没有找到相关智能体</p>
                </div>
              )}
            </div>
          </div>
        </div>
        
      </div>
    </div>
  );
}
