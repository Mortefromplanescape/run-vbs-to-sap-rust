use std::io::Write;
use eframe::egui;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use tempfile::NamedTempFile;
use anyhow::Context;
use egui::{TextEdit, ProgressBar};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "SAP GUI Automation Pro",
        options,
        Box::new(|_cc| Box::new(MyApp::new())),
    )
}

struct MyApp {
    script_content: String,
    logs: Vec<String>,
    receiver: Option<Receiver<anyhow::Result<()>>>,
    progress: f32,
    is_running: bool,
}

impl MyApp {
    fn new() -> Self {
        Self {
            script_content: String::from(r#"
                ' Пример скрипта по умолчанию
                If Not IsObject(application) Then
                    Set SapGuiAuto = GetObject("SAPGUI")
                    Set application = SapGuiAuto.GetScriptingEngine
                End If
                WScript.Echo "Скрипт выполнен!"
            "#),
            logs: Vec::new(),
            receiver: None,
            progress: 0.0,
            is_running: false,
        }
    }

    fn add_log(&mut self, log: String) {
        self.logs.push(log);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    self.is_running = false;
                    self.progress = 0.0;
                    match result {
                        Ok(_) => self.add_log("✅ Скрипт успешно выполнен".into()),
                        Err(e) => self.add_log(format!("❌ Ошибка: {}", e)),
                    }
                    self.receiver = None;
                }
                Err(TryRecvError::Empty) => {
                    self.progress = (self.progress + 0.01) % 1.0;
                }
                Err(TryRecvError::Disconnected) => {
                    self.add_log("⚠️ Соединение с потоком потеряно".into());
                    self.is_running = false;
                    self.receiver = None;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SAP Automation Tool v2.0");
            
            ui.label("Редактор скрипта:");
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    TextEdit::multiline(&mut self.script_content)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .show(ui);
                });

            ui.separator();
            
            ui.horizontal(|ui| {
                let button_text = if self.is_running { "⏹ Остановить" } else { "▶ Запуск" };
                
                if self.is_running {
                    ui.add(
                        ProgressBar::new(self.progress)
                            .animate(true)
                    );
                }

                if ui.button(button_text).clicked() {
                    if !self.is_running {
                        self.start_script();
                    } else {
                        self.is_running = false;
                        self.add_log("⏹ Выполнение прервано пользователем".into());
                    }
                }
            });

            ui.separator();
            
            ui.label("Журнал выполнения:");
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &self.logs {
                        ui.monospace(log);
                    }
                });

            ctx.request_repaint();
        });
    }
}

impl MyApp {
    fn start_script(&mut self) {
        self.is_running = true;
        self.progress = 0.0;
        self.add_log("🚀 Запуск скрипта...".into());
        
        let (sender, receiver) = bounded(1);
        self.receiver = Some(receiver);
        
        let script = self.script_content.clone();
        
        std::thread::spawn(move || {
            let result = execute_script(&script);
            let _ = sender.send(result);
        });
    }
}

fn execute_script(script: &str) -> anyhow::Result<()> {
    let mut temp_file = NamedTempFile::new()
        .context("Не удалось создать временный файл")?;
    
    temp_file.write_all(script.as_bytes())
        .context("Ошибка записи скрипта")?;
    
    let output = std::process::Command::new("cscript.exe")
        .args(&["//Nologo", &temp_file.path().to_string_lossy()])
        .output()
        .context("Ошибка запуска cscript.exe")?;
    
    if output.status.success() {
        Ok(())
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Код ошибки {}: {}", output.status, error_msg)
    }
}
