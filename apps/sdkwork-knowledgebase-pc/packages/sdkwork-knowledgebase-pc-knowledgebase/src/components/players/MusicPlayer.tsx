import React, { useState, useRef, useEffect, useMemo } from 'react';
import { 
  Music, Play, Pause, Sparkles, Download, 
  Volume2, VolumeX, Sliders, Repeat, 
  SkipForward, SkipBack, Heart, ListMusic, Radio, 
  Headphones, FileText, AlignLeft
} from 'lucide-react';
import { isKnowledgebaseApiAvailable, shouldUseKnowledgebaseDemoFallback } from 'sdkwork-knowledgebase-pc-core';
import { DocumentMeta } from '../../services/document';
import { AIService } from '../../services/ai';

function resolveTrackAudioUrl(url: string | undefined): string {
  if (url) {
    return url;
  }
  return shouldUseKnowledgebaseDemoFallback()
    ? 'https://cdn.pixabay.com/download/audio/2022/10/25/audio_220e8b15d9.mp3?filename=piano-moment-9835.mp3'
    : '';
}

const NEUTRAL_TRACK_COVER = `data:image/svg+xml,${encodeURIComponent(
  '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="400"><rect width="400" height="400" fill="#262626"/><text x="200" y="220" text-anchor="middle" fill="#a3a3a3" font-family="sans-serif" font-size="64">♪</text></svg>',
)}`;

function resolveTrackCoverUrl(mockCoverUrl: string): string {
  return shouldUseKnowledgebaseDemoFallback() ? mockCoverUrl : NEUTRAL_TRACK_COVER;
}

export interface MusicTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  genre: string;
  durationStr: string;
  durationSecs: number;
  url: string;
  coverUrl: string;
  lyrics: { time: number; text: string; translation?: string }[];
}

export const DEMO_PLAYLIST: MusicTrack[] = [
  {
    id: 'track-1',
    title: 'Golden Hour Story',
    artist: 'Lofi Ambient Collective',
    album: 'Sunset Library Vol. 1',
    genre: 'Lofi Chill / Acoustic',
    durationStr: '04:15',
    durationSecs: 255,
    url: 'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3',
    coverUrl: 'https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=300&auto=format&fit=crop&q=60',
    lyrics: [
      { time: 0, text: '🎵 (Instrumental Intro - Soft Acoustic Guitar)', translation: '🎵 (吉他前奏 - 轻柔舒缓的木吉他弦音)' },
      { time: 10, text: 'Sunlight slips through the golden window panes...', translation: '阳光透过金黄色的窗棂洒落在地板上...' },
      { time: 22, text: 'Chasing the shadows, washing away our pains.', translation: '驱散尘世的阴霾，洗尽所有疲惫与悲伤。' },
      { time: 35, text: 'Every note holds a memory we used to share...', translation: '谱写的每一个音符，都承载着你我共度的时光...' },
      { time: 48, text: 'Warm breeze rising, floating in the quiet air.', translation: '和煦微风拂面吹过，在静谧的空气中悄然流淌。' },
      { time: 61, text: '🎵 (Beat Drop - Soft kick drums joining in)', translation: '🎵 (鼓点渐入 - 慢摇节奏缓缓融合)' },
      { time: 75, text: 'Hold on to the golden hour, before it fades away.', translation: '紧紧留住这曼妙的金色时刻，在它在夜色中逝去之前。' },
      { time: 88, text: 'All the secret lines and words we wanted to say...', translation: '那些深藏在心底、未曾言说的秘密情话...' },
      { time: 102, text: 'We write them in the melody, simple and true.', translation: '我们把它们谱写成歌，朴素而纯真地告白。' },
      { time: 115, text: 'Floating like clouds, and reflecting the morning dew.', translation: '犹如浮云游动，又似清晨莹润的露珠闪烁。' },
      { time: 130, text: '🎵 (Melodic Bridge - Electric piano keys solo)', translation: '🎵 (情感过渡 - 柔美电子琴Solo段落)' },
      { time: 155, text: 'Underneath the vast and starry endless sky...', translation: '在浩瀚无垠、星光灿烂的穹顶之下...' },
      { time: 172, text: 'We count the beats of life, letting arguments pass by.', translation: '倾听生命的心跳，将琐碎的争执消融在微风里。' },
      { time: 190, text: 'Golden stories written on the dusty library shelves.', translation: '落满尘埃的图书馆书架上，写满了金色的故事。' },
      { time: 208, text: 'Finding who we are, rediscovering our true selves.', translation: '寻找本真的自我，重新拥抱真实的内心。' },
      { time: 225, text: '🎵 (Outro - Echoing strings fading)', translation: '🎵 (尾奏 - 弦乐渐弱回荡)' },
      { time: 245, text: 'Just stay with me in the sunset...', translation: '只愿与你，共赏这落日余晖...' }
    ]
  },
  {
    id: 'track-2',
    title: 'Neon Odyssey',
    artist: 'Retro Horizon',
    album: 'Cyber Grid 1988',
    genre: 'Synthwave / Retro',
    durationStr: '05:02',
    durationSecs: 302,
    url: 'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-2.mp3',
    coverUrl: 'https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=300&auto=format&fit=crop&q=60',
    lyrics: [
      { time: 0, text: '⚡ (Electronic Synth Prelude - Fast Arpeggios)', translation: '⚡ (电子合成器前奏 - 快速琶音律动)' },
      { time: 15, text: 'Driving through the neon-lit concrete canyon...', translation: '驾车穿梭在霓虹闪烁的钢铁森林深处...' },
      { time: 30, text: 'Digital rain falls on a lonely companion.', translation: '赛博世界的雨滴，拍打着孤独同伴的车窗。' },
      { time: 45, text: 'Flashes of purple, magenta and cyan beams...', translation: '粉紫与冰蓝的激光束交变闪烁...' },
      { time: 60, text: 'We are just lines of codes inside these dreaming machines.', translation: '在冰冷的机芯梦境中，我们不过是跳跃的代码。' },
      { time: 75, text: '⚡ (Power Chorus - High resonance synthesizer lead)', translation: '⚡ (副歌爆发 - 高共鸣合成器主音领奏)' },
      { time: 90, text: 'Run into the grid, run away from the past!', translation: '全速奔向网格，逃离往昔岁月的追捕！' },
      { time: 105, text: 'In this cyberpunk highway, nothing is built to last.', translation: '在这条高科技的高速路上，没有永恒的存在。' },
      { time: 120, text: 'Speed of light, keeping the engines on fire!', translation: '以光速驰骋，让引擎彻底燃烧！' },
      { time: 135, text: 'Chasing the digital hope of our heart\'s desire.', translation: '追寻虚构网格中，那绝不妥协的黎明之光。' },
      { time: 150, text: '⚡ (Guitar Synth Solo and Laser FX)', translation: '⚡ (合成吉他独奏与激光声效)' }
    ]
  },
  {
    id: 'track-3',
    title: 'Midnight Espresso',
    artist: 'Cafe Jazz Trio',
    album: 'Rainy Day Brews',
    genre: 'Acoustic Jazz / Piano',
    durationStr: '03:44',
    durationSecs: 224,
    url: 'https://www.soundhelix.com/examples/mp3/SoundHelix-Song-4.mp3',
    coverUrl: 'https://images.unsplash.com/photo-1511192336575-5a79af67a629?w=300&auto=format&fit=crop&q=60',
    lyrics: [
      { time: 0, text: '☕ (Calm Piano chord progression & Rain backdrop)', translation: '☕ (宁静的爵士钢琴和弦伴奏与窗外细雨声)' },
      { time: 12, text: 'Sipping hot espresso, clock ticking midnight chime...', translation: '细品温热的浓缩咖啡，午夜钟声正缓缓敲响...' },
      { time: 26, text: 'Letting other busy souls fight with their ticking time.', translation: '任由那些喧嚣的灵魂去跟时间搏斗吧，与我无关。' },
      { time: 40, text: 'Here in this dim room, with golden vintage brass...', translation: '在此间昏暗小室，闪耀着复古黄铜的微光。' },
      { time: 54, text: 'Watching misty rain slide down the window glass.', translation: '凝望窗外的珠雨慢慢滑落打湿玻璃。' },
      { time: 68, text: '☕ (Saxophone enter - Smooth Jazz Melancholy)', translation: '☕ (萨克斯声渐入 - 经典舒缓爵士忧郁感)' }
    ]
  }
];

export interface MusicPlayerProps {
  activeDoc: DocumentMeta;
  onToastMessage?: (msg: string) => void;
  isTranscribing?: boolean;
  onTranscribeStart?: () => void;
  onTranscribeComplete?: (content: string) => void;
  aiEnabled?: boolean;
}

export function MusicPlayer({
  activeDoc,
  onToastMessage,
  isTranscribing,
  onTranscribeStart,
  onTranscribeComplete,
  aiEnabled = true,
}: MusicPlayerProps) {
  const demoPlaylist = shouldUseKnowledgebaseDemoFallback() ? DEMO_PLAYLIST : [];
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [volume, setVolume] = useState(0.8);
  const [isMuted, setIsMuted] = useState(false);

  const [activeDemoTrackId, setActiveDemoTrackId] = useState<string>('doc');
  const [repeatMode, setRepeatMode] = useState<'all' | 'one' | 'none'>('all');
  const [eqPreset, setEqPreset] = useState<'standard' | 'pop' | 'jazz' | 'rock' | 'vocal' | 'classical'>('standard');
  const [activeMusicTab, setActiveMusicTab] = useState<'lyrics' | 'playlist' | 'eq'>('lyrics');
  const [favoriteSongs, setFavoriteSongs] = useState<string[]>([]);
  const [visualizerHeights, setVisualizerHeights] = useState<number[]>(
    Array.from({ length: 28 }, () => Math.floor(Math.random() * 60) + 15)
  );

  const triggerToast = (msg: string) => {
    if (onToastMessage) {
      onToastMessage(msg);
    }
  };

  // Execute transcription
  useEffect(() => {
    if (aiEnabled && isTranscribing && onTranscribeComplete) {
      if (isKnowledgebaseApiAvailable() && !activeDoc.url) {
        triggerToast('音频文件缺少下载地址，无法转写。');
        onTranscribeComplete('');
        return;
      }
      triggerToast('AI 正在语音转文字，请稍候...');
      AIService.speechToText(activeDoc.url || '', {
        spaceId: activeDoc.kbId,
        documentId: activeDoc.id,
      }).then(text => {
        onTranscribeComplete(text);
        triggerToast('语音转写已经成功完成！');
      }).catch(err => {
        console.error(err);
        triggerToast(err instanceof Error ? err.message : '语音转写失败。');
        onTranscribeComplete('');
      });
    }
  }, [activeDoc, aiEnabled, isTranscribing, onTranscribeComplete]);

  // Sync volume state with HTMLAudioElement
  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.volume = isMuted ? 0 : volume;
      audioRef.current.muted = isMuted;
    }
  }, [volume, isMuted]);

  // Jitter the visualizer bars when audio is playing
  useEffect(() => {
    let intervalId: any;
    if (isPlaying) {
      intervalId = setInterval(() => {
        setVisualizerHeights(
          Array.from({ length: 28 }, () => Math.floor(Math.random() * 75) + 12)
        );
      }, 120);
    } else {
      setVisualizerHeights([
        15, 20, 30, 42, 55, 60, 45, 30, 22, 18, 24, 38, 
        52, 65, 58, 40, 25, 15, 22, 35, 48, 55, 42, 28, 
        20, 15, 12, 10
      ]);
    }
    return () => {
      if (intervalId) clearInterval(intervalId);
    };
  }, [isPlaying]);

const AUDIO_TRANSCRIPT = [
  { time: 0, text: '上午好，各位！感谢大家参加本周的季度产品研讨会。', translation: '' },
  { time: 5, text: '今天我们主要讨论下个季度的产品研发路线图，', translation: '' },
  { time: 8, text: '特别是关于我们知识库的新功能：包括AI摘要、录音转写和音视频播放。', translation: '' },
  { time: 14, text: '（翻阅资料的沙沙声）', translation: '' },
  { time: 16, text: '我们要确保用户上传的音频和视频不仅能播放，还能自动生成检索标签。', translation: '' },
  { time: 22, text: '张总，您对这个功能的预期进度是怎么看的？', translation: '' },
  { time: 26, text: '我觉得可以在四周内完成核心的播放器设计与转写接口联调，', translation: '' },
  { time: 31, text: '然后在第五周进行内部灰度测试。', translation: '' },
  { time: 34, text: '同意。那么开发团队这边评估一下联调的难点。', translation: '' },
  { time: 38, text: '主要难点在于前端需要定制播放器UI，不能完全使用浏览器默认的控制条，', translation: '' },
  { time: 43, text: '特别是我们需要歌词与转写内容的高亮跟随功能，这需要精准的timeUpdate事件处理。', translation: '' },
  { time: 51, text: '好的，这点请务必在方案设计里详细说明。我们接下来看第二项议题...', translation: '' }
];

  const activeTrack = useMemo<MusicTrack>(() => {
    if (activeDemoTrackId === 'doc') {
      if (activeDoc.type === 'audio') {
        return {
          id: 'doc',
          title: activeDoc.title,
          artist: activeDoc.author || '参会人员',
          album: '会议录音',
          genre: '语音识别',
          durationStr: activeDoc.size || '32:15',
          durationSecs: 1935,
          url: resolveTrackAudioUrl(activeDoc.url),
          coverUrl: resolveTrackCoverUrl('https://images.unsplash.com/photo-1589903308904-1010c2294adc?w=400&q=80'),
          lyrics: isKnowledgebaseApiAvailable()
            ? [{ time: 0, text: `🎵 正在播放《${activeDoc.title}》`, translation: '' }]
            : AUDIO_TRANSCRIPT
        };
      }

      if (activeDoc.id === 'doc-music-1') {
        return {
          id: 'doc',
          title: 'Retro Cyberpunk Beats',
          artist: 'Neon Rider',
          album: 'Metropolis 2099',
          genre: 'Synthwave / Cyberpunk',
          durationStr: '04:22',
          durationSecs: 262,
          url: resolveTrackAudioUrl(activeDoc.url),
          coverUrl: resolveTrackCoverUrl('https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?w=400&q=80'),
          lyrics: [
            { time: 0, text: '⚡ [System Initialization] - Loading "Retro Cyberpunk Beats" by Neon Rider', translation: '⚡ [霓虹系统准备完毕] - 正在载入推荐曲目 《Retro Cyberpunk Beats》' },
            { time: 10, text: 'Cruising through neon-drenched rainy streets...', translation: '驾驶爱车全速穿越霓虹闪烁的雨夜街道...' },
            { time: 24, text: 'The synthesizer bassline begins to pulse beneath your feet.', translation: '超低频合成器贝斯开始在脚底有节奏地律动跃鸣。' },
            { time: 38, text: 'We are just lines of codes inside these dreaming machines.', translation: '在这寂静的钢铁机芯梦境中，你我皆是优雅滑行的代码。' },
            { time: 52, text: 'Flashes of purple, magenta and cyan beams lighting our path.', translation: '粉紫与冰蓝的红外激光束交织闪烁，照亮未来的轨迹。' },
            { time: 70, text: '🎵 (Aesthetic Mechanical Solo - Cybernetic Equalizer Active)', translation: '🎵 (声学艺术独奏 - 3D赛博音效均衡内核生效中)' },
            { time: 92, text: 'Chasing the digital horizon, leaving gravity behind.', translation: '全速前行折闪般掠过地平线，彻底挣脱重力的羁绊。' },
            { time: 115, text: 'Keep holding on directly to the sound stream before it fades.', translation: '请置身于这股高品质动态音频洪流中，沉浸直至余音散去。' }
          ]
        };
      }
      if (activeDoc.id === 'doc-music-2') {
        return {
          id: 'doc',
          title: 'Lofi Coffee Shop Study',
          artist: 'Chilled Cat',
          album: 'Rainy Afternoons',
          genre: 'Lofi Chillout / Jazzhop',
          durationStr: '03:12',
          durationSecs: 192,
          url: resolveTrackAudioUrl(activeDoc.url),
          coverUrl: resolveTrackCoverUrl('https://images.unsplash.com/photo-1514525253161-7a46d19cd819?w=400&q=80'),
          lyrics: [
            { time: 0, text: '☕ Raindrops gently patter against the cozy window glass...', translation: '☕ 淅淅沥沥的雨滴轻敲温暖舒适的咖啡馆橱窗...' },
            { time: 10, text: 'Warm coffee steam rises, mixing with lazy jazzy piano chords.', translation: '温热的一杯美式咖啡蒸汽氤氲，融入懒洋洋的爵士钢琴和弦。' },
            { time: 24, text: 'A safe, tranquil space to read books, write scripts, and write clean code.', translation: '这里是您最安心的港湾，静静品读周刊、撰写推文、敲击清爽代码。' },
            { time: 38, text: 'No rush, no stress, let the acoustic tape hiss wash over you.', translation: '拒绝喧嚣与浮躁，让微小的磁带底层咝咝声轻抚您的疲倦。' },
            { time: 55, text: '🎵 (Relaxing Piano Bridge - Vintage Tube Amp active)', translation: '🎵 (复古胆机氛围间奏 - 悠闲低保真声场漫溢)' },
            { time: 78, text: 'Golden thoughts written carefully in your personal knowledge base.', translation: '将那些深夜里灵光一闪的金色火花，悄然镌刻在个人知识库中。' },
            { time: 105, text: 'Finding peace and cozy synergy in this afternoon lofi mix.', translation: '在这首暖煦慵懒的下午茶低保真声波中，找寻内心深处的秩序感。' }
          ]
        };
      }

      return {
        id: 'doc',
        title: activeDoc.title,
        artist: activeDoc.author || '我的资源',
        album: '我的知识库音源',
        genre: '导入音频',
        durationStr: activeDoc.size || '03:45',
        durationSecs: 225,
        url: resolveTrackAudioUrl(activeDoc.url),
        coverUrl: resolveTrackCoverUrl('https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=300&auto=format&fit=crop&q=60'),
        lyrics: isKnowledgebaseApiAvailable()
          ? [{ time: 0, text: `🎵 正在播放《${activeDoc.title}》`, translation: '' }]
          : [
          { time: 0, text: `🎵 开始播放《${activeDoc.title}》`, translation: `🎵 Playing "${activeDoc.title}"` },
          { time: 8, text: 'Deep thinking meets gorgeous acoustic vibrations...', translation: '深度思考与美妙的声学振动相遇...' },
          { time: 18, text: 'This music is played directly from your personal knowledge-base assets.', translation: '此音乐直接从您的个人知识库资产中加载播放。' },
          { time: 30, text: 'Click anywhere on these lyrics timestamps to seek the audio playback!', translation: '点击歌词里的任意时间印记，即可跳转音频进度！' }
        ]
      };
    }

    const found = demoPlaylist.find(t => t.id === activeDemoTrackId);
    return found || {
      id: 'doc',
      title: activeDoc.title,
      artist: activeDoc.author || '我的资源',
      album: '我的知识库音源',
      genre: '导入音频',
      durationStr: activeDoc.size || '03:45',
      durationSecs: 225,
      url: resolveTrackAudioUrl(activeDoc.url),
      coverUrl: resolveTrackCoverUrl('https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=300&auto=format&fit=crop&q=60'),
      lyrics: []
    };
  }, [activeDemoTrackId, activeDoc, demoPlaylist]);

  const audioUrlToPlay = useMemo(() => {
    return activeTrack.url;
  }, [activeTrack]);

  const togglePlay = () => {
    if (audioRef.current) {
      if (isPlaying) {
        audioRef.current.pause();
      } else {
        audioRef.current.play().catch(() => {});
      }
    }
  };

  const handleNextTrack = () => {
    const list = ['doc', ...demoPlaylist.map(t => t.id)];
    const currentIndex = list.indexOf(activeDemoTrackId);
    const nextIndex = (currentIndex + 1) % list.length;
    setActiveDemoTrackId(list[nextIndex]);
    const info = nextIndex === 0 ? activeDoc.title : demoPlaylist[nextIndex - 1].title;
    triggerToast(`🎵 切换至下一曲: ${info}`);
  };

  const handlePrevTrack = () => {
    const list = ['doc', ...demoPlaylist.map(t => t.id)];
    const currentIndex = list.indexOf(activeDemoTrackId);
    const prevIndex = (currentIndex - 1 + list.length) % list.length;
    setActiveDemoTrackId(list[prevIndex]);
    const info = prevIndex === 0 ? activeDoc.title : demoPlaylist[prevIndex - 1].title;
    triggerToast(`🎵 切换至上一曲: ${info}`);
  };

  const toggleFavorite = (id: string) => {
    if (favoriteSongs.includes(id)) {
      setFavoriteSongs(favoriteSongs.filter(item => item !== id));
      triggerToast('💔 已从收藏夹中移出');
    } else {
      setFavoriteSongs([...favoriteSongs, id]);
      triggerToast('❤️ 已加入收藏歌单！');
    }
  };

  const handleAudioEnded = () => {
    if (repeatMode === 'one' && audioRef.current) {
      audioRef.current.currentTime = 0;
      audioRef.current.play().catch(() => {});
    } else if (repeatMode === 'all') {
      handleNextTrack();
    } else {
      setIsPlaying(false);
    }
  };

  const handleTimeUpdate = () => {
    if (audioRef.current) {
      setCurrentTime(audioRef.current.currentTime);
    }
  };

  const handleLoadedMetadata = () => {
    if (audioRef.current) {
      setDuration(audioRef.current.duration || 210);
    }
  };

  const handleSpeedChange = (rate: number) => {
    setPlaybackRate(rate);
    if (audioRef.current) {
      audioRef.current.playbackRate = rate;
    }
  };

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  };

  return (
    <div id={`music-player-${activeDoc.id}`} className="w-full h-full flex flex-col min-h-0 bg-white dark:bg-[#09090b] text-zinc-800 dark:text-zinc-100 overflow-hidden relative">
      {/* Dynamic Soundwave Equalizer Styles */}
      <style>{`
        @keyframes bounce-eq {
          0%, 100% { transform: scaleY(0.15); }
          50% { transform: scaleY(1); }
        }
        .animate-bounce-eq {
          animation: bounce-eq 1.2s ease-in-out infinite;
          transform-origin: bottom;
        }
      `}</style>

      {/* 1. Header Toolbar */}
      <div className="bg-white dark:bg-[var(--color-kb-panel)] px-4 h-[40px] flex items-center justify-between z-20 border-b border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/80 shrink-0 shadow-sm backdrop-blur-md">
        <div className="min-w-0 flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-lg bg-pink-50 dark:bg-zinc-900 border border-pink-100 dark:border-zinc-800 flex items-center justify-center text-pink-500 shrink-0 shadow-sm">
            <Headphones size={13} strokeWidth={2.5} />
          </div>
          <div className="min-w-0 font-sans">
            <h3 className="text-[12px] font-bold text-zinc-900 dark:text-zinc-100 truncate tracking-tight flex items-center gap-2" title={activeDoc.title}>
              <span>{activeDoc.title}</span>
            </h3>
            <p className="text-[10px] font-mono font-medium text-zinc-400 flex items-center gap-1.5 mt-0.5 leading-none uppercase tracking-wide whitespace-nowrap">
              <span>HI-RES AUDIO</span>
              <span className="opacity-40 text-zinc-200 dark:text-zinc-800">/</span>
              <span>1411 KBPS</span>
              <span className="opacity-40 text-zinc-200 dark:text-zinc-800">/</span>
              <span>FLAC</span>
            </p>
          </div>
        </div>

        {/* Status & actions widget */}
        <div className="flex items-center gap-2.5 h-full shrink-0">
          {aiEnabled && activeDoc.type === 'audio' && (
            <button 
              type="button"
              className="px-2.5 h-7 flex items-center gap-1.2 bg-fuchsia-100/80 text-fuchsia-700 hover:bg-fuchsia-200 dark:bg-fuchsia-500/10 dark:text-fuchsia-400 border border-fuchsia-200/50 dark:border-fuchsia-500/20 text-[10.5px] font-bold rounded-lg transition-all active:scale-95 disabled:opacity-50 disabled:grayscale shrink-0 outline-none select-none"
              onClick={() => {
                onTranscribeStart?.();
              }}
              disabled={isTranscribing || !!activeDoc.content}
            >
              <Sparkles size={11} className={isTranscribing ? "animate-spin text-fuchsia-500" : "text-fuchsia-500 dark:text-fuchsia-400"} />
              {activeDoc.content ? '转录完成' : (isTranscribing ? '正在转录...' : '语音转文字')}
            </button>
          )}

          <div className="flex items-center h-7 gap-1.5 bg-[#fafafa] dark:bg-zinc-950 px-2.5 rounded-lg border border-zinc-200/80 dark:border-[var(--color-kb-panel-border)]/50 scale-95 origin-right text-[10px] text-zinc-400 dark:text-zinc-500 font-bold uppercase tracking-widest leading-none">
            {isPlaying ? (
              <span className="flex items-center gap-1 text-emerald-500"><span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>播放中 DIRECT LINK</span>
            ) : (
              <span className="flex items-center gap-1">已暂停 SYSTEM IDLE</span>
            )}
          </div>
        </div>
      </div>

      {/* Embedded HTML5 Audio element */}
      <audio 
        ref={audioRef} 
        src={audioUrlToPlay}
        onTimeUpdate={handleTimeUpdate}
        onLoadedMetadata={handleLoadedMetadata}
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onEnded={handleAudioEnded}
      />

      {/* Flex container wrapping columns */}
      <div className="flex-1 w-full flex flex-col lg:flex-row min-h-0 bg-transparent overflow-hidden">

        {/* COLUMN 1: Flat Compact Controls & Artwork Panel */}
      <div className="w-full lg:w-[360px] shrink-0 p-6 flex flex-col justify-between bg-zinc-50/80 dark:bg-zinc-950/40 relative min-h-0">
        <div className="absolute inset-0 bg-gradient-to-b from-white/10 dark:from-zinc-900/5 via-transparent to-zinc-100/30 dark:to-zinc-950/20 z-0 pointer-events-none" />
        
        <div className="relative z-10 w-full flex flex-col items-center">
          
          {/* Header Metadata */}
          <div className="w-full flex items-center justify-between mb-2">
            <div className="flex items-center gap-1.5">
              <Radio size={12} className="text-zinc-500 dark:text-zinc-550 animate-pulse" />
              <span className="text-[9px] font-mono uppercase tracking-widest text-zinc-600 dark:text-zinc-500 font-bold">Hi-Res Stream</span>
            </div>
            <div className="text-[9px] font-mono text-zinc-500 dark:text-zinc-600 tracking-wider">
              PCM 48KHZ
            </div>
          </div>

          {/* Premium Borderless Artwork */}
          <div className="relative my-3 flex items-center justify-center w-full">
            <div className="relative w-48 h-48 rounded-xl overflow-hidden shadow-xl group select-none transition-transform duration-500 hover:scale-[1.01]">
              <img 
                src={activeTrack.coverUrl} 
                alt="Album Cover" 
                referrerPolicy="no-referrer"
                className="w-full h-full object-cover transition-transform duration-700 group-hover:scale-102" 
              />
              {isPlaying && (
                <div className="absolute bottom-2 right-2 w-6 h-6 rounded-full bg-white/90 dark:bg-zinc-950/95 flex items-center justify-center text-rose-500 shadow-md">
                  <Music size={11} className="animate-pulse" />
                </div>
              )}
            </div>
          </div>

          {/* Album Metadata Block */}
          <div className="w-full text-center space-y-0.5 my-2">
            <div className="flex items-center justify-center gap-1.5">
              <h2 className="text-md font-bold text-zinc-900 dark:text-zinc-100 tracking-wide truncate max-w-[240px]">
                {activeTrack.title}
              </h2>
              <button 
                onClick={() => toggleFavorite(activeTrack.id)}
                className="p-1 text-zinc-500 hover:text-red-500 transition-colors"
                title="收藏"
              >
                <Heart 
                  size={13} 
                  className={favoriteSongs.includes(activeTrack.id) ? 'fill-red-500 text-red-500 scale-110' : ''} 
                />
              </button>
            </div>
            <p className="text-xs text-zinc-400 font-medium">
              {activeTrack.artist}
            </p>
            <p className="inline-block px-1.5 py-0.5 rounded bg-zinc-200 dark:bg-zinc-900 border border-zinc-300 dark:border-zinc-900 text-[8px] font-bold text-zinc-600 dark:text-zinc-500 uppercase tracking-widest leading-none">
              {activeTrack.genre}
            </p>
          </div>

          {/* Flat Acoustic Spectrum Field */}
          <div className="w-full bg-zinc-200/50 dark:bg-zinc-900/10 rounded-lg p-2.5 mt-2 border border-zinc-200 dark:border-zinc-900/20">
            <div className="flex items-center justify-between mb-1 text-[8px] uppercase font-mono tracking-widest text-zinc-500">
              <span className="flex items-center gap-1"><Sliders size={10} className="text-zinc-500 dark:text-zinc-400" /> Spectrum field</span>
              <span className="text-zinc-500 dark:text-zinc-650">{isPlaying ? 'Active' : 'Idle'}</span>
            </div>
            <div className="h-5 flex items-end justify-between gap-[2.5px] select-none">
              {visualizerHeights.map((height, hidx) => (
                <div 
                  key={hidx}
                  style={{ height: `${height}%` }}
                  className={`w-full rounded-sm transition-all duration-300 ${
                    isPlaying 
                      ? 'bg-rose-500/80 hover:bg-rose-500' 
                      : 'bg-zinc-300 dark:bg-zinc-800/40'
                  }`}
                />
              ))}
            </div>
          </div>

        </div>
        
        {/* Playback Progress and Control Rails */}
        <div className="relative z-10 w-full mt-4 space-y-3">
          
          {/* Flat Slate Seek Bar */}
          <div className="space-y-1">
            <div className="relative w-full h-1 bg-zinc-200 dark:bg-zinc-900 rounded-full cursor-pointer hover:h-1.5 transition-all group/progress">
              <input 
                type="range"
                min={0}
                max={duration > 0 ? duration : (activeTrack.durationSecs || 210)}
                value={currentTime}
                onChange={(e) => {
                  const targetTime = parseFloat(e.target.value);
                  if (audioRef.current) {
                    audioRef.current.currentTime = targetTime;
                    setCurrentTime(targetTime);
                  }
                }}
                className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
              />
              <div 
                style={{ width: `${Math.min(100, Math.max(0, (currentTime / (duration > 0 ? duration : (activeTrack.durationSecs || 210))) * 100))}%` }}
                className="absolute top-0 left-0 h-full bg-rose-500 rounded-full"
              />
            </div>
            <div className="flex justify-between items-center text-[9px] font-mono text-zinc-500">
              <span>{formatTime(currentTime)}</span>
              <span>{formatTime(duration > 0 ? duration : (activeTrack.durationSecs || 210))}</span>
            </div>
          </div>

          {/* Master Controller Row */}
          <div className="flex items-center justify-between">
            {/* Loop Controls */}
            <button 
              onClick={() => {
                const modes: ('all' | 'one' | 'none')[] = ['all', 'one', 'none'];
                const next = modes[(modes.indexOf(repeatMode) + 1) % modes.length];
                setRepeatMode(next);
                triggerToast(next === 'all' ? '🔁 循环播放全部' : next === 'one' ? '🔂 单曲循环' : '➡️ 顺序播放');
              }}
              className={`p-1.5 rounded transition-colors ${repeatMode !== 'none' ? 'text-rose-500 bg-rose-100 dark:bg-rose-950/10' : 'text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300'}`}
              title="播放模式"
            >
              <Repeat size={13} className={repeatMode === 'one' ? 'scale-110' : ''} />
            </button>

            {/* Back track */}
            <button 
              onClick={handlePrevTrack}
              className="p-1.5 rounded-full text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 transition-colors"
              title="上一首"
            >
              <SkipBack size={13} />
            </button>

            {/* Premium Matte Circular Play/Pause Button */}
            <button 
              onClick={togglePlay}
              className="p-2.5 bg-zinc-900 dark:bg-zinc-100 text-white dark:text-zinc-950 hover:bg-zinc-800 dark:hover:bg-zinc-200 rounded-full shadow-md active:scale-95 transition-all flex items-center justify-center border border-transparent"
              title={isPlaying ? '暂停' : '播放'}
            >
              {isPlaying ? <Pause size={13} fill="currentColor" /> : <Play size={13} className="ml-0.5" fill="currentColor" />}
            </button>

            {/* Next track */}
            <button 
              onClick={handleNextTrack}
              className="p-1.5 rounded-full text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-200 transition-colors"
              title="下一首"
            >
              <SkipForward size={13} />
            </button>

            {/* Quick Playback speed */}
            <div className="relative group/speed">
              <button className="px-1.5 py-0.5 border border-zinc-300 dark:border-zinc-900 bg-zinc-100 dark:bg-zinc-900/50 rounded font-mono text-[8px] text-zinc-500 hover:text-zinc-800 dark:hover:text-white transition-colors">
                {playbackRate.toFixed(2)}x
              </button>
              <div className="absolute bottom-full right-0 mb-1 py-1 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 rounded shadow-xl hidden group-hover/speed:block z-40 w-14 text-center">
                {[0.5, 1.0, 1.5, 2.0].map((rate) => (
                  <button
                    key={rate}
                    onClick={() => handleSpeedChange(rate)}
                    className={`w-full py-0.5 font-mono text-[9px] block hover:bg-zinc-100 dark:hover:bg-zinc-800 ${playbackRate === rate ? 'text-rose-500 font-bold' : 'text-zinc-600 dark:text-zinc-400'}`}
                  >
                    {rate.toFixed(1)}x
                  </button>
                ))}
              </div>
            </div>

            {/* HOVER VOLUME CONTROLLER - VERTICAL POPUP DESIGN */}
            <div className="flex items-center group/volume relative select-none">
              <button 
                onClick={() => {
                  const nextMute = !isMuted;
                  setIsMuted(nextMute);
                  triggerToast(nextMute ? '🔇 静音' : '🔊 取消静音');
                }}
                className="p-1.5 text-zinc-500 hover:text-rose-500 transition-colors"
                title="音量"
              >
                {isMuted || volume === 0 ? <VolumeX size={13} className="text-rose-500" /> : <Volume2 size={13} className="text-zinc-400 group-hover/volume:text-rose-500 transition-colors" />}
              </button>
              
              {/* Vertical Pop-up slider - avoids layout deformation. Uses padding to bridge hover gap. */}
              <div className="absolute bottom-full right-1/2 translate-x-1/2 pb-2 opacity-0 pointer-events-none group-hover/volume:opacity-100 group-hover/volume:pointer-events-auto transition-all duration-200 z-50">
                <div className="p-3 bg-white dark:bg-zinc-900 border border-zinc-200 dark:border-zinc-800/80 rounded-xl shadow-2xl flex flex-col items-center gap-2 transform origin-bottom scale-95 group-hover/volume:scale-100 transition-transform duration-200">
                  <span className="text-[9px] font-mono font-bold text-zinc-800 dark:text-zinc-300">
                    {Math.round((isMuted ? 0 : volume) * 100)}%
                  </span>
                  <div className="h-20 flex items-center justify-center py-1">
                    <input 
                      type="range"
                      min={0}
                      max={1}
                      step={0.05}
                      value={isMuted ? 0 : volume}
                      onChange={(e) => {
                        const val = parseFloat(e.target.value);
                        setVolume(val);
                        setIsMuted(false);
                      }}
                      style={{ WebkitAppearance: 'slider-vertical' } as React.CSSProperties}
                      className="h-16 w-1 bg-zinc-300 dark:bg-zinc-800 rounded-full accent-rose-500 cursor-pointer focus:outline-none"
                    />
                  </div>
                </div>
              </div>
            </div>

          </div>

        </div>

      </div>

      {/* COLUMN 2: Scroll Lyrics & Interactive Tabs */}
      <div className="flex-1 flex flex-col min-h-0 bg-transparent">
        
        {/* Borderless Tab Header */}
        <div className="h-12 shrink-0 px-6 border-b border-zinc-200 dark:border-zinc-900/40 flex items-center justify-between bg-zinc-50 dark:bg-zinc-950/20 relative z-10">
          <div className="flex gap-1.5 overflow-x-auto no-scrollbar">
            {(activeDoc.type === 'audio' 
              ? [
                  { id: 'lyrics', label: '语音转写 / Transcript', icon: FileText },
                  { id: 'playlist', label: '章节大纲 / Segments', icon: AlignLeft },
                  { id: 'eq', label: '语音清晰化 / Vocal EQ', icon: Sliders }
                ]
              : [
                  { id: 'lyrics', label: '精美歌词 / Lyrics', icon: Music },
                  { id: 'playlist', label: '播放曲目 / Playlist', icon: ListMusic },
                  { id: 'eq', label: '智能均衡 / Studio EQ', icon: Sliders }
                ]
            ).map((tab) => {
               const Icon = tab.icon;
               return (
                 <button
                   key={tab.id}
                   onClick={() => setActiveMusicTab(tab.id as any)}
                   className={`px-3 py-1 rounded-full text-[11px] font-semibold flex items-center gap-1.5 transition-all whitespace-nowrap ${
                     activeMusicTab === tab.id 
                       ? 'bg-zinc-800 dark:bg-zinc-100 text-white dark:text-zinc-900 shadow-md' 
                       : 'text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 hover:bg-zinc-200/50 dark:hover:bg-zinc-900/10'
                   }`}
                 >
                  <Icon size={11} />
                  {tab.label}
                </button>
              )
            })}
          </div>

          {activeTrack.url && (
            <a 
              href={activeTrack.url}
              download={activeTrack.title + '.mp3'}
              className="px-2.5 py-1 bg-zinc-100 dark:bg-zinc-900/30 hover:bg-zinc-200 dark:hover:bg-zinc-900/60 text-zinc-500 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-white border border-zinc-200 dark:border-zinc-900 rounded-full text-[10px] font-semibold flex items-center gap-1 transition-colors shrink-0"
            >
              <Download size={10} />
              <span>Download</span>
            </a>
          )}
        </div>

        {/* Tab Scroller Grid */}
        <div className="flex-1 p-5 overflow-y-auto relative min-h-0 custom-scrollbar">
          
          {/* Tab 1: Lyrics Stream */}
          {activeMusicTab === 'lyrics' && (
            <div className="h-full flex flex-col justify-between space-y-3">
              <p className="text-[9px] text-zinc-500 dark:text-zinc-650 font-mono tracking-wide uppercase">
                {activeDoc.type === 'audio' 
                  ? '💡 CLICK ANY TIMESTAMP TO SEEK AUDIO' 
                  : '💡 CLICK ANY LYRIC TIMESTAMP TO SEEK AUDIO PLACE'}
              </p>
              <div className="space-y-3.5 scroll-smooth min-h-0 flex-1 overflow-y-auto pr-2 relative">
                {activeTrack.lyrics.map((line, idx) => {
                  const nextL = activeTrack.lyrics[idx + 1];
                  const isCurrentActive = currentTime >= line.time && (!nextL || currentTime < nextL.time);
                  
                  return (
                    <div
                      key={idx}
                      onClick={() => {
                        if (audioRef.current) {
                          audioRef.current.currentTime = line.time;
                          setCurrentTime(line.time);
                          if (!isPlaying) {
                            audioRef.current.play()
                              .then(() => setIsPlaying(true))
                              .catch(() => {});
                          }
                        }
                      }}
                      className={`p-2.5 rounded-lg cursor-pointer transition-all duration-300 border ${
                        isCurrentActive
                          ? 'bg-rose-50 dark:bg-rose-950/10 border-rose-200 dark:border-rose-900/20 text-zinc-900 dark:text-zinc-100 shadow-sm'
                          : 'bg-transparent border-transparent text-zinc-500 hover:text-zinc-700 dark:hover:text-zinc-300 hover:bg-zinc-100 dark:hover:bg-zinc-900/5'
                      }`}
                    >
                      <div className="flex items-start gap-2.5">
                        <span className={`text-[8px] font-mono select-none px-1 py-0.5 rounded shrink-0 ${
                          isCurrentActive ? 'bg-rose-100 dark:bg-rose-500/10 text-rose-500 dark:text-rose-400' : 'bg-zinc-100 dark:bg-zinc-900/40 text-rose-400/60'
                        }`}>
                          {formatTime(line.time)}
                        </span>
                        <div className="space-y-0.5 min-w-0">
                          <p className={`text-xs tracking-wide ${isCurrentActive ? 'font-bold text-rose-600 dark:text-rose-400/90' : 'font-medium'}`}>
                            {line.text}
                          </p>
                          {line.translation && (
                            <p className={`text-[10px] ${isCurrentActive ? 'text-zinc-500 dark:text-zinc-400 font-medium' : 'text-zinc-400 dark:text-zinc-600'}`}>
                              {line.translation}
                            </p>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Tab 2: Curated Sounds List / Segments */}
          {activeMusicTab === 'playlist' && (
            <div className="space-y-3">
              {activeDoc.type === 'audio' ? (
                <>
                  <p className="text-[9px] text-zinc-500 font-mono tracking-wider uppercase mb-1">
                    Meeting Segments (4)
                  </p>
                  
                  <div className="space-y-1.5">
                    {[
                      { title: '会议开场与日程确认', time: 0, duration: '05:20' },
                      { title: '回顾上一季度产品数据', time: 320, duration: '12:45' },
                      { title: '讨论知识库新功能架构', time: 1085, duration: '08:30' },
                      { title: '工作分配与会议总结', time: 1595, duration: '05:40' }
                    ].map((seg, sIdx) => {
                      const isCurrentSeg = currentTime >= seg.time && (sIdx === 3 || currentTime < [0, 320, 1085, 1595, 99999][sIdx + 1]);
                      return (
                        <div key={sIdx} 
                          onClick={() => {
                            if (audioRef.current) {
                              audioRef.current.currentTime = seg.time;
                              setCurrentTime(seg.time);
                              if (!isPlaying) {
                                audioRef.current.play().then(() => setIsPlaying(true)).catch(() => {});
                              }
                            }
                          }}
                          className={`p-2.5 rounded-xl border flex justify-between items-center cursor-pointer transition-all ${
                            isCurrentSeg 
                              ? 'bg-rose-50 dark:bg-rose-950/10 border-rose-200 dark:border-rose-500/20 shadow-sm' 
                              : 'bg-zinc-50 dark:bg-zinc-950/20 border-zinc-200 dark:border-zinc-900/40 hover:bg-zinc-100 dark:hover:bg-zinc-900/40'
                          }`}
                        >
                           <div className="min-w-0 flex-1 flex items-center gap-3.5">
                              <div className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${isCurrentSeg ? 'bg-rose-100 dark:bg-rose-500/20 text-rose-500' : 'bg-zinc-100 dark:bg-zinc-900 text-zinc-500 dark:text-zinc-600'}`}>
                                <AlignLeft size={14} />
                              </div>
                              <div className="min-w-0">
                                <span className="text-[7.5px] font-mono text-zinc-500 dark:text-zinc-550 uppercase tracking-widest block mb-0.5">Segment 0{sIdx + 1}</span>
                                <h4 className={`text-xs font-bold truncate ${isCurrentSeg ? 'text-zinc-900 dark:text-zinc-100' : 'text-zinc-600 dark:text-zinc-400'}`}>{seg.title}</h4>
                              </div>
                           </div>
                           <div className="text-[9px] font-mono text-zinc-500 shrink-0">{seg.duration}</div>
                        </div>
                      )
                    })}
                  </div>
                </>
              ) : (
                <>
                  <p className="text-[9px] text-zinc-500 font-mono tracking-wider uppercase mb-1">
                    Knowledge Audio Assets ({1 + demoPlaylist.length} Tracks)
                  </p>
                  
                  <div className="space-y-1.5">
                    {/* Active Document Track Option */}
                    <div 
                      onClick={() => {
                        setActiveDemoTrackId('doc');
                        triggerToast(`🎵 开始载入知识库音频: ${activeDoc.title}`);
                      }}
                      className={`p-2.5 rounded-xl flex items-center justify-between cursor-pointer border transition-all ${
                        activeDemoTrackId === 'doc'
                          ? 'bg-zinc-100 dark:bg-zinc-900/50 border-rose-200 dark:border-rose-500/20 shadow-sm'
                          : 'bg-zinc-50 dark:bg-zinc-950/20 border-transparent hover:bg-zinc-100 dark:hover:bg-zinc-900/20'
                      }`}
                    >
                      <div className="flex items-center gap-3.5 min-w-0">
                        <div className="relative shrink-0">
                          <img 
                            src={resolveTrackCoverUrl('https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=100&auto=format&fit=crop&q=60')} 
                            alt="cover sticker" 
                            className="w-8 h-8 rounded-lg object-cover" 
                          />
                          {activeDemoTrackId === 'doc' && isPlaying && (
                            <div className="absolute inset-0 bg-black/40 rounded-lg flex items-center justify-center">
                              <span className="w-1 h-1 bg-rose-500 rounded-full animate-ping" />
                            </div>
                          )}
                        </div>
                        <div className="min-w-0">
                          <span className="text-[7.5px] font-bold text-rose-500 uppercase tracking-widest block mb-0.5">Knowledge File</span>
                          <h4 className="text-xs font-bold text-zinc-800 dark:text-zinc-200 truncate">{activeDoc.title}</h4>
                          <p className="text-[8.5px] text-zinc-500 truncate">{activeDoc.author || '未标艺术家'}</p>
                        </div>
                      </div>

                      <div className="flex items-center gap-3 shrink-0 font-mono text-[9px]">
                        <span className="text-zinc-500 dark:text-zinc-600 block">{activeDoc.size || '3.5MB'}</span>
                        {activeDemoTrackId === 'doc' && isPlaying ? (
                          <Pause size={11} className="text-rose-500" />
                        ) : (
                          <Play size={11} className="text-zinc-400 dark:text-zinc-550" />
                        )}
                      </div>
                    </div>

                    {/* Playlist Demo Entries */}
                    {demoPlaylist.map((track) => (
                      <div 
                        key={track.id}
                        onClick={() => {
                          setActiveDemoTrackId(track.id);
                          triggerToast(`🎵 开始载入曲目: ${track.title}`);
                        }}
                        className={`p-2.5 rounded-xl flex items-center justify-between cursor-pointer border transition-all ${
                          activeDemoTrackId === track.id
                            ? 'bg-zinc-100 dark:bg-zinc-900/50 border-rose-200 dark:border-rose-500/20 shadow-sm'
                            : 'bg-zinc-50 dark:bg-zinc-950/20 border-transparent hover:bg-zinc-100 dark:hover:bg-zinc-900/20'
                        }`}
                      >
                        <div className="flex items-center gap-3.5 min-w-0">
                          <div className="relative shrink-0">
                            <img 
                              src={track.coverUrl} 
                              alt={track.title} 
                              className="w-8 h-8 rounded-lg object-cover" 
                            />
                            {activeDemoTrackId === track.id && isPlaying && (
                              <div className="absolute inset-0 bg-black/40 rounded-lg flex items-center justify-center">
                                <span className="w-1 h-1 bg-rose-500 rounded-full animate-ping" />
                              </div>
                            )}
                          </div>
                          <div className="min-w-0">
                            <span className="text-[7.5px] font-mono text-zinc-500 dark:text-zinc-550 uppercase tracking-widest block mb-0.5">{track.genre}</span>
                            <h4 className="text-xs font-bold text-zinc-800 dark:text-zinc-200 truncate">{track.title}</h4>
                            <p className="text-[8.5px] text-zinc-500 dark:text-zinc-550 truncate">{track.artist}</p>
                          </div>
                        </div>

                        <div className="flex items-center gap-3 shrink-0 font-mono text-[9px]">
                          <span className="text-zinc-500 dark:text-zinc-650 block">{track.durationStr}</span>
                          {activeDemoTrackId === track.id && isPlaying ? (
                            <Pause size={11} className="text-rose-500" />
                          ) : (
                            <Play size={11} className="text-zinc-400 dark:text-zinc-600 hover:text-zinc-600 dark:hover:text-zinc-400" />
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                </>
              )}
            </div>
          )}

          {/* Tab 3: Curated Bass EQ Presets */}
          {activeMusicTab === 'eq' && (
            <div className="space-y-4">
              <div className="bg-zinc-100/50 dark:bg-zinc-900/10 p-3.5 rounded-xl border border-zinc-200 dark:border-zinc-900/30">
                <h4 className="text-xs font-bold text-rose-600 dark:text-rose-500 uppercase tracking-wider mb-1 flex items-center gap-2">
                  <Sparkles size={11} className="text-amber-500 dark:text-amber-400" /> Studio EQ Preset System
                </h4>
                <p className="text-[10px] text-zinc-500 dark:text-zinc-400 leading-normal">
                  高精度数字限幅算法，优化中频均衡与三维耳机混响声场定位。
                </p>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {[
                  { id: 'standard', name: '标准原音 (Direct Clean)', desc: '平直原音解析' },
                  { id: 'pop', name: '流行高敏 (Acoustic Pop)', desc: '提升人声频段解析度' },
                  { id: 'jazz', name: '醇美爵士 (Smooth Jazz)', desc: '温润中低回响阶层' },
                  { id: 'rock', name: '超重低音 (Mega Bass)', desc: '加强低频乐器频段瞬态' },
                  { id: 'vocal', name: '清晰人声 (Vibe Vocal)', desc: '过滤环境杂讯突出伴奏' },
                  { id: 'classical', name: '音乐厅 (Grand Auditorium)', desc: '模拟穹顶反射宽声场' }
                ].map((preset) => (
                  <div
                    key={preset.id}
                    onClick={() => {
                      setEqPreset(preset.id as any);
                      triggerToast(`🎚️ 音效均衡已切换: ${preset.name}`);
                    }}
                    className={`p-2.5 rounded-xl cursor-pointer border text-left transition-all ${
                      eqPreset === preset.id
                        ? 'bg-rose-100 dark:bg-rose-950/15 border-rose-300 dark:border-rose-500/30 text-zinc-900 dark:text-zinc-100 shadow-sm'
                        : 'bg-zinc-50 dark:bg-zinc-950/20 border-transparent hover:bg-zinc-100 dark:hover:bg-zinc-900/20 text-zinc-600 dark:text-zinc-450'
                    }`}
                  >
                    <h5 className="text-[11px] font-bold mb-0.5">{preset.name}</h5>
                    <p className="text-[9px] text-zinc-550 leading-normal">{preset.desc}</p>
                  </div>
                ))}
              </div>

              {/* Fader Knobs Bands Illustration */}
              <div className="bg-zinc-50 dark:bg-zinc-950/30 p-3.5 rounded-xl border border-zinc-200 dark:border-zinc-900/50 space-y-2">
                <span className="text-[8px] font-mono uppercase tracking-widest text-zinc-500 dark:text-zinc-600 font-bold block">Digital Hertz Lever Gain (dB)</span>
                <div className="h-28 flex items-center justify-around gap-1 pt-1.5">
                  {[
                    { hz: '32Hz', val: eqPreset === 'rock' ? 8 : eqPreset === 'pop' ? 3 : 0 },
                    { hz: '125Hz', val: eqPreset === 'rock' ? 6 : eqPreset === 'classical' ? 2 : 0 },
                    { hz: '500Hz', val: eqPreset === 'vocal' ? 5 : eqPreset === 'jazz' ? 3 : 0 },
                    { hz: '2kHz', val: eqPreset === 'pop' ? 4 : eqPreset === 'rock' ? 2 : 0 },
                    { hz: '8kHz', val: eqPreset === 'classical' ? 5 : eqPreset === 'vocal' ? 2 : 0 },
                    { hz: '16kHz', val: eqPreset === 'classical' ? 7 : eqPreset === 'rock' ? -1 : 0 }
                  ].map((lever, lKey) => (
                    <div key={lKey} className="h-full flex flex-col items-center justify-between">
                      <span className="text-[7.5px] font-mono text-rose-500 dark:text-rose-450">{lever.val > 0 ? `+${lever.val}` : lever.val}dB</span>
                      <div className="w-1 h-12 bg-zinc-200 dark:bg-zinc-900 rounded relative flex items-end justify-center">
                        <div 
                          style={{ height: `${((lever.val + 10) / 20) * 100}%` }}
                          className="w-full bg-gradient-to-t from-rose-500 dark:from-rose-600 to-pink-400 dark:to-pink-500 rounded"
                        />
                      </div>
                      <span className="text-[8px] font-mono text-zinc-500 dark:text-zinc-650">{lever.hz}</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

        </div>

      </div>

    </div>

    </div>
  );
}
