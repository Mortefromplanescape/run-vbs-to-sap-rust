pub const APP_VERSION: &str = r#"v0.10"#;
pub const LABEL_MAIN_WINDOW: &str = r#"АО ПК "Азимут" клиент автоматизации SAP"#;
pub const LABEL_THEME: &str = r#"Тема"#;
pub const LABEL_LOG: &str = r#"Журнал(лог) выполнения скрипта"#;
pub const LABEL_EDITOR: &str = r#"Редактор скрипта"#;
pub const BUTTON_RUN_SCRIPT: &str = r#"Запустить скрипт на выполнение (F5)"#;
pub const BUTTON_STOP_SCRIPT: &str = r#"Остановить выполнение скрипта (Esc)"#;
pub const BUTTON_COPY_LOGS: &str = r#"Копировать логи"#;
pub const BUTTON_CLEAR_LOGS: &str = r#"Очистить логи"#;

pub const ICON_OK: &str = r#"✅"#;
pub const ICON_ERR: &str = r#"❌"#;
pub const ICON_WARN: &str = r#"⚠️"#;
pub const ICON_COPY: &str = r#"📋"#;
pub const ICON_INFO: &str = r#"ℹ️"#;
pub const ICON_RUN: &str = r#"🚀"#;
pub const ICON_PLAY: &str = r#"▶"#;
pub const ICON_STOP: &str = r#"⏹"#;

pub const MSG_I_FILE_PATH: &str = r#"Путь к файлу"#;
pub const MSG_I_FILE_SIZE: &str = r#"Размер скрипта в байтах"#;
pub const MSG_I_FILE_CREATING: &str = r#"Создание временного файла..."#;
pub const MSG_I_SCRIPT_RUNNING: &str = r#"Запуск скрипта..."#;
pub const MSG_W_SCRIPT_MANUAL_STOPPED: &str = r#"Выполнение скрипта прервано пользователем"#;
pub const MSG_W_THREAD_LOST: &str = r#"Соединение с потоком потеряно"#;
pub const MSG_E_FILE_CREATE: &str = r#"Ошибка создания файла"#;
pub const MSG_E_FILE_WRITE: &str = r#"Ошибка записи"#;

pub const SCRIPT_DEFAULT: &str = r#"
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

pub const THEMES: [&str; 3] = ["Тёмная", "Светлая", "Синяя"];
pub const DEFAULT_THEME_INDEX: usize = 0;
pub const LOG_CAPACITY: usize = 1000;
pub const PROGRESS_STEP: f32 = 0.005;