use eframe::egui;
use crossbeam_channel::{Receiver, TryRecvError};
use super::{constants::*, models::LogEntry, execution};
use egui::{Color32, ColorImage, TextureHandle, TextEdit, ProgressBar};

pub struct MyApp {
    pub script_content: String,
    pub logs: Vec<LogEntry>,
    pub receiver: Option<Receiver<anyhow::Result<String>>>,
    pub progress: f32,
    pub is_running: bool,
    pub selected_theme: usize,
    pub scroll_to_bottom: bool,
    pub icon_texture: Option<TextureHandle>,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let icon_bytes = include_bytes!("../../assets/logo.png");
        let icon_image = image::io::Reader::new(std::io::Cursor::new(icon_bytes))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
        
        let size = [icon_image.width() as usize, icon_image.height() as usize];
        let color_image = ColorImage::from_rgba_unmultiplied(
            size,
            &icon_image.to_rgba8()
        );
        
        let icon_texture = cc.egui_ctx.load_texture(UI_TECH_APP_ICON, color_image, Default::default());

        Self {
            script_content: SCRIPT_DEFAULT.to_string(),
            logs: Vec::new(),
            receiver: None,
            progress: 0.0,
            is_running: false,
            selected_theme: DEFAULT_THEME_INDEX,
            scroll_to_bottom: false,
            icon_texture: Some(icon_texture),
        }
    }

    pub fn add_log(&mut self, text: String, color: Color32) {
        use chrono::Local;
        
        let timestamp = Local::now().format(APP_LOG_FORMAT_STRING).to_string();
        self.logs.push(LogEntry { text, color, timestamp });
        self.scroll_to_bottom = true;
        if self.logs.len() > LOG_CAPACITY {
            self.logs.remove(0);
        }
    }

    pub fn apply_theme(&self, ctx: &egui::Context) {
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

    pub fn start_script(&mut self) {
        use tempfile::Builder;
        
        self.add_log(format!("{} {}", ICON_INFO, MSG_I_FILE_CREATING), Color32::GRAY);
        
        let temp_file = match Builder::new()
            .suffix(".vbs")
            .tempfile() 
        {
            Ok(f) => f,
            Err(e) => {
                self.add_log(format!("{} {}: {}", ICON_ERR, MSG_E_FILE_CREATE, e), Color32::RED);
                return;
            }
        };
        
        let temp_path = temp_file.into_temp_path();
        self.add_log(
            format!("{} {}: {}", ICON_INFO, MSG_I_FILE_PATH, temp_path.display()),
            Color32::GRAY
        );

        if let Err(e) = self.write_script_to_file(&temp_path) {
            self.add_log(format!("{} {}: {}", ICON_ERR, MSG_E_FILE_WRITE, e), Color32::RED);
            return;
        }

        self.prepare_execution(temp_path);
    }

    fn write_script_to_file(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        let script = self.script_content.replace('\n', "\r\n");
        let script_utf16: Vec<u16> = script.encode_utf16().collect();
        
        let mut content = vec![0xFF, 0xFE];
        for c in script_utf16 {
            content.extend(c.to_le_bytes());
        }
        
        file.write_all(&content)?;
        Ok(())
    }

    fn prepare_execution(&mut self, temp_path: tempfile::TempPath) {
        use crossbeam_channel::bounded;
        
        self.add_log(
            format!("{} {}: {}", ICON_INFO, MSG_I_FILE_SIZE, 
                temp_path.as_os_str().len()),
            Color32::GRAY
        );
        
        self.is_running = true;
        self.progress = 0.0;
        self.add_log(format!("{} {}", ICON_RUN, MSG_I_SCRIPT_RUNNING), Color32::LIGHT_BLUE);
        
        let (sender, receiver) = bounded(1);
        self.receiver = Some(receiver);
        
        std::thread::spawn(move || {
            let result = execution::execute_script(&temp_path);
            let _ = sender.send(result);
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.handle_thread_messages();
        self.draw_ui(ctx);
        ctx.request_repaint();
    }
}

impl MyApp {
    fn handle_thread_messages(&mut self) {
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => self.handle_execution_result(result),
                Err(TryRecvError::Empty) => self.update_progress(),
                Err(TryRecvError::Disconnected) => self.handle_thread_disconnect(),
            }
        }
    }

    fn handle_execution_result(&mut self, result: anyhow::Result<String>) {
        self.is_running = false;
        self.progress = 0.0;
        
        match result {
            Ok(output) => self.process_success_output(output),
            Err(e) => self.add_log(format!("{} {}", ICON_ERR, e), Color32::RED),
        }
        
        self.receiver = None;
    }

    fn process_success_output(&mut self, output: String) {
        for line in output.lines() {
            let (color, text) = if line.contains(UI_TECH_ERROR) {
                (Color32::RED, format!("{} {}", ICON_ERR, line))
            } else if line.contains(UI_TECH_WARNING) {
                (Color32::YELLOW, format!("{} {}", ICON_WARN, line))
            } else {
                (Color32::GREEN, format!("{} {}", ICON_OK, line))
            };
            self.add_log(text, color);
        }
    }

    fn update_progress(&mut self) {
        self.progress = (self.progress + PROGRESS_STEP) % 1.0;
    }

    fn handle_thread_disconnect(&mut self) {
        self.add_log(
            format!("{} {}", ICON_WARN, MSG_W_THREAD_LOST),
            Color32::YELLOW
        );
        self.is_running = false;
        self.receiver = None;
    }

    fn draw_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top(UI_TECH_HEADER).show(ctx, |ui| {
            self.draw_header(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_main_content(ui);
        });
    }

    fn draw_header(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.separator();
            ui.horizontal(|ui| {
                self.draw_icon(ui);
                ui.separator();
                self.draw_title(ui);
                ui.separator();
                self.draw_theme_selector(ui);
                ui.separator();
                ui.vertical(|ui| {
                    self.draw_copy_button(ui);
                    self.draw_clear_button(ui);
                });
            });
        });
    }

    fn draw_icon(&self, ui: &mut egui::Ui) {
        if let Some(icon) = &self.icon_texture {
            ui.image(icon, [256.0, 64.0]);
        }
    }

    fn draw_title(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading(LABEL_MAIN_WINDOW);
            ui.label(APP_VERSION);
        });
    }

    fn draw_theme_selector(&mut self, ui: &mut egui::Ui) {
        ui.label(LABEL_THEME);
        egui::ComboBox::from_id_source(UI_TECH_THEME_SELECTOR)
            .selected_text(THEMES[self.selected_theme])
            .show_ui(ui, |ui| {
                for (i, theme) in THEMES.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_theme, i, *theme);
                }
            });
    }

    fn draw_copy_button(&mut self, ui: &mut egui::Ui) {
        if ui.button(format!("{} {}", ICON_COPY, BUTTON_COPY_LOGS)).clicked() {
            let logs: String = self.logs
                .iter()
                .map(|e| format!("{} {}", e.timestamp, e.text))
                .collect::<Vec<_>>()
                .join("\n");
            ui.output_mut(|o| o.copied_text = logs);
        }
    }

    fn draw_clear_button(&mut self, ui: &mut egui::Ui) {
        if ui.button(format!("{} {}", ICON_CLEAR, BUTTON_CLEAR_LOGS)).clicked() {
            let logs: String = self.logs
                .iter()
                .map(|e| format!("{} {}", e.timestamp, e.text))
                .collect::<Vec<_>>()
                .join("\n");
            ui.output_mut(|o| o.copied_text = logs);
        }
    }

    fn draw_main_content(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            self.draw_editor(ui);
            self.draw_controls(ui);
            self.draw_logs(ui);
        });
    }

    fn draw_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(LABEL_EDITOR);
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                TextEdit::multiline(&mut self.script_content)
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .show(ui);
            });
        ui.separator();
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.draw_progress_bar(ui);
            self.draw_run_button(ui);
            self.handle_keyboard_shortcuts(ui.ctx());
        });
    }

    fn draw_progress_bar(&self, ui: &mut egui::Ui) {
        if self.is_running {
            ui.add(
                ProgressBar::new(self.progress)
                    .animate(true)
                    .desired_width(200.0)
            );
        }
    }

    fn draw_run_button(&mut self, ui: &mut egui::Ui) {
        let button_text = if self.is_running { 
            format!("{} {}", ICON_STOP, BUTTON_STOP_SCRIPT)
        } else { 
            format!("{} {}", ICON_PLAY, BUTTON_RUN_SCRIPT)
        };

        if ui.button(button_text).clicked() {
            self.toggle_script_execution();
        }
    }

    fn toggle_script_execution(&mut self) {
        if !self.is_running {
            self.start_script();
        } else {
            self.is_running = false;
            self.add_log(
                format!("{} {}", ICON_WARN, MSG_W_SCRIPT_MANUAL_STOPPED),
                Color32::LIGHT_YELLOW
            );
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) && !self.is_running {
            self.start_script();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.is_running {
            self.toggle_script_execution();
        }
    }

    fn draw_logs(&mut self, ui: &mut egui::Ui) {
        ui.label(LABEL_LOG);
        egui::ScrollArea::vertical()
            .id_source(UI_TECH_LOG_SCROLL)
            .max_height(300.0)
            .stick_to_bottom(self.scroll_to_bottom)
            .show(ui, |ui| {
                for entry in &self.logs {
                    ui.add(egui::Label::new(entry.to_layout_job()).sense(egui::Sense::click()));
                }
                self.scroll_to_bottom = false;
            });
    }
}