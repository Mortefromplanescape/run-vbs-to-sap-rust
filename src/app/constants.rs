pub const APP_VERSION: &str = r#"v0.10"#;
pub const APP_LOG_FORMAT_STRING: &str = r#"%H:%M:%S%.3f"#;

pub const LABEL_MAIN_WINDOW: &str = r#"АО ПК "Азимут" клиент автоматизации SAP"#;
pub const LABEL_EDITOR: &str = r#"Область редактирования скрипта:"#;
pub const LABEL_LOG: &str = r#"Журнал выполнения скрипта:"#;
pub const LABEL_THEME: &str = r#"Тема:"#;

pub const BUTTON_STOP_SCRIPT: &str = r#"Остановить выполнение скрипта (Esc)"#;
pub const BUTTON_RUN_SCRIPT: &str = r#"Запустить скрипт на выполнение (F5)"#;
pub const BUTTON_COPY_LOGS: &str = r#"Копировать журнал"#;
pub const BUTTON_CLEAR_LOGS: &str = r#"Очистить журнал"#;

pub const ICON_OK: &str = r#"🗸"#; //✅
pub const ICON_ERR: &str = r#"🗴"#; //❌
pub const ICON_WARN: &str = r#"⚠️"#;
pub const ICON_COPY: &str = r#"🗐"#; //📋
pub const ICON_CLEAR: &str = r#"♺"#; //🧹♲
pub const ICON_INFO: &str = r#"ℹ️"#;
pub const ICON_RUN: &str = r#"🚀"#;
pub const ICON_PLAY: &str = r#"▶"#;
pub const ICON_STOP: &str = r#"⏹"#;

pub const MSG_I_FILE_PATH: &str = r#"Путь к файлу"#;
pub const MSG_I_FILE_SIZE: &str = r#"Размер файла скрипта в байтах"#;
pub const MSG_I_FILE_CREATING: &str = r#"Создание временного файла..."#;
pub const MSG_I_SCRIPT_RUNNING: &str = r#"Запуск скрипта..."#;
pub const MSG_W_SCRIPT_MANUAL_STOPPED: &str = r#"Выполнение скрипта прервано пользователем"#;
pub const MSG_W_THREAD_LOST: &str = r#"Соединение с потоком потеряно"#;
pub const MSG_E_FILE_CREATE: &str = r#"Ошибка создания файла скрипта"#;
pub const MSG_E_FILE_WRITE: &str = r#"Ошибка записи файла скрипта"#;

pub const UI_TECH_THEME_SELECTOR: &str = r#"theme_selector"#;
pub const UI_TECH_LOG_SCROLL: &str = r#"log_scroll"#;
pub const UI_TECH_APP_ICON: &str = r#"app-icon"#;
pub const UI_TECH_WARNING: &str = r#"Warning"#;
pub const UI_TECH_HEADER: &str = r#"header"#;
pub const UI_TECH_ERROR: &str = r#"Error"#;

pub const THEMES: [&str; 3] = ["Тёмная", "Светлая", "Тёмно-синяя"];
pub const DEFAULT_THEME_INDEX: usize = 2;
pub const LOG_CAPACITY: usize = 1000;
pub const PROGRESS_STEP: f32 = 0.005;

pub const SCRIPT_DEFAULT: &str = r#"
On Error Resume Next

' Проверка установки SAP GUI
	Set SapGuiAuto = GetObject("SAPGUI")
	If Err.Number <> 0 Then
	    WScript.Echo "Error 01: SAP GUI не запущен или не установлен!"
	    WScript.Quit 1001
	End If

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
"#;


// ' Проверка активной сессии
// Set application = SapGuiAuto.GetScriptingEngine
// If Err.Number <> 0 Then
//     WScript.Echo "Error 02: Scripting API недоступно!"
//     WScript.Quit 1002
// End If

// ' Пример операции
// WScript.Echo "Успех: Версия SAP GUI " & application.Version
// WScript.Quit 0
// "#;