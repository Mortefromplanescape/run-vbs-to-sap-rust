use std::io::Write;
use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_16LE};
use eframe::egui;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use image::io::Reader as ImageReader;
use tempfile::NamedTempFile;
use anyhow::Context;
use egui::{TextEdit, ProgressBar, Color32, text::LayoutJob, FontId, ColorImage, TextureHandle};
use chrono::Local;

const APP_VERSION: &str = r#"v0.10"#;
// const PATH_LOGO: &str = r#"../assets/logo.png"#;
const LABEL_MAIN_WINDOW: &str = r#"АО ПК "Азимут" клиент автоматизации SAP"#;
const LABEL_THEME: &str = r#"Тема"#;
const LABEL_LOG: &str = r#"Журнал(лог) выполнения скрипта"#;
const LABEL_EDITOR: &str = r#"Редактор скрипта"#;
const BUTTON_RUN_SCRIPT: &str = r#"Запустить скрипт на выполнение (F5)"#;
const BUTTON_STOP_SCRIPT: &str = r#"Остановить выполнение скрипта (Esc)"#;
const BUTTON_COPY_LOGS: &str = r#"Копировать логи"#;
const BUTTON_CLEAR_LOGS: &str = r#"Очистить логи"#;
const ICON_OK: &str = r#"✅"#;
const ICON_ERR: &str = r#"❌"#;
const ICON_WARN: &str = r#"⚠️"#;
const ICON_COPY: &str = r#"📋"#;
const ICON_INFO: &str = r#"ℹ️"#;
const ICON_RUN: &str = r#"🚀"#;
const ICON_PLAY: &str = r#"▶"#;
const ICON_STOP: &str = r#"⏹"#;
const MSG_I_FILE_PATH: &str = r#"Путь к файлу"#;
const MSG_I_FILE_SIZE: &str = r#"Размер скрипта в байтах"#;
const MSG_I_FILE_CREATE: &str = r#"Создание временного файла..."#;
const MSG_W_SCRIPT_MANUAL_STOPPED: &str = r#"Выполнение скрипта прервано пользователем"#;
const MSG_W_THREAD_LOST: &str = r#"Соединение с потоком потеряно"#;
const MSG_E_FILE_CREATE: &str = r#"Ошибка создания файла"#;
const MSG_E_FILE_WRITE: &str = r#"Ошибка записи"#;
const SCRIPT_DEFAULT: &str = r#"
On Error Resume Next

' Проверка установки SAP GUI
Set SapGuiAuto = GetObject("SAPGUI")
If Err.Number <> 0 Then
    WScript.Echo "Error 01: SAP GUI не установлен!"
    WScript.Quit 1001
End If

' Проверка активной сессии
Set application = SapGuiAuto.GetScriptingEngine
If Err.Number <> 0 Then
    WScript.Echo "Error 02: Scripting API недоступно!"
    WScript.Quit 1002
End If

' Пример операции
WScript.Echo "Успех: Версия SAP GUI " & application.Version
WScript.Quit 0
"#;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        LABEL_MAIN_WINDOW,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(MyApp::new(cc))
        }),
    )
}

struct MyApp {
    script_content: String,
    logs: Vec<LogEntry>,
    receiver: Option<Receiver<anyhow::Result<String>>>,
    progress: f32,
    is_running: bool,
    selected_theme: usize,
    themes: Vec<&'static str>,
    scroll_to_bottom: bool,
    icon_texture: Option<TextureHandle>,
}

#[derive(Clone)]
struct LogEntry {
    text: String,
    color: Color32,
    timestamp: String,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let icon_bytes = include_bytes!("../assets/logo.png");
        
        let icon_image = ImageReader::new(std::io::Cursor::new(icon_bytes))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        
        let size = [icon_image.width() as usize, icon_image.height() as usize];
        let icon_rgba = icon_image.to_rgba8();
        let color_image = ColorImage::from_rgba_unmultiplied(size, &icon_rgba);
        
        let icon_texture = cc.egui_ctx.load_texture("app-icon", color_image, Default::default());

        Self {
            script_content: SCRIPT_DEFAULT.to_string(),
            logs: Vec::new(),
            receiver: None,
            progress: 0.0,
            is_running: false,
            selected_theme: 0,
            themes: vec!["Тёмная", "Светлая", "Синяя"],
            scroll_to_bottom: false,
            icon_texture: Some(icon_texture),
        }
    }

    fn add_log(&mut self, text: String, color: Color32) {
        let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
        self.logs.push(LogEntry { text, color, timestamp });
        self.scroll_to_bottom = true;
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let visuals = match self.selected_theme {
            0 => egui::Visuals::dark(),
            1 => egui::Visuals::light(),
            2 => egui::Visuals {
                dark_mode: true,
                override_text_color: Some(Color32::from_rgb(100, 200, 255)),
                ..egui::Visuals::dark()
            },
            _ => egui::Visuals::dark(),
        };
        ctx.set_visuals(visuals);
    }

    fn start_script(&mut self) {
        self.add_log(MSG_I_FILE_CREATE.into(), Color32::GRAY);
        
        let temp_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(e) => {
                self.add_log(format!("{} {}: {}", ICON_ERR, MSG_E_FILE_CREATE, e), Color32::RED);
                return;
            }
        };
        
        let temp_path = temp_file.into_temp_path();
        self.add_log(format!("{} {}: {}", ICON_INFO, MSG_I_FILE_PATH, temp_path.display()), Color32::GRAY);

        let mut file = match std::fs::File::create(&temp_path) {
            Ok(f) => f,
            Err(e) => {
                self.add_log(format!("{} {}: {}", ICON_ERR, MSG_E_FILE_WRITE, e), Color32::RED);
                return;
            }
        };

        // Подготовка контента с правильными переводами строк и кодировкой
        let script = self.script_content.replace('\n', "\r\n");
        let script_bytes = script.encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect::<Vec<u8>>();
        
        // Сохраняем скрипт в UTF-16LE с BOM
        let mut content = vec![0xFF, 0xFE]; // BOM для UTF-16LE
        let script = self.script_content.replace('\n', "\r\n");
        let script_utf16: Vec<u16> = script.encode_utf16().collect();
        
        for c in script_utf16 {
            content.extend(c.to_le_bytes());
        }

        if let Err(e) = file.write_all(&content) {
            self.add_log(format!("{} {}: {}", ICON_ERR, MSG_E_FILE_WRITE, e), Color32::RED);
            return;
        }

        self.add_log(format!("{} {}: {}", ICON_INFO, MSG_I_FILE_SIZE, content.len()), Color32::GRAY);
        self.is_running = true;
        self.progress = 0.0;
        self.add_log("🚀 Запуск скрипта...".to_string(), Color32::LIGHT_BLUE);
        
        let (sender, receiver) = bounded(1);
        self.receiver = Some(receiver);
        
        let script_path = temp_path.to_owned();
        std::thread::spawn(move || {
            let result = execute_script(&script_path);
            let _ = sender.send(result);
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);

        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    self.is_running = false;
                    self.progress = 0.0;
                    match result {
                        Ok(output) => {
                            for line in output.lines() {
                                let (color, text) = if line.contains("Error") {
                                    (Color32::RED, format!("{} {}", ICON_ERR, line))
                                } else if line.contains("Warning") {
                                    (Color32::YELLOW, format!("{} {}", ICON_WARN, line))
                                } else {
                                    (Color32::GREEN, format!("{} {}", ICON_OK, line))
                                };
                                self.add_log(text, color);
                            }
                        }
                        Err(e) => self.add_log(format!("{} {}", ICON_ERR, e), Color32::RED),
                    }
                    self.receiver = None;
                }
                Err(TryRecvError::Empty) => {
                    self.progress = (self.progress + 0.005) % 1.0;
                }
                Err(TryRecvError::Disconnected) => {
                    self.add_log(format!("{} {}", ICON_WARN, MSG_W_THREAD_LOST), Color32::YELLOW);
                    self.is_running = false;
                    self.receiver = None;
                }
            }
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(icon) = &self.icon_texture {
                    ui.image(icon, [256.0, 64.0]);
                }
                ui.separator();
                ui.vertical(|ui| {
                    ui.heading(LABEL_MAIN_WINDOW);
                    ui.label(APP_VERSION);
                });
                
                ui.separator();
                ui.label(LABEL_THEME);
                egui::ComboBox::from_id_source("theme_selector")
                    .selected_text(self.themes[self.selected_theme])
                    .show_ui(ui, |ui| {
                        for (i, theme) in self.themes.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_theme, i, *theme);
                        }
                    });
                ui.separator();
                if ui.button(format!("{} {}", ICON_COPY, BUTTON_COPY_LOGS)).clicked() {
                    let logs: String = self.logs
                        .iter()
                        .map(|e| format!("{} {}", e.timestamp, e.text))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ui.output_mut(|o| o.copied_text = logs);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // ui.horizontal(|ui| {
                    ui.heading(LABEL_MAIN_WINDOW);
                    // ui.label("v0.1");
                    ui.label(LABEL_EDITOR);
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                        TextEdit::multiline(&mut self.script_content)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .show(ui);
                        });
                //     ui.separator();
                // });

                ui.separator();
                
                ui.horizontal(|ui| {
                    let button_text = if self.is_running { 
                        format!("{} {}", ICON_STOP, BUTTON_STOP_SCRIPT)
                    } else { 
                        format!("{} {}", ICON_PLAY, BUTTON_RUN_SCRIPT)
                    };
                    
                    if self.is_running {
                        ui.add(
                            ProgressBar::new(self.progress)
                                .animate(true)
                                .desired_width(200.0)
                        );
                    }

                    if ui.button(button_text).clicked() 
                        || (ui.input(|i| i.key_pressed(egui::Key::F5)) && !self.is_running)
                    {
                        if !self.is_running {
                            self.start_script();
                        } else {
                            self.is_running = false;
                            self.add_log(format!("{} {}", ICON_WARN, MSG_W_SCRIPT_MANUAL_STOPPED), Color32::LIGHT_YELLOW);
                        }
                    }

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.is_running {
                        self.is_running = false;
                        self.add_log(format!("{} {}", ICON_WARN, MSG_W_SCRIPT_MANUAL_STOPPED), Color32::LIGHT_YELLOW);
                    }
                });

                ui.separator();
                
                ui.label(LABEL_LOG);
                egui::ScrollArea::vertical()
                    .id_source("log_scroll")
                    .max_height(300.0)
                    .stick_to_bottom(self.scroll_to_bottom)
                    .show(ui, |ui| {
                        for entry in &self.logs {
                            let mut job = LayoutJob::default();
                            job.append(
                                &format!("[{}] ", entry.timestamp),
                                0.0,
                                egui::TextFormat::simple(
                                    FontId::monospace(12.0),
                                    Color32::GRAY,
                                ),
                            );
                            job.append(
                                &entry.text,
                                0.0,
                                egui::TextFormat::simple(
                                    FontId::monospace(12.0),
                                    entry.color,
                                ),
                            );

                            ui.add(egui::Label::new(job).sense(egui::Sense::click()));
                        }
                        self.scroll_to_bottom = false;
                    });
            });

            ctx.request_repaint();
        });
    }
}

fn execute_script(temp_path: &std::path::Path) -> anyhow::Result<String> {    
    let output = std::process::Command::new("cscript.exe")
        .args(&["//Nologo", &temp_path.to_string_lossy()])
        .output()
        .context("Ошибка выполнения скрипта")?;

    // Универсальный декодер с автоопределением кодировки
    let decode_output = |bytes: &[u8]| -> String {
        // Проверка BOM для UTF-16LE
        if bytes.starts_with(&[0xFF, 0xFE]) {
            return UTF_16LE.decode(&bytes[2..]).0.into_owned();
        }

        // Автоопределение кодировки
        let mut detector = EncodingDetector::new();
        detector.feed(bytes, true);
        let encoding = detector.guess(None, true);

        // Преобразование в соответствующую кодировку
        let (decoded, _, _) = encoding.decode(bytes);
        decoded.into_owned()
    };

    let stdout = decode_output(&output.stdout);
    let stderr = decode_output(&output.stderr);

    // Очистка вывода от артефактов
    let clean = |s: String| s.replace('\0', "")
        .trim()
        .replace("\r\n", "\n")
        .to_string();

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&format!("[Вывод]\n{}\n", clean(stdout)));
    }
    if !stderr.is_empty() {
        result.push_str(&format!("[Ошибки]\n{}\n", clean(stderr)));
    }
    
    if output.status.success() {
        Ok(result.trim().to_string())
    } else {
        anyhow::bail!("Код ошибки: {}\n{}", output.status, result.trim())
    }
}