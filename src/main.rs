use std::io::Write;
use eframe::egui;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use tempfile::NamedTempFile;
use anyhow::Context;
use egui::{TextEdit, ProgressBar, Color32, text::LayoutJob, FontId};
use chrono::Local;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "SAP Automation Pro",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(MyApp::new())
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
}

#[derive(Clone)]
struct LogEntry {
    text: String,
    color: Color32,
    timestamp: String,
}

impl MyApp {
    fn new() -> Self {
        Self {
            script_content: DEFAULT_SCRIPT.to_string(),
            logs: Vec::new(),
            receiver: None,
            progress: 0.0,
            is_running: false,
            selected_theme: 0,
            themes: vec!["Тёмная", "Светлая", "Синяя"],
            scroll_to_bottom: false,
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
                ui.heading("SAP Automation Tool v4.0");
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
                ui.label("VBScript Editor:");
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        TextEdit::multiline(&mut self.script_content)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .show(ui);
                    });

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
    
    temp_file.write_all(script.as_bytes())
        .context("Failed to write script")?;
    
    let output = std::process::Command::new("cscript.exe")
        .args(&["//Nologo", &temp_file.path().to_string_lossy()])
        .output()
        .context("Failed to execute cscript.exe")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
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

const DEFAULT_SCRIPT: &str = r#"
On Error Resume Next

Set SapGuiAuto = GetObject("SAPGUI")
If Err.Number <> 0 Then
    WScript.Echo "Error: SAP GUI not found! Please start SAP Logon first."
    WScript.Quit 1
End If

Set application = SapGuiAuto.GetScriptingEngine
Set connection = application.Children(0)
Set session = connection.Children(0)

session.findById("wnd[0]").maximize
WScript.Echo "Info: SAP window maximized"

session.findById("wnd[0]/tbar[0]/okcd").text = "/n"
session.findById("wnd[0]").sendVKey 0
WScript.Echo "Success: Reset SAP session"

WScript.Echo "Script completed successfully"
"#;
