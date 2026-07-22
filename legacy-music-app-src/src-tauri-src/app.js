const $ = id => document.getElementById(id);
const API_BASE = (() => {
  const p = new URLSearchParams(location.search).get('port');
  return p ? `http://127.0.0.1:${p}` : 'http://127.0.0.1:8899';
})();

let currentSource = 'kuwo';
let currentSong = null;
let audio = new Audio();

// 源配置
const SOURCE_CONFIG = {
  kuwo: { name: '酷我音乐', desc: 'VIP歌曲免费播' },
  qq: { name: 'QQ音乐', desc: '正版曲库' },
  bilibili: { name: 'B站音乐', desc: '视频转音频' },
  kugou: { name: '酷狗音乐', desc: '概念版源' }
};

// 初始化
document.addEventListener('DOMContentLoaded', () => {
  initSourceButtons();
  initSearch();
  initPlayer();
});

// 源按钮切换
function initSourceButtons() {
  document.querySelectorAll('.source-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.source-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      currentSource = btn.dataset.source;
      updateSourceDisplay();
    });
  });
}

function updateSourceDisplay() {
  const cfg = SOURCE_CONFIG[currentSource];
  $('current-source').textContent = cfg.name;
  $('source-desc').textContent = cfg.desc;
}

// 搜索
function initSearch() {
  $('search-btn').addEventListener('click', doSearch);
  $('search-input').addEventListener('keypress', e => {
    if (e.key === 'Enter') doSearch();
  });
}

async function doSearch() {
  const kw = $('search-input').value.trim();
  if (!kw) return;

  const resultsDiv = $('results');
  resultsDiv.innerHTML = '<div style="color:var(--muted);padding:40px;text-align:center">搜索中...</div>';

  // source -> mode 映射
  const modeMap = {
    kuwo: 'normal',      // 酷我原生
    qq: 'qq',
    bilibili: 'bilibili',
    kugou: 'normal'      // 酷狗暂用酷我兜底
  };
  const mode = modeMap[currentSource] || 'normal';

  try {
    const res = await fetch(`${API_BASE}/api/search?keyword=${encodeURIComponent(kw)}&mode=${mode}`);
    const data = await res.json();
    renderResults(data.songs || []);
  } catch (err) {
    resultsDiv.innerHTML = `<div style="color:var(--accent);padding:40px;text-align:center">搜索失败: ${err.message}</div>`;
  }
}

function renderResults(songs) {
  const resultsDiv = $('results');
  if (!songs.length) {
    resultsDiv.innerHTML = '<div style="color:var(--muted);padding:40px;text-align:center">无结果</div>';
    return;
  }

  resultsDiv.innerHTML = songs.map(s => `
    <div class="song-item" data-song='${JSON.stringify(s).replace(/'/g, "&#39;")}'>
      <div class="song-info">
        <div class="song-title">${escapeHtml(s.name)}</div>
        <div class="song-meta">${escapeHtml(s.artist)} · ${escapeHtml(s.album || '未知专辑')}</div>
      </div>
      <div class="song-actions">
        <button onclick="playSong(this)">播放</button>
        <button onclick="addToQueue(this)">+队列</button>
      </div>
    </div>
  `).join('');

  // 点击整行播放
  document.querySelectorAll('.song-item').forEach(item => {
    item.addEventListener('click', e => {
      if (e.target.tagName === 'BUTTON') return;
      playSong(item.querySelector('button'));
    });
  });
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// 播放
function playSong(btn) {
  const item = btn.closest('.song-item');
  const song = JSON.parse(item.dataset.song);
  currentSong = song;

  $('player-title').textContent = song.name;
  $('player-artist').textContent = song.artist;
  $('btn-play').textContent = '⏸';

  // 获取播放URL - 根据当前源调用不同路由
  const playUrlMap = {
    kuwo: `${API_BASE}/api/play/kuwo/${encodeURIComponent(song.song_id)}?name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`,
    qq: `${API_BASE}/api/play/qq/${encodeURIComponent(song.song_id)}?name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`,
    bilibili: `${API_BASE}/api/play/bilibili?song_id=${encodeURIComponent(song.song_id)}&name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`,
    kugou: `${API_BASE}/api/play/kuwo/${encodeURIComponent(song.song_id)}?name=${encodeURIComponent(song.name)}&singer=${encodeURIComponent(song.artist)}`
  };
  const playUrl = playUrlMap[currentSource] || playUrlMap.kuwo;

  fetch(playUrl)
    .then(r => r.json())
    .then(data => {
      if (data.url) {
        audio.src = data.url;
        audio.play();
      } else {
        alert('获取播放链接失败: ' + (data.error || '未知错误'));
      }
    })
    .catch(err => {
      alert('播放请求失败: ' + err.message);
    });
}

function addToQueue(btn) {
  const item = btn.closest('.song-item');
  const song = JSON.parse(item.dataset.song);
  // TODO: 实现队列
  console.log('添加到队列:', song);
}

// 播放器控制
function initPlayer() {
  $('btn-play').addEventListener('click', () => {
    if (!audio.src) return;
    if (audio.paused) {
      audio.play();
      $('btn-play').textContent = '⏸';
    } else {
      audio.pause();
      $('btn-play').textContent = '▶';
    }
  });

  audio.addEventListener('ended', () => {
    $('btn-play').textContent = '▶';
  });

  $('btn-lyrics').addEventListener('click', () => {
    if (!currentSong) return;
    window.open(`${API_BASE}/lyrics-pop.html?song_id=${currentSong.song_id}&source=${currentSource}`,
      'lyrics', 'width=400,height=600');
  });
}
