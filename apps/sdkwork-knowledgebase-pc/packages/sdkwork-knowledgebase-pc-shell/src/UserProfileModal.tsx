import React, { useState, useEffect, useRef } from 'react';
import { isBlank } from '@sdkwork/utils';
import { X, Camera, Mail, Phone, Briefcase, Info, Clock, Check, Edit2, AlertCircle, Sparkles, Smile, ShieldCheck } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useLocalStorage } from '@sdkwork/sdkwork-knowledgebase-pc-commons';
import { toast } from '@sdkwork/sdkwork-knowledgebase-pc-knowledgebase';

export interface UserProfile {
  name: string;
  avatar: string;
  title: string;
  department: string;
  email: string;
  phone: string;
  bio: string;
  timezone: string;
  status: 'online' | 'busy' | 'away' | 'offline';
  statusText?: string;
}

export interface UserProfileModalProps {
  account?: import('sdkwork-knowledgebase-pc-core').KnowledgebaseAccountViewModel;
  isOpen: boolean;
  onClose: () => void;
}

const PRESET_AVATARS = [
  'https://api.dicebear.com/7.x/notionists/svg?seed=Felix&backgroundColor=f0d9b5',
  'https://api.dicebear.com/7.x/notionists/svg?seed=Aneka&backgroundColor=c0aede',
  'https://api.dicebear.com/7.x/notionists/svg?seed=Milo&backgroundColor=b6e3f4',
  'https://api.dicebear.com/7.x/notionists/svg?seed=Koko&backgroundColor=ffd5dc',
  'https://api.dicebear.com/7.x/notionists/svg?seed=Loki&backgroundColor=d1f4c9',
  'https://api.dicebear.com/7.x/notionists/svg?seed=Nala&backgroundColor=ffdfbf',
];

const EMOTE_PRESETS = ['👩‍💻', '👨‍💻', '🤖', '🦊', '🎨', '🚀', '🌟', '🍀', '✨', '☕'];

const QUICK_STATUS_PRESETS = [
  { text: '💻 专注于写作', status: 'busy' as const },
  { text: '☕ 稍作休息', status: 'away' as const },
  { text: '📅 会议进行中', status: 'busy' as const },
  { text: '💡 沉淀灵感中', status: 'online' as const },
  { text: '🏡 远程办公中', status: 'online' as const },
  { text: '🌴 暂离休假', status: 'offline' as const },
];

export const DEFAULT_USER_PROFILE: UserProfile = {
  name: '宋语冰 (Alice)',
  avatar: PRESET_AVATARS[0],
  title: '高级知识架构师',
  department: '数字化体验与知识管理部',
  email: 'alice@company.com',
  phone: '138-1234-5678',
  bio: '格物致知，知行合一。专注于构建企业数字化知识管理与智能协同系统。',
  timezone: 'GMT+8 (北京时间)',
  status: 'online',
  statusText: '在架写作中 ✍️'
};

export function UserProfileModal({ isOpen, onClose }: UserProfileModalProps) {
  const { t } = useTranslation('shell');
  const [profile, setProfile] = useLocalStorage<UserProfile>('app-user-profile', DEFAULT_USER_PROFILE);
  const [isEditing, setIsEditing] = useState(false);
  const [activeTab, setActiveTab] = useState<'profile' | 'statistics'>('profile');
  
  // Edit form states
  const [editName, setEditName] = useState('');
  const [editAvatar, setEditAvatar] = useState('');
  const [editTitle, setEditTitle] = useState('');
  const [editDepartment, setEditDepartment] = useState('');
  const [editEmail, setEditEmail] = useState('');
  const [editPhone, setEditPhone] = useState('');
  const [editBio, setEditBio] = useState('');
  const [editTimezone, setEditTimezone] = useState('');
  const [editStatus, setEditStatus] = useState<'online' | 'busy' | 'away' | 'offline'>('online');
  const [editStatusText, setEditStatusText] = useState('');
  
  const [dragOver, setDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Initialize edit form values when edit mode starts or profile changes
  useEffect(() => {
    if (profile) {
      setEditName(profile.name || '');
      setEditAvatar(profile.avatar || '');
      setEditTitle(profile.title || '');
      setEditDepartment(profile.department || '');
      setEditEmail(profile.email || '');
      setEditPhone(profile.phone || '');
      setEditBio(profile.bio || '');
      setEditTimezone(profile.timezone || t('beijingTime', { defaultValue: 'GMT+8 (北京时间)' }));
      setEditStatus(profile.status || 'online');
      setEditStatusText(profile.statusText || '');
    }
  }, [profile, isEditing, t]);

  if (!isOpen) return null;

  const handleSave = () => {
    if (isBlank(editName)) {
      toast.error(t('nameRequired', { defaultValue: '用户昵称/姓名不能为空' }));
      return;
    }
    const updated: UserProfile = {
      name: editName,
      avatar: editAvatar,
      title: editTitle,
      department: editDepartment,
      email: editEmail,
      phone: editPhone,
      bio: editBio,
      timezone: editTimezone,
      status: editStatus,
      statusText: editStatusText
    };
    setProfile(updated);
    setIsEditing(false);
    toast.success(t('profileSaved', { defaultValue: '个人资料保存成功！' }));
  };

  const handleFileUpload = (file: File) => {
    if (!file.type.startsWith('image/')) {
      toast.error(t('invalidImage', { defaultValue: '请选择有效的图片文件' }));
      return;
    }
    if (file.size > 2 * 1024 * 1024) {
      toast.error(t('imageTooLarge', { defaultValue: '图片大小不能超过 2MB' }));
      return;
    }
    const reader = new FileReader();
    reader.onload = (e) => {
      if (e.target?.result) {
        setEditAvatar(e.target.result as string);
        toast.success(t('avatarReadSuccess', { defaultValue: '头像读取成功！' }));
      }
    };
    reader.readAsDataURL(file);
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(true);
  };

  const handleDragLeave = () => {
    setDragOver(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      handleFileUpload(e.dataTransfer.files[0]);
    }
  };

  const statusMap = {
    online: { label: t('statusOnline', { defaultValue: '在线' }), color: 'bg-emerald-500', text: 'text-emerald-500' },
    busy: { label: t('statusBusy', { defaultValue: '忙碌' }), color: 'bg-rose-500', text: 'text-rose-500' },
    away: { label: t('statusAway', { defaultValue: '离开' }), color: 'bg-amber-500', text: 'text-amber-500' },
    offline: { label: t('statusOffline', { defaultValue: '离线' }), color: 'bg-zinc-400', text: 'text-zinc-500' }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-zinc-950/40 backdrop-blur-[4px] transition-opacity p-4">
      <div className="w-[580px] max-h-[90vh] bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded-2xl shadow-2xl flex flex-col overflow-hidden transform transition-all animate-in fade-in zoom-in-95 duration-200">
        
        {/* Banner with modern cover aesthetic */}
        <div className="relative h-[110px] bg-gradient-to-r from-[var(--color-kb-accent)]/85 via-[var(--color-kb-accent)]/65 to-[var(--color-kb-accent)]/45 flex-shrink-0">
          <button 
            onClick={onClose} 
            className="absolute top-4 right-4 p-1.5 rounded-full bg-black/10 hover:bg-black/20 text-white transition-colors"
            title={t('close', { defaultValue: '关闭' })}
          >
            <X size={16} />
          </button>
          
          <div className="absolute -bottom-10 left-6 flex items-end">
            <div className="relative group">
              <div className="w-[84px] h-[84px] rounded-2xl border-[3.5px] border-white dark:border-zinc-900 overflow-hidden bg-zinc-100 shadow-lg shrink-0">
                {isEditing ? (
                  <div 
                    onClick={() => fileInputRef.current?.click()}
                    onDragOver={handleDragOver}
                    onDragLeave={handleDragLeave}
                    onDrop={handleDrop}
                    className={`w-full h-full cursor-pointer relative flex flex-col items-center justify-center transition-all ${dragOver ? 'bg-[var(--color-kb-accent)]/20' : 'hover:bg-black/25'}`}
                  >
                    {EMOTE_PRESETS.includes(editAvatar) ? (
                      <span className="text-4xl">{editAvatar}</span>
                    ) : (
                      <img src={editAvatar} alt="Avatar" className="w-full h-full object-cover" />
                    )}
                    <div className="absolute inset-0 bg-black/30 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                      <Camera className="text-white" size={20} />
                    </div>
                  </div>
                ) : (
                  profile.avatar && EMOTE_PRESETS.includes(profile.avatar) ? (
                    <div className="w-full h-full flex items-center justify-center text-4xl">{profile.avatar}</div>
                  ) : (
                    <img src={profile.avatar} alt="Avatar" className="w-full h-full object-cover" />
                  )
                )}
              </div>
              
              {/* Online indicator badge */}
              <span className={`absolute -bottom-1 -right-1 w-5 h-5 rounded-full border-2 border-white dark:border-zinc-900 ${statusMap[isEditing ? editStatus : profile.status].color} flex items-center justify-center shadow-md`}>
                <span className="animate-ping absolute inline-flex h-3 w-3 rounded-full bg-white opacity-40"></span>
              </span>
            </div>

            <div className="ml-4 mb-1 pb-1">
              <div className="flex items-center space-x-2">
                <h2 className="text-lg font-bold text-zinc-900 dark:text-zinc-50 tracking-tight">
                  {isEditing ? editName : profile.name}
                </h2>
                {!isEditing && (
                  <span className="flex items-center space-x-1 px-1.5 py-0.5 rounded-md bg-[var(--color-kb-accent)]/10 text-[10px] text-[var(--color-kb-accent)] font-semibold border border-[var(--color-kb-accent)]/20">
                    <ShieldCheck size={11} />
                    <span>{t('primaryAdminBadge', { defaultValue: '主管理员' })}</span>
                  </span>
                )}
              </div>
              <p className="text-[12px] text-zinc-500 dark:text-zinc-400 font-medium">
                {isEditing ? editTitle : profile.title}
              </p>
            </div>
          </div>
        </div>

        {/* Hidden File Input */}
        <input 
          type="file" 
          ref={fileInputRef} 
          className="hidden" 
          accept="image/*" 
          onChange={(e) => {
            if (e.target.files && e.target.files[0]) {
              handleFileUpload(e.target.files[0]);
            }
          }} 
        />

        {/* Scrollable Container */}
        <div className="flex-1 overflow-y-auto mt-11 px-6 py-4 space-y-5 scrollbar-thin scrollbar-thumb-zinc-200 dark:scrollbar-thumb-zinc-800">
          
          {/* Status Bar */}
          {!isEditing && (profile.statusText || profile.status) && (
            <div className="p-3 rounded-xl bg-zinc-50 dark:bg-zinc-950/40 border border-zinc-100 dark:border-zinc-800/80 flex items-center justify-between">
              <div className="flex items-center space-x-2 text-xs">
                <span className={`w-2 h-2 rounded-full ${statusMap[profile.status].color}`} />
                <span className="font-semibold text-zinc-700 dark:text-zinc-300">
                  {t('statusColon', { defaultValue: '状态：' })}{statusMap[profile.status].label}
                </span>
                {profile.statusText && (
                  <span className="text-zinc-400 dark:text-zinc-500">|</span>
                )}
                {profile.statusText && (
                  <span className="text-zinc-600 dark:text-zinc-400 italic">“{profile.statusText}”</span>
                )}
              </div>
              <button 
                onClick={() => setIsEditing(true)} 
                className="text-xs text-[var(--color-kb-accent)] hover:text-[var(--color-kb-accent-hover)] hover:underline font-semibold flex items-center space-x-1"
              >
                <Edit2 size={12} />
                <span>{t('customStatus', { defaultValue: '自定义状态' })}</span>
              </button>
            </div>
          )}

          {isEditing ? (
            /* Editing State Form Layout */
            <div className="space-y-4">
              
              {/* Profile Photo Customizer */}
              <div className="space-y-2 p-4 bg-zinc-50 dark:bg-zinc-950/20 rounded-xl border border-zinc-100 dark:border-zinc-800/60">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-bold text-zinc-700 dark:text-zinc-300">{t('customAvatar', { defaultValue: '自定义头像' })}</label>
                  <button 
                    type="button"
                    onClick={() => fileInputRef.current?.click()}
                    className="text-[11px] text-[var(--color-kb-accent)] hover:text-[var(--color-kb-accent-hover)] font-semibold hover:underline"
                  >
                    {t('uploadLocalImage', { defaultValue: '本地上传图片' })}
                  </button>
                </div>
                
                {/* Preset collections */}
                <div className="space-y-2">
                  <div className="flex items-center space-x-2 overflow-x-auto py-1">
                    {PRESET_AVATARS.map((url, idx) => (
                      <button 
                        key={idx}
                        type="button"
                        onClick={() => setEditAvatar(url)}
                        className={`w-10 h-10 rounded-lg overflow-hidden border-2 transition-all ${editAvatar === url ? 'border-[var(--color-kb-accent)] scale-105 shadow' : 'border-transparent hover:scale-105'}`}
                      >
                        <img src={url} alt="preset" className="w-full h-full object-cover" />
                      </button>
                    ))}
                  </div>
                  
                  <div className="flex items-center space-x-1.5 overflow-x-auto py-1 border-t border-zinc-100 dark:border-zinc-800/50 pt-1.5">
                    <span className="text-[10px] text-zinc-400 dark:text-zinc-500 shrink-0 mr-1">{t('emojiAvatar', { defaultValue: '表情头像:' })}</span>
                    {EMOTE_PRESETS.map((emote, idx) => (
                      <button 
                        key={idx}
                        type="button"
                        onClick={() => setEditAvatar(emote)}
                        className={`w-7 h-7 flex items-center justify-center rounded-md text-base border transition-all ${editAvatar === emote ? 'bg-[var(--color-kb-accent)]/10 border-[var(--color-kb-accent)] scale-105' : 'bg-transparent border-zinc-100 dark:border-zinc-800 hover:bg-zinc-100 dark:hover:bg-zinc-800'}`}
                      >
                        {emote}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              {/* Status and Custom Status Note */}
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('currentStatus', { defaultValue: '当前状态' })}</label>
                  <select 
                    value={editStatus}
                    onChange={(e) => setEditStatus(e.target.value as any)}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  >
                    <option value="online">{t('selectOnline', { defaultValue: '🟢 在线 (Online)' })}</option>
                    <option value="busy">{t('selectBusy', { defaultValue: '🔴 忙碌 (Busy)' })}</option>
                    <option value="away">{t('selectAway', { defaultValue: '🟡 离开 (Away)' })}</option>
                    <option value="offline">{t('selectOffline', { defaultValue: '⚫ 离线 (Offline)' })}</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('currentMotto', { defaultValue: '当前手记/签名' })}</label>
                  <input 
                    type="text" 
                    value={editStatusText}
                    onChange={(e) => setEditStatusText(e.target.value)}
                    placeholder={t('statusTextPlaceholder', { defaultValue: '状态文字（如：开会中）' })}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  />
                </div>
              </div>

              {/* Quick Preset status chips */}
              <div className="space-y-1.5">
                <label className="block text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-widest">{t('quickStatusPreset', { defaultValue: '快速设定状态签到' })}</label>
                <div className="flex flex-wrap gap-1.5">
                  {QUICK_STATUS_PRESETS.map((preset, idx) => (
                    <button
                      key={idx}
                      type="button"
                      onClick={() => {
                        setEditStatus(preset.status);
                        setEditStatusText(preset.text);
                      }}
                      className="text-[10px] px-2.5 py-1 bg-zinc-100 hover:bg-[var(--color-kb-accent)]/10 dark:bg-zinc-800/80 dark:hover:bg-zinc-800 rounded-lg text-zinc-600 dark:text-zinc-300 hover:text-[var(--color-kb-accent)] transition-all border border-transparent hover:border-[var(--color-kb-accent)]/20 active:scale-95"
                    >
                      {preset.text}
                    </button>
                  ))}
                </div>
              </div>

              {/* Basic credentials */}
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('nameNickname', { defaultValue: '姓名 / 昵称' })} <span className="text-rose-500">*</span></label>
                  <input 
                    type="text" 
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('timezonePreference', { defaultValue: '时区偏好' })}</label>
                  <select 
                    value={editTimezone}
                    onChange={(e) => setEditTimezone(e.target.value)}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  >
                    <option value="GMT+8 (北京时间)">{t('beijingTime', { defaultValue: 'GMT+8 (北京时间)' })}</option>
                    <option value="GMT-8 (太平洋时间)">{t('pacificTime', { defaultValue: 'GMT-8 (太平洋时间)' })}</option>
                    <option value="GMT+0 (世界协调时)">{t('utcTime', { defaultValue: 'GMT+0 (世界协调时)' })}</option>
                  </select>
                </div>
              </div>

              {/* Job title and Department */}
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('jobTitle', { defaultValue: '主管职称 / 职位' })}</label>
                  <input 
                    type="text" 
                    value={editTitle}
                    onChange={(e) => setEditTitle(e.target.value)}
                    placeholder={t('jobTitlePlaceholder', { defaultValue: '例如：高级产品经理' })}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('departmentAttr', { defaultValue: '工作部门 / 属性' })}</label>
                  <input 
                    type="text" 
                    value={editDepartment}
                    onChange={(e) => setEditDepartment(e.target.value)}
                    placeholder={t('deptPlaceholder', { defaultValue: '业务、研发或自媒体部' })}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  />
                </div>
              </div>

              {/* Contact info */}
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('contactEmail', { defaultValue: '联系邮箱' })}</label>
                  <input 
                    type="email" 
                    value={editEmail}
                    onChange={(e) => setEditEmail(e.target.value)}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  />
                </div>
                <div>
                  <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('contactPhone', { defaultValue: '联系电话' })}</label>
                  <input 
                    type="text" 
                    value={editPhone}
                    onChange={(e) => setEditPhone(e.target.value)}
                    className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none focus:border-[var(--color-kb-accent)]"
                  />
                </div>
              </div>

              {/* Personal Biography */}
              <div>
                <label className="block text-xs font-bold text-zinc-700 dark:text-zinc-300 mb-1.5">{t('personalBio', { defaultValue: '个人签名 / 简历 / 简介' })}</label>
                <textarea 
                  value={editBio}
                  onChange={(e) => setEditBio(e.target.value)}
                  rows={2}
                  className="w-full bg-zinc-50/50 dark:bg-zinc-950/40 border border-zinc-200 dark:border-zinc-800 rounded-xl px-3 py-2 text-xs text-zinc-800 dark:text-zinc-200 focus:outline-none resize-none focus:border-[var(--color-kb-accent)]"
                  placeholder={t('bioPlaceholder', { defaultValue: '介绍您的背景、兴趣或者专长...' })}
                />
              </div>

            </div>
          ) : (
            /* Elegant Visualized Detailed Profile Card with Premium Tabs */
            <div className="space-y-4 animate-in fade-in duration-200">
              
              {/* Tab Selector */}
              <div className="flex border-b border-zinc-100 dark:border-zinc-800/80 pb-0.5 shrink-0">
                <button
                  type="button"
                  onClick={() => setActiveTab('profile')}
                  className={`pb-2 px-4 text-xs font-semibold border-b-2 transition-all ${
                    activeTab === 'profile'
                      ? 'border-[var(--color-kb-accent)] text-[var(--color-kb-accent)]'
                      : 'border-transparent text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-400'
                  }`}
                >
                  {t('profileTab', { defaultValue: '个人档案' })}
                </button>
                <button
                  type="button"
                  onClick={() => setActiveTab('statistics')}
                  className={`pb-2 px-4 text-xs font-semibold border-b-2 transition-all ${
                    activeTab === 'statistics'
                      ? 'border-[var(--color-kb-accent)] text-[var(--color-kb-accent)]'
                      : 'border-transparent text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-400'
                  }`}
                >
                  {t('statsTab', { defaultValue: '创作成就与统计' })}
                </button>
              </div>

              {activeTab === 'profile' ? (
                <div className="space-y-4 animate-in fade-in duration-200">
                  {/* Bio block */}
                  {profile.bio && (
                    <div className="p-4 bg-zinc-50/50 dark:bg-zinc-950/20 border border-zinc-100 dark:border-zinc-800/80 rounded-2xl relative overflow-hidden">
                      <div className="absolute top-2 right-2 text-zinc-200 dark:text-zinc-800 pointer-events-none">
                        <Smile size={32} />
                      </div>
                      <h4 className="text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-widest mb-1.5">{t('personalMottoQuote', { defaultValue: '个人格言 & 签名' })}</h4>
                      <p className="text-xs text-zinc-600 dark:text-zinc-300 leading-relaxed font-medium">
                        {profile.bio}
                      </p>
                    </div>
                  )}

                  {/* Work Profile Details Panel */}
                  <div className="bg-white dark:bg-zinc-900 border border-zinc-100 dark:border-zinc-800/60 rounded-2xl p-4 space-y-3.5 shadow-sm">
                    <h4 className="text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-widest mb-1 pb-1 border-b border-zinc-100 dark:border-zinc-800">
                      {t('workProfileInfo', { defaultValue: '职业与岗位信息' })}
                    </h4>

                    <div className="grid grid-cols-2 gap-4">
                      <div className="flex items-center space-x-3.5">
                        <span className="p-2 bg-[var(--color-kb-accent)]/10 text-[var(--color-kb-accent)] rounded-xl shrink-0">
                          <Briefcase size={15} />
                        </span>
                        <div>
                          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 block">{t('positionLabel', { defaultValue: '所在职位' })}</span>
                          <span className="text-xs font-semibold text-zinc-800 dark:text-zinc-200">{profile.title || t('notFilled', { defaultValue: '暂未填写' })}</span>
                        </div>
                      </div>

                      <div className="flex items-center space-x-3.5">
                        <span className="p-2 bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 rounded-xl shrink-0">
                          <Info size={15} />
                        </span>
                        <div>
                          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 block">{t('workDeptLabel', { defaultValue: '工作部门' })}</span>
                          <span className="text-xs font-semibold text-zinc-800 dark:text-zinc-200">{profile.department || t('defaultDept', { defaultValue: '企业高管 / 合伙人' })}</span>
                        </div>
                      </div>

                      <div className="flex items-center space-x-3.5">
                        <span className="p-2 bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 rounded-xl shrink-0">
                          <Clock size={15} />
                        </span>
                        <div>
                          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 block">{t('timezonePreference', { defaultValue: '时区偏好' })}</span>
                          <span className="text-xs font-semibold text-zinc-800 dark:text-zinc-200">{profile.timezone}</span>
                        </div>
                      </div>

                      <div className="flex items-center space-x-3.5">
                        <span className="p-2 bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 rounded-xl shrink-0">
                          <Sparkles size={15} />
                        </span>
                        <div>
                          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 block">{t('memberRoleLabel', { defaultValue: '成员角色' })}</span>
                          <span className="text-xs font-semibold text-zinc-800 dark:text-zinc-200">{t('roleOwner', { defaultValue: '系统拥有者 (Owner)' })}</span>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Contact Information Details */}
                  <div className="bg-white dark:bg-zinc-900 border border-zinc-100 dark:border-zinc-800/60 rounded-2xl p-4 space-y-3.5 shadow-sm">
                    <h4 className="text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-widest mb-1 pb-1 border-b border-zinc-100 dark:border-zinc-800">
                      {t('contactCoopInfo', { defaultValue: '联络及协同信息' })}
                    </h4>

                    <div className="space-y-2.5">
                      <div className="flex items-center space-x-3.5">
                        <span className="p-2 bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 rounded-xl shrink-0">
                          <Mail size={15} />
                        </span>
                        <div className="overflow-hidden">
                          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 block">{t('workEmailLabel', { defaultValue: '工作邮箱' })}</span>
                          <span className="text-xs font-semibold text-zinc-800 dark:text-zinc-200 truncate block">{profile.email || t('noEmailBound', { defaultValue: '未绑定工作邮件' })}</span>
                        </div>
                      </div>

                      <div className="flex items-center space-x-3.5">
                        <span className="p-2 bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-300 rounded-xl shrink-0">
                          <Phone size={15} />
                        </span>
                        <div>
                          <span className="text-[10px] text-zinc-400 dark:text-zinc-500 block">{t('contactPhone', { defaultValue: '联系电话' })}</span>
                          <span className="text-xs font-semibold text-zinc-800 dark:text-zinc-200">{profile.phone || t('noPhoneLinked', { defaultValue: '未关联移动电话' })}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="space-y-4 animate-in fade-in duration-200">
                  {/* KPI Row */}
                  <div className="grid grid-cols-2 gap-4">
                    <div className="p-4 bg-zinc-50 dark:bg-zinc-950/20 border border-zinc-100 dark:border-zinc-800/80 rounded-2xl flex flex-col justify-between h-[100px] shadow-sm hover:border-[var(--color-kb-accent)]/30 transition-all">
                      <span className="text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-wider block">{t('kpiDocs', { defaultValue: '✍️ 撰写手记与文档' })}</span>
                      <div className="flex items-baseline space-x-1">
                        <span className="text-3xl font-extrabold text-zinc-800 dark:text-zinc-100">42</span>
                        <span className="text-[10px] text-zinc-400 dark:text-zinc-500">{t('unitPieces', { defaultValue: '篇' })}</span>
                      </div>
                    </div>
                    <div className="p-4 bg-zinc-50 dark:bg-zinc-950/20 border border-zinc-100 dark:border-zinc-800/80 rounded-2xl flex flex-col justify-between h-[100px] shadow-sm hover:border-[var(--color-kb-accent)]/30 transition-all">
                      <span className="text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-wider block">{t('kpiSpaces', { defaultValue: '📁 管理协作知识学堂' })}</span>
                      <div className="flex items-baseline space-x-1">
                        <span className="text-3xl font-extrabold text-zinc-800 dark:text-zinc-100">3</span>
                        <span className="text-[10px] text-zinc-400">{t('unitItems', { defaultValue: '个' })}</span>
                      </div>
                    </div>
                  </div>
                  
                  {/* Achievement stats */}
                  <div className="bg-white dark:bg-zinc-900 border border-zinc-100 dark:border-zinc-800/60 rounded-2xl p-4 space-y-4 shadow-sm">
                    <h4 className="text-[11px] font-bold text-zinc-400 dark:text-zinc-500 uppercase tracking-widest pb-1 border-b border-zinc-100 dark:border-zinc-800">
                      {t('coopAchievements', { defaultValue: '知识协同成就' })}
                    </h4>
                    
                    <div className="space-y-3.5">
                      <div>
                        <div className="flex items-center justify-between text-xs mb-1.5">
                          <span className="text-zinc-500 dark:text-zinc-400">{t('activeDaysSlider', { defaultValue: '本月活跃创作天数' })}</span>
                          <span className="font-semibold text-zinc-700 dark:text-zinc-200">18 / 30 {t('unitDays', { defaultValue: '天' })}</span>
                        </div>
                        <div className="w-full h-2 bg-zinc-100 dark:bg-zinc-800 rounded-full overflow-hidden">
                          <div className="h-full bg-[var(--color-kb-accent)]" style={{ width: '60%' }}></div>
                        </div>
                      </div>

                      <div>
                        <div className="flex items-center justify-between text-xs mb-1.5">
                          <span className="text-zinc-500 dark:text-zinc-400">{t('upvotesLikes', { defaultValue: '文档全网收藏/点赞' })}</span>
                          <span className="font-semibold text-zinc-700 dark:text-zinc-200">158 {t('unitTimes', { defaultValue: '次' })}</span>
                        </div>
                        <div className="w-full h-2 bg-zinc-100 dark:bg-zinc-800 rounded-full overflow-hidden">
                          <div className="h-full bg-emerald-500" style={{ width: '82%' }}></div>
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className="p-3.5 bg-[var(--color-kb-accent)]/5 border border-[var(--color-kb-accent)]/10 rounded-xl flex items-start space-x-2.5">
                    <Sparkles size={16} className="text-[var(--color-kb-accent)] shrink-0 mt-0.5" />
                    <div>
                      <h5 className="text-[11px] font-bold text-zinc-700 dark:text-zinc-200">{t('badgeTitle', { defaultValue: '✨ 智能排版助手推荐奖励' })}</h5>
                      <p className="text-[10px] text-zinc-500 dark:text-zinc-400 mt-1 leading-relaxed">{t('badgeDesc', { defaultValue: '由于您积极参与知识库沉淀和高质量排版，系统已解锁“初级知识架构师”勋章奖励。' })}</p>
                    </div>
                  </div>
                </div>
              )}

            </div>
          )}

        </div>

        {/* Modal Footer Actions */}
        <div className="px-6 py-4 border-t border-zinc-100 dark:border-zinc-800 bg-zinc-50/50 dark:bg-zinc-950/20 flex justify-between items-center flex-shrink-0">
          <div className="flex items-center space-x-1.5 text-[11px] text-zinc-400 dark:text-zinc-500">
            <AlertCircle size={13} />
            <span>{t('revisionSyncNote', { defaultValue: '资料更新后系统中的修订作者记录同步更新' })}</span>
          </div>

          <div className="flex items-center space-x-2">
            {isEditing ? (
              <>
                <button 
                  type="button" 
                  onClick={() => setIsEditing(false)}
                  className="px-4 py-2 text-xs font-semibold text-zinc-600 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800/80 rounded-xl transition-all active:scale-95"
                >
                  {t('cancelBtn', { defaultValue: '取消' })}
                </button>
                <button 
                  type="button" 
                  onClick={handleSave}
                  className="px-4.5 py-2 text-xs font-semibold bg-[var(--color-kb-accent)] hover:bg-[var(--color-kb-accent-hover)] text-white rounded-xl shadow-md active:scale-95 transition-all flex items-center space-x-1.5"
                >
                  <Check size={14} />
                  <span>{t('saveConfig', { defaultValue: '保存配置' })}</span>
                </button>
              </>
            ) : (
              <>
                <button 
                  type="button" 
                  onClick={onClose}
                  className="px-4 py-2 text-xs font-semibold text-zinc-600 hover:text-zinc-800 dark:text-zinc-400 dark:hover:text-zinc-200 hover:bg-zinc-100 dark:hover:bg-zinc-800/80 rounded-xl transition-all active:scale-95"
                >
                  {t('close', { defaultValue: '关闭' })}
                </button>
                <button 
                  type="button" 
                  onClick={() => setIsEditing(true)}
                  className="px-4.5 py-2 text-xs font-semibold bg-[var(--color-kb-accent)]/10 hover:bg-[var(--color-kb-accent)]/20 text-[var(--color-kb-accent)] rounded-xl transition-all active:scale-95 border border-[var(--color-kb-accent)]/30 flex items-center space-x-1.5"
                >
                  <Edit2 size={13} />
                  <span>{t('editProfile', { defaultValue: '编辑资料' })}</span>
                </button>
              </>
            )}
          </div>
        </div>

      </div>
    </div>
  );
}
