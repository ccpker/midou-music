/**
 * 本地音乐库状态管理 — Phase 3: 多库 + 回收站 + 歌词管理
 * 对接 Rust library.rs IPC 命令
 */
import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import { open } from '@tauri-apps/plugin-dialog'
import { readDir, copyFile, stat } from '@tauri-apps/plugin-fs'
import { appDataDir } from '@tauri-apps/api/path'
import { invoke } from '@tauri-apps/api/core'

const AUDIO_EXTENSIONS = ['.mp3', '.flac', '.wav', '.ogg', '.m4a']
const MAX_FILE_SIZE = 200 * 1024 * 1024 // 200MB

export const useLibraryStore = defineStore('library', () => {
  // ── State ──
  const importing = ref(false)
  const importProgress = ref('')
  const importError = ref('')

  // Library list and current active
  const libraries = ref([])
  const activeLibrary = ref(null) // { id, name, path, is_master, folders[], trash[] }
  const loading = ref(false)

  // ── Computed ──
  const folders = computed(() => activeLibrary.value?.folders || [])
  const trashItems = computed(() => activeLibrary.value?.trash || [])
  const hasActiveLibrary = computed(() => !!activeLibrary.value)
  const defaultFolderId = computed(() => {
    const def = folders.value.find(f => f.name === '默认')
    return def?.id || ''
  })

  // ── Actions: 库管理 ──
  async function loadLibraries() {
    loading.value = true
    try {
      libraries.value = await invoke('list_libraries')
      // 自动选中激活的库
      const active = libraries.value.find(l => l.active)
      if (active) {
        activeLibrary.value = await invoke('get_active_library')
      }
    } catch (err) {
      console.error('[library] loadLibraries error:', err)
    } finally {
      loading.value = false
    }
  }

  async function createLibrary(name, path) {
    try {
      await invoke('create_library', { name, path })
      await loadLibraries()
    } catch (err) {
      console.error('[library] createLibrary error:', err)
      throw err
    }
  }

  async function switchLibrary(libraryId) {
    try {
      activeLibrary.value = await invoke('switch_library', { libraryId })
      await loadLibraries()
    } catch (err) {
      console.error('[library] switchLibrary error:', err)
      throw err
    }
  }

  async function deleteLibrary(libraryId) {
    try {
      await invoke('delete_library', { libraryId })
      if (activeLibrary.value?.id === libraryId) {
        activeLibrary.value = null
      }
      await loadLibraries()
    } catch (err) {
      console.error('[library] deleteLibrary error:', err)
      throw err
    }
  }

  async function mergeLibrary(fromId, toId) {
    try {
      await invoke('merge_library', { fromId, toId })
      await loadLibraries()
      // Refresh active library if it was the target
      if (activeLibrary.value?.id === toId) {
        activeLibrary.value = await invoke('get_active_library')
      }
    } catch (err) {
      console.error('[library] mergeLibrary error:', err)
      throw err
    }
  }

  async function setMasterLibrary(libraryId) {
    try {
      await invoke('set_master_library', { libraryId })
      await loadLibraries()
    } catch (err) {
      console.error('[library] setMasterLibrary error:', err)
      throw err
    }
  }

  // ── Actions: 文件夹管理 ──
  async function createFolder(name, parentId = null) {
    try {
      await invoke('create_folder', { name, parentId })
      await refreshActiveLibrary()
    } catch (err) {
      console.error('[library] createFolder error:', err)
      throw err
    }
  }

  async function deleteFolder(folderId) {
    try {
      await invoke('delete_folder', { folderId })
      await refreshActiveLibrary()
    } catch (err) {
      console.error('[library] deleteFolder error:', err)
      throw err
    }
  }

  async function addSongToFolder(musicId, folderId) {
    try {
      await invoke('add_song_to_folder', { musicId, folderId })
      await refreshActiveLibrary()
    } catch (err) {
      console.error('[library] addSongToFolder error:', err)
      throw err
    }
  }

  // ── Actions: 回收站 ──
  async function moveToTrash(musicId) {
    try {
      await invoke('move_to_trash', { musicId })
      await refreshActiveLibrary()
    } catch (err) {
      console.error('[library] moveToTrash error:', err)
      throw err
    }
  }

  async function restoreFromTrash(musicId) {
    try {
      await invoke('restore_from_trash', { musicId })
      await refreshActiveLibrary()
    } catch (err) {
      console.error('[library] restoreFromTrash error:', err)
      throw err
    }
  }

  async function emptyTrash() {
    try {
      const count = await invoke('empty_trash')
      console.log(`[library] 清空回收站: ${count} 首歌`)
      await refreshActiveLibrary()
      return count
    } catch (err) {
      console.error('[library] emptyTrash error:', err)
      throw err
    }
  }

  // ── Actions: 歌词 ──
  async function saveLyrics(musicId, lrcContent) {
    try {
      await invoke('save_lyrics', { musicId, lrcContent })
    } catch (err) {
      console.error('[library] saveLyrics error:', err)
      throw err
    }
  }

  async function getLyrics(musicId) {
    try {
      return await invoke('get_lyrics', { musicId })
    } catch (err) {
      console.error('[library] getLyrics error:', err)
      return ''
    }
  }

  // ── Actions: 入库 ──
  async function addSongToLibrary(title, artist, source, sourceId, filePath, fileSize, folderId = null) {
    try {
      const musicId = await invoke('add_song_to_library', {
        title, artist, source, sourceId,
        filePath, fileSize, folderId,
      })
      await refreshActiveLibrary()
      return musicId
    } catch (err) {
      console.error('[library] addSongToLibrary error:', err)
      throw err
    }
  }

  // ── Helpers ──
  async function refreshActiveLibrary() {
    if (!activeLibrary.value?.id) return
    try {
      activeLibrary.value = await invoke('get_active_library')
    } catch (err) {
      console.error('[library] refreshActiveLibrary error:', err)
    }
  }

  // ── 导入本地音乐文件夹 (P2 保留，接入新 IPC) ──
  async function importLocalMusic() {
    importError.value = ''
    importProgress.value = '选择文件夹...'

    const selected = await open({
      multiple: false,
      directory: true,
      title: '选择音乐文件夹',
    })

    if (!selected) {
      importProgress.value = ''
      return
    }

    importing.value = true

    try {
      const libDir = await getLibraryDir()
      importProgress.value = '扫描音频文件...'
      const audioFiles = await scanAudioFiles(selected, libDir)

      if (audioFiles.length === 0) {
        importProgress.value = ''
        importError.value = '未找到支持的音频文件'
        return
      }

      let copied = 0
      let skipped = 0
      let imported = 0

      for (const file of audioFiles) {
        importProgress.value = `导入中... (${copied}/${audioFiles.length})`

        if (file.size > MAX_FILE_SIZE) {
          console.log(`[library] 跳过 ${file.name} (${(file.size / (1024 * 1024)).toFixed(1)}MB > 200MB)`)
          skipped++
          continue
        }

        // 目标路径
        let destName = file.name
        let dest = `${libDir}/${destName}`
        let counter = 1
        while (await fileExists(dest)) {
          const dotIdx = destName.lastIndexOf('.')
          if (dotIdx > 0) {
            destName = `${destName.slice(0, dotIdx)}_${counter}${destName.slice(dotIdx)}`
          } else {
            destName = `${destName}_${counter}`
          }
          dest = `${libDir}/${destName}`
          counter++
        }

        importProgress.value = `复制: ${file.name} → ${destName}`
        await copyFile(file.path, dest)

        // Phase 3: 入库到当前激活库
        if (activeLibrary.value?.id) {
          try {
            const libRelPath = destName // relative to library dir root
            const title = file.name.replace(/\.[^.]+$/, '') // strip extension
            await addSongToLibrary(title, '', 'local', destName, libRelPath, file.size)
            imported++
          } catch (err) {
            console.error(`[library] 入库失败: ${file.name}`, err)
          }
        }

        copied++
      }

      await refreshActiveLibrary()

      importProgress.value = ''
      importError.value = ''
      console.log(`[library] 导入完成: ${copied} 个文件, ${imported} 首入库, 跳过 ${skipped} 个`)
    } catch (err) {
      console.error('[library] 导入失败:', err)
      importError.value = `导入失败: ${err}`
    } finally {
      importing.value = false
    }
  }

  // ── Internal: 获取库目录 ──
  async function getLibraryDir() {
    return activeLibrary.value?.path || (await appDataDir() + 'music-app/library')
  }

  // ── Internal: 检查文件是否存在 ──
  async function fileExists(path) {
    try {
      await stat(path)
      return true
    } catch {
      return false
    }
  }

  // ── U2: 添加本地非主库（不复制文件） ──
  async function scanLocalLibrary(name, path) {
    importError.value = ''
    importProgress.value = '创建库...'

    try {
      // 1. 创建库（若已有主库则为 non-master）
      await createLibrary(name, path)
      await loadLibraries()

      // 2. 切换到新库
      const lib = libraries.value.find(l => l.path === path)
      if (!lib) throw new Error('库创建后未找到')
      await switchLibrary(lib.id)

      // 3. 扫描目录下音频文件
      importProgress.value = '扫描音频文件...'
      const audioFiles = await scanAudioFilesLocal(path, path, 0)

      if (audioFiles.length === 0) {
        importProgress.value = ''
        importError.value = '未找到支持的音频文件（mp3/flac/wav/ogg/m4a）'
        return
      }

      // 4. 逐个入库（不复制文件，file_path 用库目录下的相对路径）
      let added = 0
      const libPathNorm = path.replace(/\\/g, '/')

      for (const file of audioFiles) {
        importProgress.value = `入库中... (${added}/${audioFiles.length})`

        if (file.size > MAX_FILE_SIZE) {
          console.log(`[library] 跳过 ${file.name} (${(file.size / (1024 * 1024)).toFixed(1)}MB > 200MB)`)
          continue
        }

        // 计算相对路径
        const absPath = file.path.replace(/\\/g, '/')
        const relPath = absPath.startsWith(libPathNorm + '/')
          ? absPath.slice(libPathNorm.length + 1)
          : file.name

        const title = file.name.replace(/\.[^.]+$/, '')
        try {
          await addSongToLibrary(title, '', 'local', relPath, relPath, file.size)
          added++
        } catch (err) {
          console.error(`[library] 入库失败: ${file.name}`, err)
        }
      }

      await refreshActiveLibrary()
      importProgress.value = ''
      importError.value = ''
      console.log(`[library] 本地库扫描完成: ${path} — ${added} 首入库`)
    } catch (err) {
      console.error('[library] 扫描本地库失败:', err)
      importError.value = `扫描失败: ${err}`
      importProgress.value = ''
    }
  }

  // ── Internal: 递归扫描音频文件（不依赖 libDir 参数，只做目录遍历） ──
  async function scanAudioFilesLocal(dir, _libDir, depth = 0) {
    const results = []
    const maxDepth = 10

    if (depth > maxDepth) return results

    const name = dir.split(/[/\\]/).pop()?.toLowerCase() || ''
    if (depth > 0 && ['system volume information', '$recycle.bin', '.trash'].includes(name)) {
      return results
    }

    try {
      const entries = await readDir(dir)
      for (const entry of entries) {
        if (entry.name.startsWith('.')) continue
        if (entry.isDirectory) {
          const subResults = await scanAudioFilesLocal(`${dir}/${entry.name}`, _libDir, depth + 1)
          results.push(...subResults)
        } else if (entry.isFile) {
          const ext = entry.name.slice(entry.name.lastIndexOf('.')).toLowerCase()
          if (AUDIO_EXTENSIONS.includes(ext)) {
            results.push({
              path: `${dir}/${entry.name}`,
              name: entry.name,
              size: entry.size || 0,
            })
          }
        }
      }
    } catch (err) {
      console.log(`[library] 跳过目录 ${dir}: ${err}`)
    }

    return results
  }

  // ── U1: 检查歌曲是否已在当前激活库中 ──
  function isSongCollected(source, sourceId) {
    if (!activeLibrary.value?.folders) return false
    for (const folder of activeLibrary.value.folders) {
      for (const song of (folder.songs || [])) {
        if (song.source === source && song.source_id === sourceId) {
          return true
        }
      }
    }
    return false
  }

  // ── Internal: 递归扫描音频文件 ──
  async function scanAudioFiles(dir, libDir, depth = 0) {
    const results = []
    const maxDepth = 10

    if (depth > maxDepth) return results

    const name = dir.split(/[/\\]/).pop()?.toLowerCase() || ''
    if (depth > 0 && ['system volume information', '$recycle.bin', '.trash'].includes(name)) {
      return results
    }

    try {
      const entries = await readDir(dir)
      for (const entry of entries) {
        if (entry.name.startsWith('.')) continue
        if (entry.isDirectory) {
          const subResults = await scanAudioFiles(`${dir}/${entry.name}`, libDir, depth + 1)
          results.push(...subResults)
        } else if (entry.isFile) {
          const ext = entry.name.slice(entry.name.lastIndexOf('.')).toLowerCase()
          if (AUDIO_EXTENSIONS.includes(ext)) {
            results.push({
              path: `${dir}/${entry.name}`,
              name: entry.name,
              size: entry.size || 0,
            })
          }
        }
      }
    } catch (err) {
      console.log(`[library] 跳过目录 ${dir}: ${err}`)
    }

    return results
  }

  return {
    // state
    importing,
    importProgress,
    importError,
    libraries,
    activeLibrary,
    loading,
    // computed
    folders,
    trashItems,
    hasActiveLibrary,
    defaultFolderId,
    // actions - 库
    loadLibraries,
    createLibrary,
    switchLibrary,
    deleteLibrary,
    mergeLibrary,
    setMasterLibrary,
    // actions - 文件夹
    createFolder,
    deleteFolder,
    addSongToFolder,
    // actions - 回收站
    moveToTrash,
    restoreFromTrash,
    emptyTrash,
    // actions - 歌词
    saveLyrics,
    getLyrics,
    // actions - 入库
    addSongToLibrary,
    importLocalMusic,
    scanLocalLibrary,
    isSongCollected,
    refreshActiveLibrary,
  }
})
