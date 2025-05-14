use std::io::Write;
use encoding_rs::{UTF_16LE, UTF_8};
use eframe::egui;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use image::io::Reader as ImageReader;
use tempfile::NamedTempFile;
use anyhow::Context;
use egui::{TextEdit, ProgressBar, Color32, text::LayoutJob, FontId, ColorImage, TextureHandle};
use chrono::Local;

const MAIN_WINDOW_CAPTION: &str = r#"АО ПК "Азимут" клиент автоматизации SAP"#;
const DEFAULT_SCRIPT: &str = r#"
On Error Resume Next

' Проверка установки SAP GUI
Set SapGuiAuto = GetObject("SAPGUI")
If Err.Number <> 0 Then
    WScript.Echo "Error 01: SAP GUI not installed!"
    WScript.Quit 1001
End If

' Проверка активной сессии
Set application = SapGuiAuto.GetScriptingEngine
If Err.Number <> 0 Then
    WScript.Echo "Error 02: SAP scripting engine not available!"
    WScript.Quit 1002
End If

' Пример простой операции
WScript.Echo "Success: SAP GUI version " & application.Version
WScript.Quit 0
"#;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        MAIN_WINDOW_CAPTION,
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
        
        // Загрузка изображения
        let icon_image = ImageReader::new(std::io::Cursor::new(icon_bytes))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        
        // Конвертация в формат egui
        let size = [icon_image.width() as usize, icon_image.height() as usize];
        let icon_rgba = icon_image.to_rgba8();
        let color_image = ColorImage::from_rgba_unmultiplied(
            size,
            &icon_rgba
        );
        
        // Создание текстуры
        let icon_texture = cc.egui_ctx.load_texture(
            "app-icon", 
            color_image, 
            Default::default()
        );

        Self {
            script_content: DEFAULT_SCRIPT.to_string(),
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
        self.add_log("ℹ️ Создание временного файла...".into(), Color32::GRAY);
        
        let _temp_file = match NamedTempFile::new() {
            Ok(f) => f,
            Err(e) => {
                self.add_log(format!("❌ Ошибка создания файла: {}", e), Color32::RED);
                return;
            }
        };
        
        self.add_log("ℹ️ Запись скрипта...".into(), Color32::GRAY);
        self.is_running = true;
        self.progress = 0.0;
        self.add_log("🚀 Запуск скрипта...".to_string(), Color32::LIGHT_BLUE);
        
        let (sender, receiver) = bounded(1);
        self.receiver = Some(receiver);
        
        let script = self.script_content.clone();
        
        std::thread::spawn(move || {
            let result = execute_script(&script);
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
                                    (Color32::RED, format!("❌ {}", line))
                                } else if line.contains("Warning") {
                                    (Color32::YELLOW, format!("⚠️ {}", line))
                                } else {
                                    (Color32::GREEN, format!("✅ {}", line))
                                };
                                self.add_log(text, color);
                            }
                        }
                        Err(e) => self.add_log(format!("❌ {}", e), Color32::RED),
                    }
                    self.receiver = None;
                }
                Err(TryRecvError::Empty) => {
                    self.progress = (self.progress + 0.005) % 1.0;
                }
                Err(TryRecvError::Disconnected) => {
                    self.add_log("⚠️ Соединение с потоком потеряно".into(), Color32::YELLOW);
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
                    ui.heading(MAIN_WINDOW_CAPTION);
                    ui.label("v0.1");
                });
                
                ui.separator();
                ui.label("Тема:");
                egui::ComboBox::from_id_source("theme_selector")
                    .selected_text(self.themes[self.selected_theme])
                    .show_ui(ui, |ui| {
                        for (i, theme) in self.themes.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_theme, i, *theme);
                        }
                    });
                ui.separator();
                if ui.button("📋 Копировать логи").clicked() {
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
                    ui.heading(MAIN_WINDOW_CAPTION);
                    // ui.label("v0.1");
                	ui.label("Редактор скрипта:");
                	egui::ScrollArea::vertical()
                    	.max_height(600.0)
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
                        "⏹ Остановить (Esc)" 
                    } else { 
                        "▶ Запуск (F5)" 
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
                            self.add_log("⏹ Выполнение прервано пользователем".into(), Color32::LIGHT_YELLOW);
                        }
                    }

                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.is_running {
                        self.is_running = false;
                        self.add_log("⏹ Выполнение прервано (Escape)".into(), Color32::LIGHT_YELLOW);
                    }
                });

                ui.separator();
                
                ui.label("Execution Log:");
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

fn execute_script(script: &str) -> anyhow::Result<String> {
    let mut temp_file = NamedTempFile::new()
        .context("Failed to create temporary file")?;
    
    temp_file.write_all(b"\xEF\xBB\xBF")?;
    temp_file.write_all(script.as_bytes())
        .context("Failed to write script")?;
    
    let temp_path = temp_file.into_temp_path();
    
    let output = std::process::Command::new("cscript.exe")
        .args(&["//Nologo", &temp_path.to_string_lossy()])
        .output()
        .context("Failed to execute cscript.exe")?;
    
    let decode_with_bom = |bytes: &[u8]| {
        if bytes.starts_with(b"\xFF\xFE") {
            UTF_16LE.decode(&bytes[2..]).0.into_owned()
        } else if bytes.starts_with(b"\xEF\xBB\xBF") {
            UTF_8.decode(&bytes[3..]).0.into_owned()
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        }
    };
    
    let stdout = decode_with_bom(&output.stdout);
    let stderr = decode_with_bom(&output.stderr);
    
    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&format!("Stdout:\n{}\n", stdout));
    }
    if !stderr.is_empty() {
        result.push_str(&format!("Stderr:\n{}\n", stderr));
    }
    
    if output.status.success() {
        Ok(result)
    } else {
        anyhow::bail!("Exit code: {}\n{}", output.status, result)
    }
}