<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { LyricPlayer } from "@applemusic-like-lyrics/core";
  import { parseTTML, type LyricLine as RawLyricLine } from "@applemusic-like-lyrics/lyric";
  import type { LyricLine } from "@applemusic-like-lyrics/core";
  import '@applemusic-like-lyrics/core/style.css';
  // DOM 与 实例引用
  let container: HTMLElement;
  let audioEl: HTMLAudioElement;
  let player: LyricPlayer;

  // 状态变量
  let audioUrl = "";
  let lyricLines: LyricLine[] = [];
  let animationFrameId: number;

  // 1. TTML 数据清洗函数 (与原代码一致)
  const mapTTMLLyric = (line: RawLyricLine): LyricLine => ({
    ...line,
    words: line.words.map((word) => ({ obscene: false, ...word })),
  });

  // 2. 初始化核心播放器组件
  onMount(() => {
    player = new LyricPlayer();
    
    // 继承外部样式，占满全屏
    const el = player.getElement();
    el.style.width = "100%";
    el.style.height = "100%";
    container.appendChild(el);
  });

  // 组件销毁时清理内存
  onDestroy(() => {
    if (animationFrameId) cancelAnimationFrame(animationFrameId);
    if (audioUrl) URL.revokeObjectURL(audioUrl);
  });

  // 3. 响应式更新：当 lyricLines 变化且 player 就绪时，设置歌词
  $: if (player && lyricLines.length > 0) {
    player.setLyricLines(lyricLines);
  }

  // 4. 音乐加载与播放同步逻辑
  function startSync() {
    let lastTime = -1;
    const onFrame = (time: number) => {
      // 只有在音频存在且未暂停时才更新动画
      if (audioEl && !audioEl.paused) {
        if (lastTime === -1) lastTime = time;
        const dt = time - lastTime;
        
        player.update(dt); // 驱动内部动画 (滚动、模糊)
        player.setCurrentTime((audioEl.currentTime * 1000) | 0); // 同步真实时间 (毫秒)
        
        lastTime = time;
        animationFrameId = requestAnimationFrame(onFrame);
      }
    };
    animationFrameId = requestAnimationFrame(onFrame);
  }

  // 5. 事件处理函数
  const loadAudio = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "audio/*";
    input.onchange = () => {
      const file = input.files?.[0];
      if (file) {
        if (audioUrl) URL.revokeObjectURL(audioUrl); // 清理旧资源
        audioUrl = URL.createObjectURL(file);
      }
    };
    input.click();
  };

  const loadLyrics = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".ttml,text/*";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (file) {
        const text = await file.text();
        lyricLines = parseTTML(text).lines.map(mapTTMLLyric);
      }
    };
    input.click();
  };
</script>

<main>
  <!-- 歌词挂载容器 -->
  <div class="lyric-container" bind:this={container}></div>

  <!-- 控制面板 -->
  <div class="control-panel">
    <div class="title">Svelte 最小同步示例</div>
    <button on:click={loadAudio}>加载音乐</button>
    <button on:click={loadLyrics}>加载歌词 (TTML)</button>
    
    {#if audioUrl}
      <!-- svelte-ignore a11y-media-has-caption -->
      <audio 
        bind:this={audioEl} 
        controls 
        src={audioUrl} 
        preload="auto"
        on:play={startSync}
        on:pause={() => cancelAnimationFrame(animationFrameId)}
      ></audio>
    {/if}
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background-color: #111; /* 黑色背景，凸显毛玻璃文字 */
  }

  main {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
  }

  .lyric-container {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    contain: paint layout; /* 性能优化 */
  }

  .control-panel {
    position: absolute;
    right: 1rem;
    bottom: 1rem;
    background-color: rgba(0, 0, 0, 0.6);
    margin: 1rem;
    padding: 1rem;
    border-radius: 0.5rem;
    color: white;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    z-index: 10;
  }

  .title {
    font-size: 0.9rem;
    color: #ccc;
    margin-bottom: 0.5rem;
  }

  button {
    padding: 8px 12px;
    background: #333;
    color: #fff;
    border: 1px solid #555;
    border-radius: 4px;
    cursor: pointer;
  }

  button:hover {
    background: #444;
  }
</style>