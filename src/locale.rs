#[derive(Clone, Copy, PartialEq)]
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

    pub fn mode_name(&self, mode: &crate::player::PlayMode) -> &str {
        use crate::player::PlayMode;
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
}

pub fn detect_locale() -> Locale {
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_default()
        .to_lowercase();

    if locale.starts_with("zh") || locale.contains("zh_cn") || locale.contains("zh-cn") {
        Locale::zh_cn()
    } else {
        Locale::en()
    }
}
