use std::io::Write;
use eframe::egui;
use crossbeam_channel::{bounded, Receiver, TryRecvError};
use tempfile::NamedTempFile;
use anyhow::Context;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(400.0, 200.0)),
        vsync: false,
        ..Default::default()
    };

    eframe::run_native(
        "SAP GUI Automation",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

#[derive(Default)]
struct MyApp {
    status: String,
    receiver: Option<Receiver<anyhow::Result<String>>>,
}

impl MyApp {
    // пример скрипта для проверки
    const VBS_SCRIPT: &'static str = r#"
        If Not IsObject(application) Then
		   Set SapGuiAuto  = GetObject("SAPGUI")
		   Set application = SapGuiAuto.GetScriptingEngine
		End If
		If Not IsObject(connection) Then
		   Set connection = application.Children(0)
		End If
		If Not IsObject(session) Then
		   Set session    = connection.Children(0)
		End If
		If IsObject(WScript) Then
		   WScript.ConnectObject session,     "on"
		   WScript.ConnectObject application, "on"
		End If
		session.findById("wnd[0]").maximize
		session.findById("wnd[0]/tbar[0]/okcd").text = "/nie01"
		session.findById("wnd[0]").sendVKey 0
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").text = "10144765"
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").caretPosition = 8
		session.findById("wnd[0]").sendVKey 0
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").text = "10044765"
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").caretPosition = 2
		session.findById("wnd[0]").sendVKey 0
		session.findById("wnd[0]/usr/ctxtRM63E-EQTYP").text = ""
		session.findById("wnd[0]/usr/ctxtRM63E-EQTYP").setFocus
		session.findById("wnd[0]/usr/ctxtRM63E-EQTYP").caretPosition = 0
		session.findById("wnd[0]").sendVKey 4
		session.findById("wnd[1]/usr/lbl[1,6]").setFocus
		session.findById("wnd[1]/usr/lbl[1,6]").caretPosition = 0
		session.findById("wnd[1]").sendVKey 2
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").text = "10144765"
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").setFocus
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").caretPosition = 3
		session.findById("wnd[0]").sendVKey 0
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").text = ""
		session.findById("wnd[0]/usr/ctxtRM63E-EQUNR").caretPosition = 0
		session.findById("wnd[0]").sendVKey 0
		session.findById("wnd[0]/tbar[0]/btn[15]").press
		session.findById("wnd[1]/usr/btnSPOP-OPTION2").press
		session.findById("wnd[0]/tbar[0]/btn[3]").press
    "#;
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Проверка статуса выполнения
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    self.status = match result {
                        Ok(msg) => format!("✅ {}", msg),
                        Err(e) => format!("❌ Ошибка: {}", e),
                    };
                    self.receiver = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.status = "⚠️ Поток выполнения прерван".to_string();
                    self.receiver = None;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SAP Automation Tool");
            ui.separator();
            
            // Кнопка запуска
            let button = ui.button("🚀 Запустить скрипт");
            
            if button.clicked() && self.receiver.is_none() {
                self.start_script();
            }
            
            // Статус
            ui.label(&self.status);
            
            // Форсируем постоянную перерисовку
            ctx.request_repaint();
        });
    }
}

impl MyApp {
    fn start_script(&mut self) {
        let (sender, receiver) = bounded(1);
        self.receiver = Some(receiver);
        
        let script = Self::VBS_SCRIPT.to_string();
        
        std::thread::spawn(move || {
            let result = execute_script(&script)
                .map(|_| "Скрипт выполнен успешно".to_string());
            
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
