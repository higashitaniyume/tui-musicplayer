#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lang {
    En,
    ZhCn,
}

#[derive(Clone)]
pub struct Locale {
    pub lang: Lang,
}

impl Locale {
    pub fn lang(&self) -> Lang { self.lang }

    pub fn from_lang(lang: Lang) -> Self {
        Self { lang }
    }

    pub fn en() -> Self {
        Self { lang: Lang::En }
    }

    pub fn zh_cn() -> Self {
        Self { lang: Lang::ZhCn }
    }

    pub fn playlist_title(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!(" Playlist ({count} tracks) "),
            Lang::ZhCn => format!(" 播放列表 ({count} 首) "),
        }
    }

    pub fn library_title(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!(" Library ({count} tracks) "),
            Lang::ZhCn => format!(" 音乐库 ({count} 首) "),
        }
    }

    pub fn no_tracks_hint(&self) -> &str {
        match self.lang {
            Lang::En => " No tracks loaded — run: tui-musicplayer <directory>",
            Lang::ZhCn => " 无曲目 — 运行: tui-musicplayer <目录>",
        }
    }

    pub fn mode_label(&self) -> &str {
        match self.lang {
            Lang::En => "Mode: ",
            Lang::ZhCn => "模式: ",
        }
    }

    pub fn playing(&self) -> &str {
        match self.lang {
            Lang::En => "Playing",
            Lang::ZhCn => "播放中",
        }
    }

    pub fn paused(&self) -> &str {
        match self.lang {
            Lang::En => "Paused",
            Lang::ZhCn => "已暂停",
        }
    }

    pub fn stopped(&self) -> &str {
        match self.lang {
            Lang::En => "Stopped",
            Lang::ZhCn => "已停止",
        }
    }

    pub fn mode_name(&self, mode: &crate::audio::PlayMode) -> &str {
        use crate::audio::PlayMode;
        match self.lang {
            Lang::En => match mode {
                PlayMode::Sequential => "Seq",
                PlayMode::Shuffle => "Shuffle",
                PlayMode::RepeatOne => "Repeat 1",
                PlayMode::RepeatAll => "Repeat All",
            },
            Lang::ZhCn => match mode {
                PlayMode::Sequential => "顺序",
                PlayMode::Shuffle => "随机",
                PlayMode::RepeatOne => "单曲循环",
                PlayMode::RepeatAll => "列表循环",
            },
        }
    }

    // Key hints
    pub fn hint_pause(&self) -> &str { match self.lang { Lang::En => "Pause ", Lang::ZhCn => "暂停 " } }
    pub fn hint_next(&self) -> &str { match self.lang { Lang::En => "Next ", Lang::ZhCn => "下一首 " } }
    pub fn hint_prev(&self) -> &str { match self.lang { Lang::En => "Prev ", Lang::ZhCn => "上一首 " } }
    pub fn hint_stop(&self) -> &str { match self.lang { Lang::En => "Stop ", Lang::ZhCn => "停止 " } }
    pub fn hint_mode(&self) -> &str { match self.lang { Lang::En => "Mode ", Lang::ZhCn => "模式 " } }
    pub fn hint_quit(&self) -> &str { match self.lang { Lang::En => "Quit", Lang::ZhCn => "退出" } }

    pub fn error_prefix(&self) -> &str {
        match self.lang {
            Lang::En => "Error",
            Lang::ZhCn => "错误",
        }
    }

    pub fn seek_error(&self) -> &str {
        match self.lang {
            Lang::En => "Seek error",
            Lang::ZhCn => "跳转错误",
        }
    }

    pub fn info_no_track(&self) -> &str {
        match self.lang { Lang::En => "No track loaded", Lang::ZhCn => "未加载曲目" }
    }
    pub fn playlist_empty_hint(&self) -> &str {
        match self.lang { Lang::En => "Press A in Library to add tracks", Lang::ZhCn => "在音乐库中按 A 添加曲目" }
    }

    // ── Lyrics ──
    pub fn lyrics_title(&self) -> &str {
        match self.lang { Lang::En => " Lyrics ", Lang::ZhCn => " 歌词 " }
    }
    pub fn no_lyrics(&self) -> &str {
        match self.lang { Lang::En => "(no .lrc file)", Lang::ZhCn => "(无歌词文件)" }
    }

    // ── Settings ──
    pub fn settings_title(&self) -> &str {
        match self.lang { Lang::En => " Settings ", Lang::ZhCn => " 设置 " }
    }
    pub fn music_folders_title(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!(" Music Folders ({count}) "),
            Lang::ZhCn => format!(" 音乐文件夹 ({count}) "),
        }
    }
    pub fn xspf_playlists_title(&self, count: usize) -> String {
        match self.lang {
            Lang::En => format!(" XSPF Playlists ({count}) "),
            Lang::ZhCn => format!(" XSPF 播放列表 ({count}) "),
        }
    }
    pub fn settings_no_folders(&self) -> &str {
        match self.lang {
            Lang::En => "No folders added — press A to add",
            Lang::ZhCn => "未添加文件夹 — 按 A 添加",
        }
    }
    pub fn settings_no_xspf(&self) -> &str {
        match self.lang {
            Lang::En => "No playlists — press I to import .xspf",
            Lang::ZhCn => "无播放列表 — 按 I 导入 .xspf",
        }
    }
    pub fn settings_hint_add_folder(&self) -> &str {
        match self.lang { Lang::En => "Add Folder", Lang::ZhCn => "添加文件夹" }
    }
    pub fn settings_hint_import_xspf(&self) -> &str {
        match self.lang { Lang::En => "Import XSPF", Lang::ZhCn => "导入 XSPF" }
    }
    pub fn settings_hint_remove(&self) -> &str {
        match self.lang { Lang::En => "Remove", Lang::ZhCn => "删除" }
    }
    pub fn settings_hint_rescan(&self) -> &str {
        match self.lang { Lang::En => "Rescan", Lang::ZhCn => "重新扫描" }
    }
    pub fn settings_hint_back(&self) -> &str {
        match self.lang { Lang::En => "Back", Lang::ZhCn => "返回" }
    }
    pub fn input_prompt_folder(&self) -> &str {
        match self.lang { Lang::En => "Enter music folder path", Lang::ZhCn => "输入音乐文件夹路径" }
    }
    pub fn input_prompt_xspf(&self) -> &str {
        match self.lang { Lang::En => "Enter .xspf file path", Lang::ZhCn => "输入 .xspf 文件路径" }
    }
    pub fn input_confirm_hint(&self) -> &str {
        match self.lang { Lang::En => "Enter: confirm  Esc: cancel", Lang::ZhCn => "Enter: 确认  Esc: 取消" }
    }
    pub fn msg_folder_added(&self) -> &str {
        match self.lang { Lang::En => "Folder added and scanned", Lang::ZhCn => "文件夹已添加并扫描" }
    }
    pub fn msg_xspf_imported(&self) -> &str {
        match self.lang { Lang::En => "XSPF playlist imported", Lang::ZhCn => "XSPF 播放列表已导入" }
    }
    pub fn msg_folder_removed(&self) -> &str {
        match self.lang { Lang::En => "Folder removed from config", Lang::ZhCn => "文件夹已从配置中移除" }
    }
    pub fn msg_xspf_removed(&self) -> &str {
        match self.lang { Lang::En => "Playlist removed from config", Lang::ZhCn => "播放列表已从配置中移除" }
    }
    pub fn msg_rescanned(&self, tracks: usize) -> String {
        match self.lang {
            Lang::En => format!("Rescanned: {tracks} tracks loaded"),
            Lang::ZhCn => format!("已重新扫描: 加载了 {tracks} 首曲目"),
        }
    }
    pub fn msg_path_not_found(&self) -> &str {
        match self.lang { Lang::En => "Path not found", Lang::ZhCn => "路径不存在" }
    }
    pub fn msg_not_xspf(&self) -> &str {
        match self.lang { Lang::En => "Not an .xspf file", Lang::ZhCn => "不是 .xspf 文件" }
    }
    pub fn settings_key_hint(&self) -> &str {
        match self.lang { Lang::En => "Settings", Lang::ZhCn => "设置" }
    }
    pub fn language_label(&self) -> &str {
        match self.lang { Lang::En => "Language", Lang::ZhCn => "语言" }
    }
    pub fn language_value(&self) -> &str {
        match self.lang { Lang::En => "English", Lang::ZhCn => "中文" }
    }
    pub fn language_toggle_hint(&self) -> &str {
        match self.lang { Lang::En => "Toggle Lang", Lang::ZhCn => "切换语言" }
    }
    pub fn audio_host_label(&self) -> &str {
        match self.lang { Lang::En => "Audio Output", Lang::ZhCn => "音频输出" }
    }
    pub fn audio_host_default(&self) -> &str {
        match self.lang { Lang::En => "System Default", Lang::ZhCn => "系统默认" }
    }
    pub fn audio_host_changed(&self, host: &str) -> String {
        match self.lang {
            Lang::En => format!("Audio output: {host}"),
            Lang::ZhCn => format!("音频输出: {host}"),
        }
    }
    pub fn audio_host_hint(&self) -> &str {
        match self.lang { Lang::En => "Audio Host", Lang::ZhCn => "音频主机" }
    }

    // ── Named playlists ──
    pub fn playlist_switched(&self, name: &str) -> String {
        match self.lang {
            Lang::En => format!("Switched to: {name}"),
            Lang::ZhCn => format!("切换到: {name}"),
        }
    }
    pub fn playlist_switched_all(&self) -> &str {
        match self.lang {
            Lang::En => "Switched to: All Tracks",
            Lang::ZhCn => "切换到: 全部曲目",
        }
    }
    pub fn playlist_switch_hint(&self) -> &str {
        match self.lang { Lang::En => "Switch PL", Lang::ZhCn => "切换列表" }
    }
    pub fn playlist_title_named(&self, name: &str, count: usize) -> String {
        match self.lang {
            Lang::En => format!(" {name} ({count} tracks) "),
            Lang::ZhCn => format!(" {name} ({count} 首) "),
        }
    }
}

pub fn detect_locale() -> Locale {
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_default()
        .to_lowercase();

    if locale.starts_with("zh") || locale.contains("zh_cn") || locale.contains("zh-cn") {
        Locale::zh_cn()
    } else if locale.starts_with("en") || locale.contains("en_us") || locale.contains("en-us") {
        Locale::en()
    } else {
        Locale::zh_cn() // default to Chinese
    }
}
